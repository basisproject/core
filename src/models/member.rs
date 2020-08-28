//! A company member represents a link between a user in the system and a
//! company, and carries other information with it such as position (occupation)
//! in the company, access permissions, and compensation.
//!
//! Members can perform labor into a [Process] within the company, which earns
//! them credits and adds costs to the company which much be assigned to
//! outgoing products and services.
//!
//! [Process]: ../process/struct.Process.html

use crate::{
    error::{Error, Result},
    models::{
        account::AccountID,
        company::{CompanyID, Permission},
        lib::{
            agent::{Agent, AgentID},
            basis_model::ActiveState,
        },
        occupation::OccupationID,
        user::UserID,
    },
};
use getset::{Getters, Setters};
use om2::{Measure, Unit, NumericUnion};
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
use std::convert::TryInto;
use url::Url;
use vf_rs::vf;

/// How often we pay workers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PayrollSchedule {
    /// Once every two weeks
    BiWeekly,
    /// Twice a month (on the 15th and the last of the month)
    SemiMonthly,
}

/// Defines compensation for a member. Handles wage, payment schedule, and
/// account information.
///
/// Can account for hourly wages or salary.
#[derive(Clone, Debug, PartialEq, Getters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Compensation {
    /// A measure of value per time (ie, credits per hour, or credits per year)
    wage: Measure,
    /// Pay into this account
    pay_into: AccountID,
    /// Our payroll schedule (biweekly, semimonthly, etc)
    schedule: PayrollSchedule,
    /// If the `period` is not hourly, we can give an estimate for the number of
    /// hours worked per week, which gives us an ability to estimate our labor
    /// hours (and not just wage payments)
    est_hours_per_week: Option<Decimal>,
}

impl Compensation {
    /// Create a standard hourly wage, paid biweekly
    pub fn new_hourly<T, A>(wage: T, pay_into: A) -> Self
        where T: Into<Decimal>,
              A: Into<AccountID>,
    {
        Self::new_hourly_with_schedule(wage, pay_into, PayrollSchedule::BiWeekly)
    }

    /// Create an hourly wage
    pub fn new_hourly_with_schedule<T, A>(wage: T, pay_into: A, schedule: PayrollSchedule) -> Self
        where T: Into<Decimal>,
              A: Into<AccountID>,
    {
        Self {
            wage: Measure::new(NumericUnion::Decimal(wage.into()), Unit::Hour),
            pay_into: pay_into.into(),
            schedule: schedule,
            est_hours_per_week: None,
        }
    }

    /// Create a standard yearly salary, paid semimonthly
    pub fn new_salary<T, A>(wage: T, pay_into: A, est_hours_per_week: Decimal) -> Self
        where T: Into<Decimal>,
              A: Into<AccountID>,
    {
        Self::new_salary_with_schedule(wage, pay_into, PayrollSchedule::SemiMonthly, est_hours_per_week)
    }

    /// Create a salary
    pub fn new_salary_with_schedule<T, A>(wage: T, pay_into: A, schedule: PayrollSchedule, est_hours_per_week: Decimal) -> Self
        where T: Into<Decimal>,
              A: Into<AccountID>,
    {
        Self {
            wage: Measure::new(NumericUnion::Decimal(wage.into()), Unit::Year),
            pay_into: pay_into.into(),
            schedule: schedule,
            est_hours_per_week: Some(est_hours_per_week),
        }
    }
}

/// Describes a company that is a member of a company.
#[derive(Clone, Debug, PartialEq, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct MemberCompany {
}

impl MemberCompany {
    /// Create a new company member
    pub fn new() -> Self {
        Self {}
    }
}

/// Describes an individual user who is a member of a company.
#[derive(Clone, Debug, PartialEq, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct MemberUser {
}

impl MemberUser {
    /// Create a new company member
    pub fn new() -> Self {
        Self {}
    }
}

/// Describes a worker who is a member of a company.
#[derive(Clone, Debug, PartialEq, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub(crate)")]
pub struct MemberWorker {
    /// Holds the id of this worker's occupation at this company.
    ///
    /// Note that this could be held in VF's `AgentRelationship::relationship`
    /// field, but since that object lives in the top-level member model and the
    /// occupation really only applies to worker members, it is a conscious
    /// decision to put occupation in the worker struct.
    occupation: OccupationID,
    /// Describes how the member is compensated for their labor. Must be
    /// defined for the member to perform labor.
    compensation: Option<Compensation>,
}

impl MemberWorker {
    /// Create a new worker member
    pub fn new<T: Into<OccupationID>>(occupation_id: T, compensation: Option<Compensation>) -> Self {
        Self {
            occupation: occupation_id.into(),
            compensation,
        }
    }
}

/// Describes the type of membership for a particular Member record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MemberClass {
    /// This member is another company.
    ///
    /// A company which is a member of greater company automatically implies
    /// its members are also members of the greater company.
    Company(MemberCompany),
    /// This member is a user.
    ///
    /// A user member is a non-productive member that generally has use of the
    /// assets of the greater company. This might be things like housing,
    /// infrastructure, etc.
    User(MemberUser),
    /// This member is a worker.
    ///
    /// Worker members are productive members of a company. They generally have
    /// a wage/occupation
    Worker(MemberWorker),
}

