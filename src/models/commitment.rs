use crate::{
    models::{
        agent::AgentID,
        agreement::AgreementID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    }
};
use vf_rs::vf;

basis_model! {
    pub struct Commitment {
        inner: vf::Commitment<AgreementID, AgreementID, AgentID, (), ProcessID, AgentID, (), ResourceSpecID, ResourceID>,
    }
    CommitmentID
    CommitmentBuilder
}

