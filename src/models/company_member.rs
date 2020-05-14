use crate::{
    models::{
        agent::AgentID,
        company::{Permission, Role},
        cost_tag::CostTagLink,
        occupation::OccupationID,
    },
};
use getset::{Getters, CopyGetters};
use om2::{Measure, Unit, NumericUnion};
use serde::{Serialize, Deserialize};
use vf_rs::vf::AgentRelationship;

/// How often we pay workers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PayrollSchedule {
    /// Once every two weeks
    BiWeekly,
    /// Twice a month (on the 15th and the last of the month)
    SemiMonthly,
}

/// Defines compensation types for a member.
///
/// Can account for hourly wages or salary.
#[derive(Clone, Debug, PartialEq, Getters, CopyGetters, Serialize, Deserialize)]
#[getset(get = "pub")]
pub struct Compensation {
    /// A measure of value per time (ie, credits per hour, or credits per year)
    wage: Measure,
    /// Our payroll schedule (biweekly, semimonthly, etc)
    schedule: PayrollSchedule,
    /// If the `period` is not hourly, we can give an estimate for the number of
    /// hours worked per week, which gives us an abillity to estimate our labor
    /// hours (and not just wage payments)
    est_hours_per_week: Option<f64>,
}

impl Compensation {
    /// Create a standard hourly wage, paid biweekly
    pub fn hourly<T>(wage: T) -> Self
        where T: Into<f64>
    {
        Self::hourly_with_schedule(wage, PayrollSchedule::BiWeekly)
    }

    /// Create an hourly wage
    pub fn hourly_with_schedule<T>(wage: T, schedule: PayrollSchedule) -> Self
        where T: Into<f64>
    {
        Self {
            wage: Measure::new(NumericUnion::Double(wage.into()), Unit::Hour),
            schedule: schedule,
            est_hours_per_week: None,
        }
    }

    /// Create a standard yearly salary, paid semimonthly
    pub fn salary<T>(wage: T, est_hours_per_week: f64) -> Self
        where T: Into<f64>
    {
        Self::salary_with_schedule(wage, PayrollSchedule::SemiMonthly, est_hours_per_week)
    }

    pub fn salary_with_schedule<T>(wage: T, schedule: PayrollSchedule, est_hours_per_week: f64) -> Self
        where T: Into<f64>
    {
        Self {
            wage: Measure::new(NumericUnion::Double(wage.into()), Unit::Year),
            schedule: schedule,
            est_hours_per_week: Some(est_hours_per_week),
        }
    }
}

basis_model! {
    /// A member of a company. Links a user to a company, and has other attached
    /// information like compensation, permission roles, etc.
    pub struct CompanyMember {
        agent_relationship: AgentRelationship<AgentID, OccupationID>,
        #[builder(default)]
        roles: Vec<Role>,
        compensation: Compensation,
        #[builder(default)]
        default_cost_tags: Vec<CostTagLink>,
    }
    CompanyMemberID
    CompanyMemberBuilder
}

impl CompanyMember {
    pub fn can(&self, permission: &Permission) -> bool {
        if !self.is_active() {
            return false;
        }
        for role in &self.roles {
            if role.can(&permission) {
                return true;
            }
        }
        false
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
    use super::*;
    use vf_rs::vf::AgentRelationship;

    fn make_member() -> CompanyMember {
        CompanyMember::builder()
            .id("zing")
            .agent_relationship(
                AgentRelationship::builder()
                    .subject(UserID::from("jerry"))
                    .object(CompanyID::from("jerry's widgets ultd"))
                    .relationship("CEO")
                    .build().unwrap()
            )
            .active(true)
            .roles(vec![Role::MemberAdmin])
            .compensation(Compensation::hourly(0.0))
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

