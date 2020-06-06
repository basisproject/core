//! A resource is a tangible asset. This can be anything, like a chair, a car,
//! a coat, or a carrot. It does not necessarily have to start with a "c". When
//! trying to understand resources, it's important to note that a resource is an
//! instance of a *resource specification*. When you look at a chair on Wamazon,
//! the page describes a resource specification. When the chair is shipped to
//! you, what you get is a resource (a manifestation of the chair specification).

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
    /// The resource model. Wraps the [vf::Resource][vfresource] object, and
    /// also tracks custody information as well as costs.
    ///
    /// [vfresource]: https://valueflo.ws/introduction/resources.html
    pub struct Resource {
        id: <<ResourceID>>,
        /// The VF object for this product instance
        inner: vf::EconomicResource<Url, ResourceSpecID, ResourceID, AgentID, ProcessID>,
        /// The agent that has custody of the resource
        in_custody_of: AgentID,
        /// The costs imbued in this resource. Note that the `inner` field's
        /// `vf::EconomicResource` object can contain a measure (ie, 5kg) and
        /// the costs attached to this resource are the *total* costs for the
        /// total measured resource. For instance, if our costs are `5 hours`
        /// and we have a measure of 16g, the `5 hours` cost encompasses all
        /// 16g.
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

