//! The interface for interacting with `CompanyMember` objects.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, CompanyID, CompanyType, Permission as CompanyPermission},
        company_member::{CompanyMember, CompanyMemberID},
        occupation::OccupationID,
        user::{User, UserID},
    },
};
use vf_rs::vf;

/// Create a new member.
pub fn create<T: Into<String>>(_caller: &User, member: Option<&CompanyMember>, id: CompanyMemberID, user_id: UserID, company_id: CompanyID, occupation_id: OccupationID, permissions: Vec<CompanyPermission>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    member.access_check(&company_id, CompanyPermission::MemberCreate)?;
    let model = CompanyMember::builder()
        .id(id)
        .inner(
            vf::AgentRelationship::builder()
                .subject(user_id.into())
                .object(company_id.into())
                .relationship(occupation_id)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .permissions(permissions)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
            Op,
            user::UserID,
        },
        transactions::tests::make_user,
        util,
    };

    #[test]
    fn can_create() {
        let id = CompanyMemberID::create();
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let now = util::time::now();
        let user = make_user(&UserID::create(), &now, Some(vec![Role::SuperAdmin]));
    }
}

