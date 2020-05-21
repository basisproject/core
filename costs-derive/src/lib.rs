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
    let fn_muldiv_rhs = fields.iter().map(|f| -> syn::Expr {
        if f.hash_val == format_ident!("Decimal") {
            syn::parse_str("Decimal::from_f64(rhs).unwrap()").unwrap()
        } else {
            syn::parse_str("rhs").unwrap()
        }
    }).collect::<Vec<_>>();

    let cost_impl = quote! {
        impl #name {
            #(
                #[doc = #fn_new_with_comment]
                pub fn #fn_new_with<T: Into<#field_hashkey>>(id: T, #field_name: #field_hashval) -> Self {
                    let mut costs = Self::new();
                    costs.#fn_track(id, #field_name);
                    costs
                }
            )*
            #(
                #[doc = #fn_track_comment]
                pub fn #fn_track<T: Into<#field_hashkey>>(&mut self, id: T, val: #field_hashval) {
                    if val < Zero::zero() {
                        panic!(#fn_track_panic);
                    }
                    let entry = self.#field_name_mut().entry(id.into()).or_insert(rust_decimal::prelude::Zero::zero());
                    *entry += val;
                }
            )*
            #(
                #[doc = #fn_get_comment]
                pub fn #fn_get<T: Into<#field_hashkey>>(&self, id: T) -> #field_hashval {
                    *self.#field_name().get(&id.into()).unwrap_or(&rust_decimal::prelude::Zero::zero())
                }
            )*

            /// Test if we hve an empty cost set
            pub fn is_zero(&self) -> bool {
                #(
                    for (_, val) in self.#field_name().iter() {
                        if val > &rust_decimal::prelude::Zero::zero() {
                            return false;
                        }
                    }
                )*
                true
            }

            /// Given a set of costs, subtract them from our current costs, but only if
            /// the result is >= 0 for each cost tracked. Then, return a costs object
            /// showing exactly how much was taken.
            pub fn take(&mut self, costs: &Costs) -> Costs {
                let mut new_costs = Costs::new();
                #(
                    for (k, lval) in self.#field_name_mut().iter_mut() {
                        let mut rval = costs.#field_name().get(k).unwrap_or(&Zero::zero()).clone();
                        let val = if lval > &mut rval { rval } else { lval.clone() };
                        *lval -= val;
                        new_costs.#fn_track(k.clone(), val.clone());
                    }
                )*
                new_costs
            }

            /// Determine if dividing one set of costs by another will result in
            /// a divide-by-zero panic.
            pub fn is_div_by_0(costs1: &Costs, costs2: &Costs) -> bool {
                #(
                    for (k, v) in costs1.#field_name().iter() {
                        let div = costs2.#field_name().get(k).map(|x| x.clone()).unwrap_or(Zero::zero());
                        if v == &Zero::zero() {
                            continue;
                        }
                        if div == Zero::zero() {
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
                        let entry = self.#field_name_mut().entry(k.clone()).or_insert(Zero::zero());
                        *entry += other.#field_name().get(k).unwrap();
                    }
                )*
                self
            }
        }

        impl Sub for Costs {
            type Output = Self;

            fn sub(mut self, other: Self) -> Self {
                #(
                    for k in other.#field_name().keys() {
                        let entry = self.#field_name_mut().entry(k.clone()).or_insert(Zero::zero());
                        *entry -= other.#field_name().get(k).unwrap();
                    }
                )*
                self
            }
        }

        impl Mul for Costs {
            type Output = Self;

            fn mul(mut self, rhs: Self) -> Self {
                #(
                    for (k, val) in self.#field_name_mut().iter_mut() {
                        *val *= rhs.#field_name().get(k).unwrap_or(&Zero::zero());
                    }
                )*
                self
            }
        }

        impl Mul<f64> for Costs {
            type Output = Self;

            fn mul(mut self, rhs: f64) -> Self {
                #(
                    for (_, val) in self.#field_name_mut().iter_mut() {
                        *val *= #fn_muldiv_rhs;
                    }
                )*
                self
            }
        }

        impl Div for Costs {
            type Output = Self;

            fn div(mut self, rhs: Self) -> Self::Output {
                #(
                    for (k, v) in self.#field_name_mut().iter_mut() {
                        let div = rhs.#field_name().get(k).map(|x| x.clone()).unwrap_or(Zero::zero());
                        if v == &Zero::zero() {
                            continue;
                        }
                        if div == Zero::zero() {
                            panic!(#fn_div_panic, k);
                        }
                        *v /= div;
                    }
                    for (k, _) in rhs.#field_name().iter() {
                        match self.#field_name().get(k) {
                            None => {
                                self.#field_name_mut().insert(k.clone(), Zero::zero());
                            }
                            _ => {}
                        }
                    }
                )*
                self
            }
        }

        impl Div<f64> for Costs {
           type Output = Self;

           fn div(mut self, rhs: f64) -> Self::Output {
               if rhs == Zero::zero() {
                   panic!("Costs::div() -- divide by zero");
               }
               #(
                   for (_, v) in self.#field_name_mut().iter_mut() {
                       *v /= #fn_muldiv_rhs;
                   }
               )*
               self
           }
        }
    };
    TokenStream::from(cost_impl)
}

