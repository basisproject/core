//! A company is a group of one or more people working towards a common economic
//! goal. Companies are the place where costs are accumulated and dispersed into
//! outgoing products and services.
//!
//! Companies have their own set of permissions that allow [Members] to perform
//! actions on the company. Note that while the [access system][access] uses
//! roles to contain various permissions, companies assign permissions directly.
//! This ultimately gives more control to companies to determine their own roles
//! (outside the perview of this library) as needed.
//!
//! [Members]: ../member/struct.Member.html
//! [access]: ../../access/

use crate::{
    costs::Costs,
    error::{Error, Result},
    models::{
        lib::agent::{Agent, AgentID},
    },
};
use rust_decimal::prelude::*;
#[cfg(feature = "with_serde")]
use serde::{Serialize, Deserialize};
use vf_rs::vf;

/// A permission gives a Member the ability to perform certain actions
/// within the context of a company they have a relationship (a set of roles)
/// with. 
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(Serialize, Deserialize))]
pub enum Permission {
    /// Can do anything
    All,

    /// Can accept a resource (for repair)
    Accept,

    /// Can create agreements (orders)
    AgreementCreate,
    /// Can finalize agreements (orders)
    AgreementFinalize,
    /// Can update agreements (orders)
    AgreementUpdate,

    /// Can cite a resource
    Cite,

    /// Can create a commitment
    CommitmentCreate,
    /// Can delete a commitment
    CommitmentDelete,
    /// Can update a commitment
    CommitmentUpdate,

    /// Can delete the company
    CompanyDelete,
    /// Can update the company's basic info
    CompanyUpdate,

    /// Can consume a resource
    Consume,

    /// Can deliver a service
    DeliverService,

    /// Can drop off (for delivery) a resource
    Dropoff,

    /// Can create a new intent
    IntentCreate,
    /// Can delete an intent
    IntentDelete,
    /// Can update an intent
    IntentUpdate,

    /// Can lower resource quantities within the company
    Lower,

    /// Can create new members (hire)
    MemberCreate,
    /// Can delete a member (fire)
    MemberDelete,
    /// Can set existing members' company permissions
    MemberSetPermissions,
    /// Can set a member's compensation (payment)
    MemberSetCompensation,
    /// Can update existing members' basic info
    MemberUpdate,

    /// Can modify a resource (for repair)
    Modify,

    /// Can move costs internally within the company
    MoveCosts,
    /// Can move resources internally within the company
    MoveResource,

    /// Can run payroll for this company
    Payroll,

    /// Can pick up (for delivery) a resource
    Pickup,

    /// Can create a process
    ProcessCreate,
    /// Can create a process
    ProcessDelete,
    /// Can create a process
    ProcessUpdate,

    /// Can create a process spec
    ProcessSpecCreate,
    /// Can create a process spec
    ProcessSpecDelete,
    /// Can create a process spec
    ProcessSpecUpdate,

    /// Can produce a resource
    Produce,

    /// Can raise resource quantities within the company
    Raise,

    /// Can create a resource
    ResourceCreate,
    /// Can delete a resource
    ResourceDelete,
    /// Can update a resource
    ResourceUpdate,

    /// Can create a resource spec
    ResourceSpecCreate,
    /// Can delete a resource spec
    ResourceSpecDelete,
    /// Can update a resource spec
    ResourceSpecUpdate,

    /// Transfer ownership/custody to another agent
    Transfer,
    /// Transfer ownership to another agent
    TransferAllRights,
    /// Transfer custody to another agent
    TransferCustody,

    /// Can use a resource in a productive process
    Use,

    /// Can record labor
    Work,
    /// Can update labor records willy-nilly
    WorkAdmin,
}

basis_model! {
    /// A company is a group of one or more people working together for a common
    /// purpose.
    ///
    /// Companies can be planned (exist by the will of the system members),
    /// syndicates (exist by the will of the workers of that company), or private
    /// (exist completely outside the system).
    pub struct Company {
        id: <<CompanyID>>,
        /// The Agent object for this company, stores its name, image, location,
        /// etc.
        inner: vf::Agent,
        /// Primary email address
        email: String,
        /// A credit value tracking this company's maximum costs
        max_costs: Decimal,
        /// The total amount of costs this company possesses. Cannot be above
        /// `max_costs` when converted to a credit value.
        total_costs: Costs,
    }
    CompanyBuilder
}

