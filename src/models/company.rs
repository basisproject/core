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
//! [Members]: ../company_member/struct.CompanyMember.html
//! [access]: ../../access/

use crate::{
    costs::Costs,
    models::{
        lib::agent::AgentID,
        process::Process,
        region::RegionID,
        resource::Resource,
    },
};
use serde::{Serialize, Deserialize};
use vf_rs::vf;

/// Describes different company types. Different types behave differently within
/// the system, and this is where we differentiate the behaviors.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CompanyType {
    /// For planned companies that span multiple regions.
    ///
    /// Example: an organization that manages a bridge or set of infrastructure
    /// between two or more regions, or a planned joint pharmaceutical research
    /// facility
    TransRegional(Vec<RegionID>),
    /// For planned companies that exist within a single region.
    ///
    /// Example: A regional transit system
    Regional(RegionID),
    /// For worker-owned companies that operate within the Basis network. Note
    /// that syndicates can span multiple regions (for instance, a company that
    /// has workers from several neighboring regions, or a company with many
    /// remote workers).
    ///
    /// Example: A local, worker-owned widget factory
    Syndicate,
    /// For (capitalist pig) companies that exist outside of the Basis system.
    ///
    /// Example: Amazon
    Private,
}

/// A permission gives a CompanyMember the ability to perform certain actions
/// within the context of a company they have a relationship (a set of roles)
/// with. 
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    All,

    CompanyUpdate,
    CompanyDelete,

    MemberCreate,
    MemberUpdate,
    MemberSetPermissions,
    MemberSetCompensation,
    MemberDelete,

    LaborSetClock,
    LaborSetWage,

    ResourceSpecCreate,
    ResourceSpecUpdate,
    ResourceSpecDelete,

    ResourceCreate,
    ResourceUpdate,
    ResourceDelete,

    ProcessSpecCreate,
    ProcessSpecUpdate,
    ProcessSpecDelete,

    ProcessCreate,
    ProcessUpdate,
    ProcessDelete,

    OrderCreate,
    OrderUpdateProcessStatus,
    OrderUpdateShipping,
    OrderUpdateShippingDates,
    OrderCancel,
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
        /// What type of company
        ty: CompanyType,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            process::ProcessID,
            resource::ResourceID,
            testutils::{make_company, make_process, make_resource},
        },
        util,
    };
    use om2::{Measure, Unit};
    use rust_decimal_macros::*;

    #[test]
    fn totals_costs() {
        let company_id = CompanyID::create();
        let company = make_company(&company_id, CompanyType::Syndicate, "jerry's delicious widgets", &util::time::now());

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

