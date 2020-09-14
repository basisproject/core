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

    let fn_get = fields.iter().map(|f| format_ident!("get_{}", f.name)).collect::<Vec<_>>();
    let fn_get_comment = fields.iter().map(|f| format!("Get a {} value out of this cost object, defaulting to zero if not found", f.name)).collect::<Vec<_>>();
    let field_name = fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>();
    let field_name_mut = fields.iter().map(|f| format_ident!("{}_mut", f.name)).collect::<Vec<_>>();
    let field_hashkey = fields.iter().map(|f| f.hash_key.clone()).collect::<Vec<_>>();
    let field_hashval = fields.iter().map(|f| f.hash_val.clone()).collect::<Vec<_>>();

    let cost_impl = quote! {
        impl #name {
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

            /// round all values to a standard decimal place
            fn round(&mut self) {
                let credits = self.credits_mut();
                *credits = Costs::do_round(credits);
                #(
                    for val in self.#field_name_mut().values_mut() {
                        *val = Costs::do_round(val);
                    }
                )*
            }

            /// Strip zeros from our Costs values
            fn strip(&mut self) {
                let credits = self.credits_mut();
                *credits = credits.normalize();
                #(
                    for val in self.#field_name_mut().values_mut() {
                        *val = val.normalize();
                    }
                )*
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
                self.credits += other.credits().clone();
                #(
                    for k in other.#field_name().keys() {
                        let entry = self.#field_name_mut().entry(k.clone()).or_insert(#field_hashval::zero());
                        *entry += other.#field_name().get(k).unwrap();
                    }
                )*
                self.normalize();
                self
            }
        }

        impl Sub for Costs {
            type Output = Self;

            fn sub(mut self, other: Self) -> Self {
                self.credits -= other.credits().clone();
                #(
                    for k in other.#field_name().keys() {
                        let entry = self.#field_name_mut().entry(k.clone()).or_insert(#field_hashval::zero());
                        *entry -= other.#field_name().get(k).unwrap();
                    }
                )*
                self.normalize();
                self
            }
        }

        impl Mul<rust_decimal::Decimal> for Costs {
            type Output = Self;

            fn mul(mut self, rhs: rust_decimal::Decimal) -> Self {
                self.credits *= rhs.clone();
                #(
                    for (_, val) in self.#field_name_mut().iter_mut() {
                        *val *= rhs;
                    }
                )*
                self.normalize();
                self
            }
        }

        impl Div<Decimal> for Costs {
            type Output = Self;

            fn div(mut self, rhs: Decimal) -> Self::Output {
                if self.is_zero() {
                    return self;
                }
                if rhs == Decimal::zero() {
                    panic!("Costs::div() -- divide by zero");
                }
                self.credits /= rhs.clone();
                #(
                    for (_, v) in self.#field_name_mut().iter_mut() {
                        *v /= rhs;
                    }
                )*
                self.normalize();
                self
            }
        }
    };
    TokenStream::from(cost_impl)
}

