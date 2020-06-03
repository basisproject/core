//! The currency module holds the Currency model, used to track various currency
//! types for purposes of [banking].
//!
//! [banking]: https://basisproject.gitlab.io/public/paper#chapter-6-banking

basis_model! {
    /// The currency model allows the backing system to track various currencies
    /// as they move through the system, which ultimately allows an accurate
    /// conversion between the internal credits and external monetary systems.
    pub struct Currency {
        id: <<CurrencyID>>,
        /// The name of the currency, probably some ISO value.
        name: String,
        /// How many decimal places this currency uses.
        decimal_scale: u32,
    }
    CurrencyBuilder
}

