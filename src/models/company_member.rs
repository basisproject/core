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
        lib::agent::AgentID,
        occupation::OccupationID,
        user::UserID,
    },
};
use getset::Getters;
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

basis_model! {
    /// A member of a company. Links a user to a company, and has other attached
    /// information like compensation, permission roles, etc.
    pub struct CompanyMember {
        id: <<CompanyMemberID>>,
        /// Our inner VF relationship (stores both the UserID and CompanyID
        /// under the `AgentID` generic type)
        inner: vf::AgentRelationship<(), AgentID, OccupationID>,
        /// The permissions this member has at their company (additive)
        permissions: Vec<Permission>,
        /// Describes how the member is compensated for their labor. Must be
        /// defined for the member to perform labor.
        compensation: Option<Compensation>,
        /// Agreement under which this membership takes place
        agreement: Option<Url>,
    }
    CompanyMemberBuilder
}

impl CompanyMember {
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

    /// Grab this member's UserID, converted from AgentID
    pub fn user_id(&self) -> Result<UserID> {
        self.inner().subject().clone().try_into()
    }

    /// Grab this member's CompanyID, converted from AgentID
    pub fn company_id(&self) -> Result<CompanyID> {
        self.inner().object().clone().try_into()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        models::{
            company::{CompanyID, Permission as CompanyPermission},
            user::UserID,
            testutils::make_member,
        },
        util,
    };
    use super::*;

    #[test]
    fn can() {
        let now = util::time::now();
        let member = make_member(&CompanyMemberID::create(), &UserID::create(), &CompanyID::create(), &OccupationID::create(), vec![CompanyPermission::MemberCreate, CompanyPermission::MemberUpdate], &now);
        let user_id: UserID = member.user_id().unwrap();
        let company_id: CompanyID = member.company_id().unwrap();
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