basis_model! {
    /// A member of a company. Links a user to a company, and has other attached
    /// information like compensation, permission roles, etc.
    pub struct Member {
        id: <<MemberID>>,
        /// Our inner VF relationship (stores the AgentIDs of both the parties
        /// involved in the relationship under `subject`/`object`).
        inner: vf::AgentRelationship<(), AgentID, ()>,
        /// Membership class (company or user). This also holds our permissions
        /// for user members.
        class: MemberClass,
        /// The permissions this member has at this company (additive)
        permissions: Vec<Permission>,
        /// Agreement under which this membership takes place. This can be an
        /// employee agreement, or any general membership agreement (for
        /// instance, there might be a "you can be a member of this housing
        /// company as long as you don't burn down your house" agreement that
        /// user members would need to agree to).
        agreement: Option<Url>,
    }
    MemberBuilder
}

impl Member {
    /// Determines if a member can perform an action (base on their permissions
    /// list). Note that we don't use roles here, the idea is that companies
    /// manage their own roles and permissions are assigned to users directly.
    pub fn can(&self, permission: &Permission) -> bool {
        if !self.is_active() {
            return false;
        }
        self.permissions().contains(&Permission::All) ||
            self.permissions().contains(permission)
    }

    /// Check if this member can perform an action on a company.
    pub fn access_check(&self, user_id: &UserID, company_id: &CompanyID, permission: Permission) -> Result<()> {
        if self.inner().subject() != &user_id.clone().into() || self.inner().object() != &company_id.clone().into() || !self.can(&permission) {
            Err(Error::InsufficientPrivileges)?;
        }
        Ok(())
    }

    /// Grab the the member's agent id for this member record
    pub fn member_id(&self) -> &AgentID {
        self.inner().subject()
    }

    /// Grab the the groups's agent id for this member record
    pub fn group_id(&self) -> &AgentID {
        self.inner().object()
    }

    /// Try and get a `CompanyID` from this member's group id.
    pub fn company_id(&self) -> Result<CompanyID> {
        self.group_id().clone().try_into()
    }

    /// Grab this member's occupation id, if it has one
    pub fn occupation_id<'a>(&'a self) -> Option<&'a OccupationID> {
        match self.class() {
            MemberClass::Worker(worker) => Some(worker.occupation()),
            _ => None,
        }
    }

    /// Grab this member's compensation object, if it has one
    pub fn compensation<'a>(&'a self) -> Option<&'a Compensation> {
        match self.class() {
            MemberClass::Worker(worker) => worker.compensation().as_ref(),
            _ => None,
        }
    }
}

impl Agent for Member {
    fn agent_id(&self) -> AgentID {
        self.id().clone().into()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        models::{
            company::{CompanyID, Permission as CompanyPermission},
            user::UserID,
            testutils::make_member_worker,
        },
        util,
    };
    use std::convert::TryInto;
    use super::*;

    #[test]
    fn can() {
        let now = util::time::now();
        let member = make_member_worker(&MemberID::create(), &UserID::create(), &CompanyID::create(), &OccupationID::create(), vec![CompanyPermission::MemberCreate, CompanyPermission::MemberUpdate], &now);
        let user_id: UserID = member.member_id().clone().try_into().unwrap();
        let company_id: CompanyID = member.group_id().clone().try_into().unwrap();
        assert!(member.can(&CompanyPermission::MemberCreate));
        assert!(member.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_ok());
        assert!(member.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_err());

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::MemberCreate, CompanyPermission::MemberUpdate, CompanyPermission::CompanyDelete]);
        assert!(member2.can(&CompanyPermission::MemberCreate));
        assert!(member2.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_ok());
        assert!(member2.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_ok());

        let mut member3 = member2.clone();
        member3.set_permissions(vec![]);
        assert!(!member3.can(&CompanyPermission::MemberCreate));
        assert!(member3.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_err());
        assert!(member3.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_err());

        let mut member4 = member2.clone();
        member4.set_deleted(Some(now.clone()));
        assert!(!member4.can(&CompanyPermission::MemberCreate));
        assert!(member4.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_err());
        assert!(member4.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_err());

        let mut member5 = member2.clone();
        member5.set_active(false);
        assert!(!member5.can(&CompanyPermission::MemberCreate));
        assert!(member5.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_err());
        assert!(member5.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_err());

        let mut member6 = member2.clone();
        member6.inner_mut().set_subject(UserID::create().into());
        assert!(member6.can(&CompanyPermission::MemberCreate));
        assert!(member6.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_err());
        assert!(member6.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_err());

        let mut member7 = member2.clone();
        member7.inner_mut().set_object(CompanyID::create().into());
        assert!(member7.can(&CompanyPermission::MemberCreate));
        assert!(member7.access_check(&user_id, &company_id, CompanyPermission::MemberCreate).is_err());
        assert!(member7.access_check(&user_id, &company_id, CompanyPermission::CompanyDelete).is_err());
    }
}

