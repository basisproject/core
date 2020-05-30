use crate::{
    models::{
        agreement::AgreementID,
        lib::agent::AgentID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    }
};
use vf_rs::vf;

basis_model! {
    pub struct Commitment {
        id: <<CommitmentID>>,
        inner: vf::Commitment<AgreementID, AgreementID, AgentID, (), ProcessID, AgentID, (), ResourceSpecID, ResourceID>,
    }
    CommitmentBuilder
}

