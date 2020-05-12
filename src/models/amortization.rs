use chrono::{DateTime, Utc};
use crate::{
    models::costs::Costs,
};

basis_model! {
    pub struct Amortization {
        company_id: String,
        name: String,
        costs: Costs,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    }
    AmortizationBuilder
}

