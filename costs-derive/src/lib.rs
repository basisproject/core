use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    DeriveInput,
    Ident,
    parse_macro_input,
};

#[derive(Debug)]
struct Field {
    name: Ident,
    hash_key: Ident,
    hash_val: Ident,
}

/// Derive our costs impl.
///
/// Effectively, we collect any HashMap fields in the struct (ignoring others)
/// and implement things like new_with_<field> or get_<field> as well as Add/Div
/// and our other math stuff.
#[proc_macro_derive(Costs)]
pub fn derive_costs(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // grab our HashMap fields from the input
    let fields: Vec<Field> = match &input.data {
        syn::Data::Struct(syn::DataStruct { fields: syn::Fields::Named(syn::FieldsNamed { named: fields, .. }), .. }) => {
            fields.iter()
                .map(|field| {
                    (
                        field.ident.as_ref().unwrap().clone(),
                        match &field.ty {
                            syn::Type::Path(syn::TypePath { path: syn::Path { segments, .. }, .. }) => {
                                Some(segments[0].clone())
                            }
                            _ => None,
                        }
                    )
                })
                .filter(|fieldspec| {
                    match &fieldspec.1 {
                        Some(path) => {
                            path.ident == syn::Ident::new("HashMap", proc_macro2::Span::call_site())
                        }
                        None => false,
                    }
                })
                .map(|(fieldname, segment)| {
                    let segment = segment.unwrap();
                    let args = match segment.arguments {
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }) => {
                            args.iter()
                                .map(|arg| {
                                    match arg {
                                        syn::GenericArgument::Type(syn::Type::Path(syn::TypePath { path: syn::Path { segments, .. }, .. })) => {
                                            segments[0].ident.clone()
                                        }
                                        _ => panic!("costs-derive::derive_costs() -- error parsing HashMap args"),
                                    }
                                })
                                .collect::<Vec<_>>()
                        }
                        _ => panic!("costs-derive::derive_costs() -- error parsing HashMap fields"),
                    };
                    Field {
                        name: fieldname,
                        hash_key: args[0].clone(),
                        hash_val: args[1].clone(),
                    }
                })
                .collect::<Vec<_>>()
        }
        _ => panic!("costs-derive::derive_costs() -- can only derive costs on a struct"),
    };

    let fn_new_with = fields.iter().map(|f| format_ident!("new_with_{}", f.name)).collect::<Vec<_>>();
    let fn_new_with_comment = fields.iter().map(|f| format!("Create a new Cost, with one {} entry", f.name)).collect::<Vec<_>>();
    let fn_track = fields.iter().map(|f| format_ident!("track_{}", f.name)).collect::<Vec<_>>();
    let fn_track_comment = fields.iter().map(|f| format!("Add a {} cost to this Cost", f.name)).collect::<Vec<_>>();
    let fn_track_panic = fields.iter().map(|f| format!("Costs::track_{}() -- given value must be >= 0", f.name)).collect::<Vec<_>>();
    let fn_get = fields.iter().map(|f| format_ident!("get_{}", f.name)).collect::<Vec<_>>();
    let fn_get_comment = fields.iter().map(|f| format!("Get a {} value out of this cost object, defaulting to zero if not found", f.name)).collect::<Vec<_>>();
    let field_name = fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
    let field_name_mut = fields.iter().map(|f| format_ident!("{}_mut", f.name)).collect::<Vec<_>>();
    let field_hashkey = fields.iter().map(|f| f.hash_key.clone()).collect::<Vec<_>>();
    let field_hashval = fields.iter().map(|f| f.hash_val.clone()).collect::<Vec<_>>();
    let fn_div_panic = fields.iter().map(|f| format!("Costs::div() -- divide by zero for {} {{:?}}", f.name)).collect::<Vec<_>>();

    let cost_impl = quote! {
        impl #name {
            #(
                #[doc = #fn_new_with_comment]
                pub fn #fn_new_with<T, V>(id: T, #field_name: V) -> Self
                    where T: Into<#field_hashkey>,
                          V: Into<#field_hashval> + Copy,
                {
                    let mut costs = Self::new();
                    costs.#fn_track(id, #field_name);
                    costs
                }
            )*
            #(
                #[doc = #fn_track_comment]
                pub fn #fn_track<T, V>(&mut self, id: T, val: V)
                    where T: Into<#field_hashkey>,
                          V: Into<#field_hashval> + Copy,
                {
                    if val.into() < #field_hashval::zero() {
                        panic!(#fn_track_panic);
                    }
                    let entry = self.#field_name_mut().entry(id.into()).or_insert(rust_decimal::prelude::Zero::zero());
                    *entry += val.into();
                    self.dezero();
                }
            )*
            #(
                #[doc = #fn_get_comment]
                pub fn #fn_get<T: Into<#field_hashkey>>(&self, id: T) -> #field_hashval {
                    *self.#field_name().get(&id.into()).unwrap_or(&#field_hashval::zero())
                }
            )*

            /// Test if we have an empty cost set
            pub fn is_zero(&self) -> bool {
                #(
                    for (_, val) in self.#field_name().iter() {
                        if val > &#field_hashval::zero() {
                            return false;
                        }
                    }
                )*
                true
            }

            /// Remove all zero values from our ranks.
            fn dezero(&mut self) {
                #(
                    let mut remove = vec![];
                    for (key, val) in self.#field_name().iter() {
                        if val == &#field_hashval::zero() {
                            remove.push(key.clone());
                        }
                    }
                    for key in remove {
                        self.#field_name_mut().remove(&key);
                    }
                )*
            }

            /// Given a set of costs, subtract them from our current costs, but only if
            /// the result is >= 0 for each cost tracked. Then, return a costs object
            /// showing exactly how much was taken.
            pub fn take(&mut self, costs: &Costs) -> Costs {
                let mut new_costs = Costs::new();
                #(
                    for (k, lval) in self.#field_name_mut().iter_mut() {
                        let mut rval = costs.#fn_get(k.clone());
                        let val = if lval > &mut rval { rval } else { lval.clone() };
                        *lval -= val;
                        new_costs.#fn_track(k.clone(), val.clone());
                    }
                )*
                new_costs.dezero();
                self.dezero();
                new_costs
            }

            /// Determine if subtracting one set of costs from another results
            /// in any negative values
            pub fn is_sub_lt_0(costs1: &Costs, costs2: &Costs) -> bool {
                let costs3 = costs1.clone() - costs2.clone();
                #(
                    for (_, v) in costs3.#field_name().iter() {
                        if *v < #field_hashval::zero() {
                            return true;
                        }
                    }
                )*
                false
            }

            /// Determine if a set of costs is greater than 0.
            pub fn is_gt_0(&self) -> bool {
                let mut count = 0;
                #(
                    for (_, v) in self.#field_name().iter() {
                        if *v > #field_hashval::zero() {
                            // count how many positive values we have
                            count += 1;
                        } else if *v <= #field_hashval::zero() {
                            // return on ANY negative or 0 vals
                            return false;
                        }
                    }
                )*
                // if we have fields and they're all > 0 then this will be true
                count > 0
            }

            /// Determine if any of our costs are below 0
            pub fn is_lt_0(&self) -> bool {
                #(
                    for (_, v) in self.#field_name().iter() {
                        if *v < #field_hashval::zero() {
                            return true;
                        }
                    }
                )*
                false
            }

            /// Determine if dividing one set of costs by another will result in
            /// a divide-by-zero panic.
            pub fn is_div_by_0(costs1: &Costs, costs2: &Costs) -> bool {
                #(
                    for (k, v) in costs1.#field_name().iter() {
                        let div = costs2.#fn_get(k.clone());
                        if v == &#field_hashval::zero() {
                            continue;
                        }
                        if div == #field_hashval::zero() {
                            return true;
                        }
                    }
                )*
                false
            }
        }

        impl Add for Costs {
            type Output = Self;

            fn add(mut self, other: Self) -> Self {
                #(
                    for k in other.#field_name().keys() {
                        let entry = self.#field_name_mut().entry(k.clone()).or_insert(#field_hashval::zero());
                        *entry += other.#field_name().get(k).unwrap();
                    }
                )*
                self.dezero();
                self
            }
        }

        impl Sub for Costs {
            type Output = Self;

            fn sub(mut self, other: Self) -> Self {
                #(
                    for k in other.#field_name().keys() {
                        let entry = self.#field_name_mut().entry(k.clone()).or_insert(#field_hashval::zero());
                        *entry -= other.#field_name().get(k).unwrap();
                    }
                )*
                self.dezero();
                self
            }
        }

        impl Mul for Costs {
            type Output = Self;

            fn mul(mut self, rhs: Self) -> Self {
                #(
                    for (k, val) in self.#field_name_mut().iter_mut() {
                        *val *= rhs.#field_name().get(k).unwrap_or(&#field_hashval::zero());
                    }
                )*
                self.dezero();
                self
            }
        }

        impl Mul<rust_decimal::Decimal> for Costs {
            type Output = Self;

            fn mul(mut self, rhs: rust_decimal::Decimal) -> Self {
                #(
                    for (_, val) in self.#field_name_mut().iter_mut() {
                        *val *= rhs;
                    }
                )*
                self.dezero();
                self
            }
        }

        impl Div for Costs {
            type Output = Self;

            fn div(mut self, rhs: Self) -> Self::Output {
                #(
                    for (k, v) in self.#field_name_mut().iter_mut() {
                        let div = rhs.#field_name().get(k).map(|x| x.clone()).unwrap_or(#field_hashval::zero());
                        if v == &#field_hashval::zero() {
                            continue;
                        }
                        if div == #field_hashval::zero() {
                            panic!(#fn_div_panic, k);
                        }
                        *v /= div;
                    }
                    for (k, _) in rhs.#field_name().iter() {
                        match self.#field_name().get(k) {
                            None => {
                                self.#field_name_mut().insert(k.clone(), #field_hashval::zero());
                            }
                            _ => {}
                        }
                    }
                )*
                self.dezero();
                self
            }
        }

        impl Div<Decimal> for Costs {
            type Output = Self;

            fn div(mut self, rhs: Decimal) -> Self::Output {
                if rhs == Decimal::zero() {
                    panic!("Costs::div() -- divide by zero");
                }
                #(
                    for (_, v) in self.#field_name_mut().iter_mut() {
                        *v /= rhs;
                    }
                )*
                self.dezero();
                self
            }
        }
    };
    TokenStream::from(cost_impl)
}

