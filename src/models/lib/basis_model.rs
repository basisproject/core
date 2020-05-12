#[macro_export]
macro_rules! basis_model {
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $($fields:tt)*
        }
        $builder:ident      // thinking of you, concat_idents!()...

    ) => {
        $(#[$struct_meta])*
        #[derive(Clone, Debug, PartialEq, getset::Getters, getset::Setters, derive_builder::Builder, serde::Serialize, serde::Deserialize)]
        #[builder(pattern = "owned", setter(into))]
        #[getset(get = "pub", set = "pub")]
        pub struct $name {
            id: String,
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

