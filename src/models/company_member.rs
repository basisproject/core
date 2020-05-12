use crate::{
    models::company::{Permission, Role},
    models::cost_tag::CostTagLink,
};
use vf_rs::vf::AgentRelationship;

basis_model! {
    pub struct CompanyMember {
        agent_relationship: AgentRelationship<String, Vec<Role>>,
        occupation: String,
        wage: f64,
        #[builder(default)]
        default_cost_tags: Vec<CostTagLink>,
    }
    CompanyMemberBuilder
}

impl CompanyMember {
    pub fn can(&self, permission: &Permission) -> bool {
        if !self.is_active() {
            return false;
        }
        for role in self.agent_relationship.relationship() {
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
        models::company::{Permission, Role},
        util,
    };
    use super::*;
    use vf_rs::vf::AgentRelationship;

    #[test]
    fn can() {
        let member = CompanyMember::builder()
            .id("zing")
            .agent_relationship(
                AgentRelationship::builder()
                    .subject("jerry")
                    .object("jerry's widgets ultd")
                    .relationship(vec![Role::MemberAdmin])
                    .build().unwrap()
            )
            .active(true)
            .occupation("builder. philanthropist. visionary.")
            .wage(0.0)
            .created(util::time::now())
            .updated(util::time::now())
            .build().unwrap();
        assert!(member.can(&Permission::MemberCreate));
        assert!(!member.can(&Permission::CompanyDelete));
    }
}

