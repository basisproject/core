use crate::{
    costs::Costs,
    models::{
        agent::AgentID,
        process::ProcessID,
        resource_spec::ResourceSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Resource {
        /// The VF object for this product instance
        inner: vf::EconomicResource<Url, ResourceSpecID, ResourceID, AgentID, ProcessID>,
        /// The company that has custody of the resource
        in_custody_of: AgentID,
        /// The costs imbued in this resource
        costs: Costs,
    }
    ResourceID
    ResourceBuilder
}

