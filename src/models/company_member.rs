use crate::{
    models::{
        account::AccountID,
        company::Permission,
        lib::agent::AgentID,
        occupation::OccupationID,
        process_spec::ProcessSpecID,
    },
};
use getset::Getters;
use om2::{Measure, Unit, NumericUnion};
use rust_decimal::prelude::*;
use serde::{Serialize, Deserialize};
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
        /// A process spec that the member attributes their labor to by default.
        /// This allows some amount of automation when determining what inner
        /// process to count their labor towards. We use a ProcessSpec instead
        /// of a Process here because Process is generally ephemeral. Must be
        /// defined for the member to perform labor.
        process_spec_id: Option<ProcessSpecID>,
    }
    CompanyMemberBuilder
}

impl CompanyMember {
    /// Determines if a member (based on their roles) can perform an action.
    pub fn can(&self, permission: &Permission) -> bool {
        if !self.is_active() {
            return false;
        }
        self.permissions().contains(&Permission::All) ||
            self.permissions().contains(permission)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        models::{
            company::{CompanyID, Permission},
            user::UserID,
        },
        util,
    };
    use rust_decimal_macros::*;
    use super::*;
    use vf_rs::vf;

    fn make_member() -> CompanyMember {
        CompanyMember::builder()
            .id("zing")
            .inner(
                vf::AgentRelationship::builder()
                    .subject(UserID::from("jerry"))
                    .object(CompanyID::from("jerry's widgets ultd"))
                    .relationship("CEO")
                    .build().unwrap()
            )
            .active(true)
            .permissions(vec![Permission::MemberCreate, Permission::MemberSetPermissions, Permission::MemberDelete])
            .compensation(Some(Compensation::new_hourly(dec!(0.0), "12345")))
            .process_spec_id(Some("1234444".into()))
            .created(util::time::now())
            .updated(util::time::now())
            .build().unwrap()
    }

    #[test]
    fn can() {
        let member = make_member();
        assert!(member.can(&Permission::MemberCreate));
        assert!(!member.can(&Permission::CompanyDelete));
    }
}

