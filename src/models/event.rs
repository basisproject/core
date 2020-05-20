use crate::{
    models::{
        agent::AgentID,
        company::CompanyID,
        process::ProcessID,
    },
};
use vf_rs::vf;

basis_model! {
    pub struct Event {
        event: vf::EconomicEvent<(), CompanyID, ProcessID, AgentID, (), (), (), (), EventID>,
    }
    EventID
    EventBuilder
}

