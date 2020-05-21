use crate::{
    costs::Costs,
    models::{
        agent::AgentID,
        company::CompanyID,
        process::ProcessID,
        resource_spec::ResourceSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Resource {
        /// The VF object for this product instance
        economic_resource: vf::EconomicResource<Url, ResourceSpecID, ResourceID, AgentID, ProcessID>,
        /// The current owner of the resource
        owned_by: CompanyID,
        /// The costs imbued in this resource
        costs: Costs,
    }
    ResourceID
    ResourceBuilder
}

