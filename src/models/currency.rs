basis_model! {
    pub struct Currency {
        id: <<CurrencyID>>,
        name: String,
        decimal_scale: u32,
    }
    CurrencyBuilder
}