impl Company {
    /// Add a set of costs to this company, checking to make sure we are not
    /// above `max_costs`. Returns the company's post-op `total_costs` value.
    pub(crate) fn increase_costs(&mut self, costs: Costs) -> Result<&Costs> {
        if costs.is_lt_0() {
            Err(Error::NegativeCosts)?;
        }
        let new_costs = self.total_costs().clone() + costs;
        let credit_value = new_costs.credits();
        if credit_value > self.max_costs() {
            Err(Error::MaxCostsReached)?;
        }
        self.set_total_costs(new_costs);
        Ok(self.total_costs())
    }

    /// Subtract a set of costs to this company. Returns the company's post-op
    /// `total_costs` value.
    ///
    /// Note that we don't need to check if we're over our `max_costs` value
    /// because we are reducing costs here.
    fn decrease_costs(&mut self, costs: Costs) -> Result<&Costs> {
        if costs.is_lt_0() {
            Err(Error::NegativeCosts)?;
        }
        let total = self.total_costs().clone();
        if Costs::is_sub_lt_0(&total, &costs) {
            Err(Error::NegativeCosts)?;
        }
        self.set_total_costs(total - costs);
        Ok(self.total_costs())
    }

    /// Transfer a set of costs from this company to another. The receiving
    /// company must not go over their `max_costs` value.
    pub fn transfer_costs_to(&mut self, company_to: &mut Company, costs: Costs) -> Result<&Costs> {
        self.decrease_costs(costs.clone())?;
        company_to.increase_costs(costs)?;
        Ok(self.total_costs())
    }
}

impl Agent for Company {
    fn agent_id(&self) -> AgentID {
        self.id().clone().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        util::{self, test::*},
    };
    use rust_decimal_macros::*;

    #[test]
    fn increase_costs() {
        let mut company = make_company(&CompanyID::create(), "jerry's delicious widgets", &util::time::now());
        company.set_max_costs(dec!(1000));
        let costs1 = Costs::new_with_labor("widgetmaker", 500);
        let total_costs = company.increase_costs(costs1.clone()).unwrap();
        assert_eq!(total_costs, &costs1);

        let costs2 = Costs::new_with_labor("truck driver", 400);
        let total_costs = company.increase_costs(costs2.clone()).unwrap();
        assert_eq!(total_costs, &(costs1.clone() + costs2.clone()));

        let costs3 = Costs::new_with_labor("CEO. THE BEST CEO. BIG HANDS", 200);
        let res = company.increase_costs(costs3.clone());
        assert_eq!(res, Err(Error::MaxCostsReached));

        let costs4 = Costs::new_with_labor("CEO. THE BEST CEO. BIG HANDS", 100);
        let total_costs = company.increase_costs(costs4.clone()).unwrap();
        assert_eq!(total_costs, &(costs1.clone() + costs2.clone() + costs4.clone()));
    }

    #[test]
    fn decrease_costs() {
        let mut company = make_company(&CompanyID::create(), "jerry's delicious widgets", &util::time::now());
        company.set_max_costs(dec!(2000));
        let mut costs = Costs::new();
        costs.track_labor("machinist", dec!(500));
        costs.track_labor("ceo", dec!(800));
        company.set_total_costs(costs.clone());

        let mut costs1 = Costs::new();
        costs1.track_labor("machinist", dec!(100));
        costs1.track_labor("ceo", dec!(100));
        let mut comp = Costs::new();
        comp.track_labor("machinist", dec!(400));
        comp.track_labor("ceo", dec!(700));
        let total_costs = company.decrease_costs(costs1).unwrap();
        assert_eq!(total_costs, &comp);

        let mut costs2 = Costs::new();
        costs2.track_labor("machinist", dec!(350));
        costs2.track_labor("ceo", dec!(600));
        let mut comp = Costs::new();
        comp.track_labor("machinist", dec!(50));
        comp.track_labor("ceo", dec!(100));
        let total_costs = company.decrease_costs(costs2).unwrap();
        assert_eq!(total_costs, &comp);

        let mut costs3 = Costs::new();
        costs3.track_labor("machinist", dec!(400));
        costs3.track_labor("ceo", dec!(600));
        let res = company.decrease_costs(costs3);
        assert_eq!(res, Err(Error::NegativeCosts));

        let mut costs4 = Costs::new();
        costs4.track_labor("marketing", dec!(10));
        let res = company.decrease_costs(costs4);
        assert_eq!(res, Err(Error::NegativeCosts));

        let mut costs5 = Costs::new();
        costs5.track_labor("marketing", dec!(10));
        costs5 = Costs::new() - costs5.clone();
        let res = company.decrease_costs(costs5);
        assert_eq!(res, Err(Error::NegativeCosts));
    }
}

