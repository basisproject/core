/// A trait that all model IDs implement.
pub trait ModelID: Into<String> + From<String> + Clone + PartialEq + Eq + std::hash::Hash {}

#[macro_export]
macro_rules! basis_model {
    (
        $(#[$struct_meta:meta])*
        pub struct $model:ident {
            id: <<$id:ident>>,
            $($fields:tt)*
        }
        $builder:ident

    ) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $id(String);

        impl $id {
            pub fn new<T: Into<String>>(id: T) -> Self {
                Self(id.into())
            }

            pub fn create() -> Self {
                Self(uuid::Uuid::new_v4().to_hyphenated().encode_lower(&mut uuid::Uuid::encode_buffer()).to_string())
            }

            /// Convert this ID to a string
            pub fn to_string(self) -> String {
                self.into()
            }

            /// Return a string ref for this ID
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl std::convert::Into<String> for $id {
            fn into(self) -> String {
                let $id(val) = self;
                val
            }
        }

        impl std::convert::From<String> for $id {
            fn from(id: String) -> Self {
                Self(id)
            }
        }

        impl std::convert::From<&str> for $id {
            fn from(id: &str) -> Self {
                Self(id.to_string())
            }
        }

        impl crate::models::lib::basis_model::ModelID for $id {}

        basis_model_inner! {
            $(#[$struct_meta])*
            #[derive(Clone, Debug, PartialEq, getset::Getters, getset::MutGetters, getset::Setters, derive_builder::Builder, serde::Serialize, serde::Deserialize)]
            #[builder(pattern = "owned", setter(into))]
            #[getset(get = "pub", get_mut, set)]
            pub struct $model {
                id: $id,
                $($fields)*
                #[builder(default)]
                active: bool,
                created: chrono::DateTime<chrono::Utc>,
                updated: chrono::DateTime<chrono::Utc>,
                deleted: Option<chrono::DateTime<chrono::Utc>>,
            }
        }

        impl $model {
            fn builder() -> $builder {
                $builder::default()
            }

            pub fn is_active(&self) -> bool {
                self.active && !self.is_deleted()
            }

            pub fn is_deleted(&self) -> bool {
                self.deleted.is_some()
            }
        }

        pub fn builder() -> $builder {
            $model::builder()
        }

        /// Defining public setters as functions means we can later export JUST
        /// the model (with no setter functions) since the setters won't be an
        /// impl on the models. While this makes the inner code a bit more obtuse
        /// (because we have to do `model::set::created(&mut mymodel, created)`
        /// instead of `mymodel.set_created(created)`, it allows us to protect our
        /// inner API and clearly define our rules for engaging with the system.
        pub mod set {
            use super::*;

            #[allow(dead_code)]
            pub fn created(model: &mut $model, created: chrono::DateTime<chrono::Utc>) {
                model.set_created(created);
            }

            #[allow(dead_code)]
            pub fn updated(model: &mut $model, updated: chrono::DateTime<chrono::Utc>) {
                model.set_updated(updated);
            }

            #[allow(dead_code)]
            pub fn deleted(model: &mut $model, deleted: Option<chrono::DateTime<chrono::Utc>>) {
                model.set_deleted(deleted);
            }

            basis_setters! {
                $model,
                $($fields)*
            }
        }

        /// Defining public mutgetters as functions means we can later export
        /// JUST the model (with no setter functions) since the setters won't be
        /// an impl on the models. While this makes the inner code a bit more
        /// obtuse (because we have to do `model::mut::inner(&mut mymodel).set_val(69)`
        /// instead of `mymodel.inner_mut().set_val(69)`, it allows us to protect
        /// our inner API and clearly define our rules for engaging with the
        /// system.
        pub mod getmut {
            use super::*;

            basis_mutgetters! {
                $model,
                $($fields)*
            }
        }
    }
}

macro_rules! basis_setters {
    (
        $model:ident,
        $(
            $(#[$field_meta:meta])*
            $field_name:ident: $field_type:ty,
        )*
    ) => {
        $(
            #[allow(dead_code)]
            pub fn $field_name(model: &mut $model, val: $field_type) {
                // NOTE: i'd love to use the model's setters here, but until
                // rust macros can concat idents, i'm not dealing with a fucking
                // proc_macro just to define setters. let's get this moving
                // forward without bikeshedding about proper use of setters for
                // 12 years.
                model.$field_name = val;
            }
        )*
    };
}

macro_rules! basis_mutgetters {
    (
        $model:ident,
        $(
            $(#[$field_meta:meta])*
            $field_name:ident: $field_type:ty,
        )*
    ) => {
        $(
            #[allow(dead_code)]
            pub fn $field_name(model: &mut $model) -> &mut $field_type {
                // NOTE: i'd love to use the model's mut getters here, but until
                // rust macros can concat idents, i'm not dealing with a fucking
                // proc_macro just to define setters. let's get this moving
                // forward without bikeshedding about proper use of setters for
                // 12 years.
                &mut model.$field_name
            }
        )*
    };
}

/// Applies meta to various fields depending on their type
macro_rules! basis_model_inner {
    // grab Vec fields and apply special meta
    (
        @parse_fields ($($parsed_fields:tt)*)
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $(#[$field_meta:meta])*
            $field_name:ident: Vec<$field_type:ty>,

            $($fields:tt)*
        }
    ) => {
        basis_model_inner! {
            @parse_fields (
                $($parsed_fields)*

                $(#[$field_meta])*
                #[builder(default)]
                #[serde(default = "Default::default", skip_serializing_if = "Vec::is_empty")]
                $field_name: Vec<$field_type>,
            )
            $(#[$struct_meta])*
            pub struct $name {
                $($fields)*
            }
        }
    };

    // grab Option fields and apply special meta
    (
        @parse_fields ($($parsed_fields:tt)*)
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $(#[$field_meta:meta])*
            $field_name:ident: Option<$field_type:ty>,

            $($fields:tt)*
        }
    ) => {
        basis_model_inner! {
            @parse_fields (
                $($parsed_fields)*

                $(#[$field_meta])*
                #[builder(default)]
                #[serde(default = "Default::default", skip_serializing_if = "Option::is_none")]
                $field_name: Option<$field_type>,
            )
            $(#[$struct_meta])*
            pub struct $name {
                $($fields)*
            }
        }
    };

    // parse "normal" fields
    (
        @parse_fields ($($parsed_fields:tt)*)
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $(#[$field_meta:meta])*
            $field_name:ident: $field_type:ty,

            $($fields:tt)*
        }
    ) => {
        basis_model_inner! {
            @parse_fields (
                $($parsed_fields)*

                $(#[$field_meta])*
                $field_name: $field_type,
            )
            $(#[$struct_meta])*
            pub struct $name {
                $($fields)*
            }
        }
    };

    // all done
    (
        @parse_fields ($($parsed_fields:tt)*)
        $(#[$struct_meta:meta])*
        pub struct $name:ident {}
    ) => {
        $(#[$struct_meta])*
        pub struct $name {
            $($parsed_fields)*
        }
    };

    // entry
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $($fields:tt)*
        }
    ) => {
        basis_model_inner! {
            @parse_fields ()
            $(#[$struct_meta])*
            pub struct $name {
                $($fields)*
            }
        }
    };
}

