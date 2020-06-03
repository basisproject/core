/// A trait that all model IDs implement.
pub trait ModelID: Into<String> + From<String> + Clone + PartialEq + Eq + std::hash::Hash {}

macro_rules! basis_model {
    (
        $(#[$struct_meta:meta])*
        pub struct $model:ident {
            id: <<$id:ident>>,
            $($fields:tt)*
        }
        $builder:ident

    ) => {
        /// ID type for this model.
        #[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $id(String);

        impl $id {
            /// Create a new id from a val
            pub fn new<T: Into<String>>(id: T) -> Self {
                Self(id.into())
            }

            /// Create a new random id (UUIDv4)
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

        impl std::convert::From<$id> for String {
            fn from(id: $id) -> Self {
                let $id(val) = id;
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

        pub(crate) mod inner {
            use super::*;

            basis_model_inner! {
                $(#[$struct_meta])*
                #[derive(Clone, Debug, PartialEq, getset::Getters, getset::MutGetters, getset::Setters, derive_builder::Builder, serde::Serialize, serde::Deserialize)]
                #[builder(pattern = "owned", setter(into))]
                #[getset(get = "pub", get_mut = "pub(crate)", set = "pub(crate)")]
                pub struct $model {
                    /// The model's ID, used to link to it from other models
                    id: $id,
                    $($fields)*
                    /// The `active` field allows a model to be deactivated,
                    /// which keeps it in data while only allowing it to be used
                    /// or altered in specific ways.
                    #[builder(default)]
                    active: bool,
                    /// Notes when the model was created.
                    created: chrono::DateTime<chrono::Utc>,
                    /// Notes when the model was last updated.
                    updated: chrono::DateTime<chrono::Utc>,
                    /// Notes if the model has been deleted, which has the same
                    /// effect of deactivation, but is permanent.
                    deleted: Option<chrono::DateTime<chrono::Utc>>,
                }
            }

            impl $model {
                /// Returns a builder for this model
                #[allow(dead_code)]
                pub(crate) fn builder() -> $builder {
                    $builder::default()
                }

                /// Determine if this model is active. This also reads the
                /// `deleted` field and will return false if the model has been
                /// deleted, so this method is the canonical place to check if
                /// the model can be used.
                pub fn is_active(&self) -> bool {
                    self.active && !self.is_deleted()
                }

                /// Checks if the model has been deleted.
                pub fn is_deleted(&self) -> bool {
                    self.deleted.is_some()
                }
            }

            impl std::convert::From<$model> for crate::models::Model {
                fn from(val: $model) -> Self {
                    crate::models::Model::$model(val)
                }
            }

            impl std::convert::TryFrom<crate::models::Model> for $model {
                type Error = crate::error::Error;

                fn try_from(val: crate::models::Model) -> std::result::Result<Self, Self::Error> {
                    match val {
                        crate::models::Model::$model(val) => Ok(val),
                        _ => Err(crate::error::Error::WrongModelType),
                    }
                }
            }
        }
        pub use inner::$model;
    }
}

/// Applies meta to various model fields depending on their type
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

