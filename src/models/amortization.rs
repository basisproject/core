use chrono::{DateTime, Utc};
use crate::{
    models::company::CompanyID,
    models::costs::Costs,
};

basis_model! {
    pub struct Amortization {
        company_id: CompanyID,
        name: String,
        ceiling: Costs,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    }
    AmortizationID
    AmortizationBuilder
}

