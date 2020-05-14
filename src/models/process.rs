use crate::{
    models::company::CompanyID,
    models::process_spec::ProcessSpecID,
};
use vf_rs::vf;

basis_model! {
    pub struct Process {
        process: vf::Process<ProcessSpecID, CompanyID, ()>,
    }
    ProcessID
    ProcessBuilder
}

