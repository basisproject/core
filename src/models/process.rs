use crate::{
    models::{
        amortization::AmortizationID,
        company::CompanyID,
        process_spec::ProcessSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Process {
        process: vf::Process<ProcessSpecID, Url, CompanyID, (), ()>,
        #[builder(default)]
        amortization_id: Option<AmortizationID>,
    }
    ProcessID
    ProcessBuilder
}

