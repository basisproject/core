use crate::{
    models::{
        agent::AgentID,
        company::CompanyID,
        process::ProcessID,
        resource::ResourceID,
        resource_spec::ResourceSpecID,
    },
};
use vf_rs::vf;

basis_model! {
    pub struct Event {
        event: vf::EconomicEvent<(), CompanyID, ProcessID, AgentID, (), (), ResourceSpecID, ResourceID, EventID>,
    }
    EventID
    EventBuilder
}

