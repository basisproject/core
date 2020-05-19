use crate::{
    models::{
        company::CompanyID,
        process_spec::ProcessSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Process {
        process: vf::Process<ProcessSpecID, Url, CompanyID, (), ()>,
    }
    ProcessID
    ProcessBuilder
}

