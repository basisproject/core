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
    models::{
        lib::agent::{Agent, AgentID},
        process::Process,
        resource::Resource,
    },
};
use serde::{Serialize, Deserialize};
use vf_rs::vf;

/// A permission gives a Member the ability to perform certain actions
/// within the context of a company they have a relationship (a set of roles)
/// with. 
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// syndicates (exist by the will of the workers of that comany), or private
    /// (exist completely outside the system).
    pub struct Company {
        id: <<CompanyID>>,
        /// The Agent object for this company, stores its name, image, location,
        /// etc.
        inner: vf::Agent,
        /// Primary email address
        email: String,
    }
    CompanyBuilder
}

impl Company {
    /// Calculate the total costs for this company, given a set of processes and
    /// resources that belong to the company.
    pub fn total_costs(&self, processes: &Vec<Process>, resources: &Vec<Resource>) -> Costs {
        let process_costs = processes.iter()
            .filter(|x| x.company_id() == self.id())
            .fold(Costs::new(), |acc, x| acc + x.costs().clone());
        let resource_costs = resources.iter()
            .filter(|x| {
                match x.inner().primary_accountable() {
                    Some(agent_id) => {
                        match agent_id {
                            AgentID::CompanyID(company_id) => self.id() == company_id,
                            _ => false,
                        }
                    }
                    None => false,
                }
            })
            .fold(Costs::new(), |acc, x| acc + x.costs().clone());
        process_costs + resource_costs
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
        models::{
            process::ProcessID,
            resource::ResourceID,
        },
        util::{self, test::*},
    };
    use om2::{Measure, Unit};
    use rust_decimal_macros::*;

    #[test]
    fn totals_costs() {
        let company_id = CompanyID::create();
        let company = make_company(&company_id, "jerry's delicious widgets", &util::time::now());

        let now = util::time::now();
        let process1 = make_process(&ProcessID::create(), &company_id, "make widgets", &Costs::new_with_labor("lumberjack", dec!(16.9)), &now);
        let process2 = make_process(&ProcessID::create(), &company_id, "market widgets", &Costs::new_with_labor("marketer", dec!(123.4)), &now);
        let resource1 = make_resource(&ResourceID::create(), &company_id, &Measure::new(dec!(10.0), Unit::One), &Costs::new_with_labor("lumberjack", dec!(23.1)), &now);
        let resource2 = make_resource(&ResourceID::create(), &company_id, &Measure::new(dec!(10.0), Unit::One), &Costs::new_with_labor("trucker", dec!(12.5)), &now);

        let costs = company.total_costs(&vec![process1, process2], &vec![resource1, resource2]);
        let mut expected_costs = Costs::new();
        expected_costs.track_labor("lumberjack", dec!(40));
        expected_costs.track_labor("trucker", dec!(12.5));
        expected_costs.track_labor("marketer", dec!(123.4));
        assert_eq!(costs, expected_costs);

        let process1 = make_process(&ProcessID::create(), &CompanyID::create(), "make widgets", &Costs::new_with_labor("lumberjack", dec!(16.9)), &now);
        let process2 = make_process(&ProcessID::create(), &company_id, "market widgets", &Costs::new_with_labor("marketer", dec!(123.4)), &now);
        let resource1 = make_resource(&ResourceID::create(), &CompanyID::create(), &Measure::new(dec!(10.0), Unit::One), &Costs::new_with_labor("lumberjack", dec!(23.1)), &now);
        let resource2 = make_resource(&ResourceID::create(), &company_id, &Measure::new(dec!(10.0), Unit::One), &Costs::new_with_labor("trucker", dec!(12.5)), &now);

        let costs = company.total_costs(&vec![process1, process2], &vec![resource1, resource2]);
        let mut expected_costs = Costs::new();
        expected_costs.track_labor("trucker", dec!(12.5));
        expected_costs.track_labor("marketer", dec!(123.4));
        assert_eq!(costs, expected_costs);
    }
}

