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
    models::{
        region::RegionID,
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
    /// between two or more regions.
    TransRegional(Vec<RegionID>),
    /// For planned companies that exist within a single region.
    ///
    /// Example: A regional medical research facility
    Regional(RegionID),
    /// For worker-owned companies that operate within a single region.
    ///
    /// Example: A local, worker-owned widget factory
    ///
    /// TODO!
    /// TODO! a company should be able to span multiple regions
    /// TODO!
    Syndicate(RegionID),
    /// For companies that exist outside of the Basis system.
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
    MemberSetPermissions,
    MemberDelete,

    LaborSetClock,
    LaborSetWage,

    ResourceSpecCreate,
    ResourceSpecUpdate,
    ResourceSpecDelete,

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

