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
    util::measure,
};
use om2::Unit;
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

impl Resource {
    /// Get this resource's Unit (if it has it)
    pub fn get_unit(&self) -> Option<Unit> {
        self.inner().accounting_quantity().clone().or_else(|| self.inner().onhand_quantity().clone())
            .map(|measure| measure.has_unit().clone())
    }

    /// Zero out the accounting/onhand quantity measurements for this resource.
    pub fn zero_measures(&mut self) {
        self.inner_mut().accounting_quantity_mut().as_mut()
            .map(|x| measure::set_zero(x));
        self.inner_mut().onhand_quantity_mut().as_mut()
            .map(|x| measure::set_zero(x));
    }
}

impl CostMover for Resource {
    fn costs(&self) -> &Costs {
        self.costs()
    }

    fn set_costs(&mut self, costs: Costs) {
        self.set_costs(costs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::CompanyID,
        },
        util::{self, test::*},
    };
    use om2::{Measure, Unit};

    #[test]
    fn compare() {
        let now = util::time::now();
        let id1 = ResourceID::new("widget1");
        let id2 = ResourceID::new("widget2");
        let company_id1 = CompanyID::new("jerry's widgets");
        let company_id2 = CompanyID::new("frank's widgets");
        let measure = Measure::new(50, Unit::Kilogram);
        let costs = Costs::new_with_labor("machinist", num!(23.2));
        let resource1 = make_resource(&id1, &company_id1, &measure, &costs, &now);
        let resource2 = make_resource(&id2, &company_id2, &measure, &costs, &now);

        assert!(resource1 == resource1);
        assert!(resource2 == resource2);
        assert!(resource1.clone() == resource1.clone());
        assert!(resource2.clone() == resource2.clone());
        assert!(resource1 != resource2);
        let mut resource3 = resource2.clone();
        assert!(resource1 != resource3);
        resource3.set_id(id1.clone());
        assert!(resource1 != resource3);
        resource3.set_in_custody_of(company_id1.clone().into());
        assert!(resource1 != resource3);
        resource3.inner_mut().set_primary_accountable(Some(company_id1.clone().into()));
        assert!(resource1 == resource3);
        resource3.set_costs(Costs::new_with_labor("machinist", num!(23.1)));
        assert!(resource1 != resource3);
        resource3.set_costs(Costs::new_with_labor("machinist", num!(23.2)));
        assert!(resource1 == resource3);
    }

    #[test]
    fn get_unit() {
        let now = util::time::now();
        let resource = make_resource(&ResourceID::create(), &CompanyID::create(), &Measure::new(69, Unit::Litre), &Costs::new_with_labor("TUNA", 54), &now);
        let mut resource2 = resource.clone();
        resource2.inner_mut().set_accounting_quantity(None);
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_onhand_quantity(None);
        let mut resource4 = resource2.clone();
        resource4.inner_mut().set_onhand_quantity(None);
        assert_eq!(resource.get_unit(), Some(Unit::Litre));
        assert_eq!(resource2.get_unit(), Some(Unit::Litre));
        assert_eq!(resource3.get_unit(), Some(Unit::Litre));
        assert_eq!(resource4.get_unit(), None);


    }
}

