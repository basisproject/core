#[macro_export]
macro_rules! basis_model {
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $($fields:tt)*
        }
        $id:ident
        $builder:ident

    ) => {
        #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $id(String);

        impl $id {
            pub fn new<T: Into<String>>(id: T) -> Self {
                Self(id.into())
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

        $(#[$struct_meta])*
        #[derive(Clone, Debug, PartialEq, getset::Getters, getset::Setters, derive_builder::Builder, serde::Serialize, serde::Deserialize)]
        #[builder(pattern = "owned", setter(into))]
        #[getset(get = "pub", set = "pub")]
        pub struct $name {
            id: $id,
            $($fields)*
            #[builder(default)]
            active: bool,
            created: chrono::DateTime<chrono::Utc>,
            updated: chrono::DateTime<chrono::Utc>,
            #[builder(setter(strip_option), default)]
            #[serde(skip_serializing_if = "Option::is_none")]
            deleted: Option<chrono::DateTime<chrono::Utc>>,
        }

        impl $name {
            pub fn builder() -> $builder {
                $builder::default()
            }

            pub fn is_active(&self) -> bool {
                self.active && !self.is_deleted()
            }

            pub fn is_deleted(&self) -> bool {
                self.deleted.is_some()
            }
        }
    }
}

