use crate::{
    costs::{Costs, CostMover},
    models::{
        lib::agent::AgentID,
        process::ProcessID,
        resource_spec::ResourceSpecID,
    },
};
use url::Url;
use vf_rs::vf;

basis_model! {
    pub struct Resource {
        id: <<ResourceID>>,
        /// The VF object for this product instance
        inner: vf::EconomicResource<Url, ResourceSpecID, ResourceID, AgentID, ProcessID>,
        /// The company that has custody of the resource
        in_custody_of: AgentID,
        /// The costs imbued in this resource
        costs: Costs,
    }
    ResourceBuilder
}

impl CostMover for Resource {
    fn costs(&self) -> &Costs {
        self.costs()
    }

    fn set_costs(&mut self, costs: Costs) {
        self.set_costs(costs);
    }
}

