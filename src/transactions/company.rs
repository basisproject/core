//! A company is a generic container that groups people, other companies, and
//! resources together.
//!
//! Companies are often places where economic activity takes place (such as
//! production), but can also group members and resources together in cases like
//! a housing company where the members are in control of the housing resources
//! the company is in stewardship of.
//!
//! See the [company model.][1]
//!
//! [1]: ../../models/company/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, CompanyID, Permission as CompanyPermission},
        company_member::{CompanyMember, CompanyMemberID},
        occupation::OccupationID,
        user::User,
    },
};
use vf_rs::vf;

/// Creates a new private company
pub fn create<T: Into<String>>(caller: &User, id: CompanyID, company_name: T, company_email: T, company_active: bool, founder_id: CompanyMemberID, founder_occupation_id: OccupationID, founder_active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyCreate)?;
    let company = Company::builder()
        .id(id.clone())
        .inner(
            vf::Agent::builder()
                .name(company_name)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .email(company_email)
        .active(company_active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let founder = CompanyMember::builder()
        .id(founder_id)
        .inner(
            vf::AgentRelationship::builder()
                .subject(caller.id().clone())
                .object(id.clone())
                .relationship(founder_occupation_id)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .permissions(vec![CompanyPermission::All])
        .active(founder_active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let mut mods = Modifications::new();
    mods.push(Op::Create, company);
    mods.push(Op::Create, founder);
    Ok(mods)
}

/// Update a private company
pub fn update_private(caller: &User, member: Option<&CompanyMember>, mut subject: Company, name: Option<String>, email: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyAdminUpdate)
        .or_else(|_| member.ok_or(Error::InsufficientPrivileges)?.access_check(caller.id(), subject.id(), CompanyPermission::CompanyUpdate))?;
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(email) = email {
        subject.set_email(email);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a private company
pub fn delete_private(caller: &User, member: Option<&CompanyMember>, mut subject: Company, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyAdminDelete)
        .or_else(|_| member.ok_or(Error::InsufficientPrivileges)?.access_check(caller.id(), subject.id(), CompanyPermission::CompanyDelete))?;
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
            Op,
            lib::agent::Agent,
            user::UserID,
            testutils::make_user,
        },
        util,
    };

    #[test]
    fn can_create() {
        let id = CompanyID::create();
        let founder_id = CompanyMemberID::create();
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let now = util::time::now();
        let user = make_user(&UserID::create(), Some(vec![Role::SuperAdmin]), &now);
        // just makin' some widgets, huh? that's cool. hey, I made a widget once,
        // it was actually pretty fun. hey if you're free later maybe we could
        // make some widgets togethe...oh, you're busy? oh ok, that's cool, no
        // problem. hey, maybe next time.
        let mods = create(&user, id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder_id.clone(), occupation_id.clone(), true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 2);

        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let founder = mods[1].clone().expect_op::<CompanyMember>(Op::Create).unwrap();
        assert_eq!(company.id(), &id);
        assert_eq!(company.inner().name(), "jerry's widgets");
        assert_eq!(company.email(), "jerry@widgets.expert");
        assert_eq!(company.active(), &true);
        assert_eq!(company.created(), &now);
        assert_eq!(company.updated(), &now);
        assert_eq!(founder.id(), &founder_id);
        assert_eq!(founder.inner().subject(), &user.agent_id());
        assert_eq!(founder.inner().object(), &id.clone().into());
        assert_eq!(founder.inner().relationship(), &occupation_id);
        assert_eq!(founder.permissions(), &vec![CompanyPermission::All]);
        assert_eq!(founder.active(), &true);
        assert_eq!(founder.created(), &now);
        assert_eq!(founder.updated(), &now);
    }

    #[test]
    fn can_update_private() {
        let id = CompanyID::create();
        let founder_id = CompanyMemberID::create();
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let now = util::time::now();
        let mut user = make_user(&UserID::create(), Some(vec![Role::SuperAdmin]), &now);
        let mods = create(&user, id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder_id.clone(), occupation_id.clone(), true, &now).unwrap().into_vec();
        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let founder = mods[1].clone().expect_op::<CompanyMember>(Op::Create).unwrap();

        user.set_roles(vec![Role::User]);
        let now2 = util::time::now();
        let mods = update_private(&user, Some(&founder), company.clone(), Some("Cool Widgets Ltd".into()), None, Some(false), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let company2 = mods[0].clone().expect_op::<Company>(Op::Update).unwrap();
        assert_eq!(company2.id(), company.id());
        assert_eq!(company2.inner().name(), "Cool Widgets Ltd");
        assert_eq!(company2.email(), "jerry@widgets.expert");
        assert_eq!(company2.active(), &false);
        assert_eq!(company2.created(), &now);
        assert_eq!(company2.updated(), &now2);

        let res = update_private(&user, None, company.clone(), Some("Cool Widgets Ltd".into()), None, Some(false), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let user = make_user(&UserID::create(), None, &now);
        let res = update_private(&user, Some(&founder), company.clone(), Some("Cool Widgets Ltd".into()), None, Some(false), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = CompanyID::create();
        let founder_id = CompanyMemberID::create();
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let now = util::time::now();
        let mut user = make_user(&UserID::create(), Some(vec![Role::SuperAdmin]), &now);
        let mods = create(&user, id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder_id.clone(), occupation_id.clone(), true, &now).unwrap().into_vec();
        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let founder = mods[1].clone().expect_op::<CompanyMember>(Op::Create).unwrap();

        user.set_roles(vec![Role::User]);
        let now2 = util::time::now();
        let mods = delete_private(&user, Some(&founder), company.clone(), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let company2 = mods[0].clone().expect_op::<Company>(Op::Delete).unwrap();
        assert_eq!(company2.created(), &now);
        assert_eq!(company2.updated(), &now);
        assert_eq!(company2.deleted(), &Some(now2));

        let res = delete_private(&user, None, company.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let user = make_user(&UserID::create(), None, &now);
        let now3 = util::time::now();
        let res = delete_private(&user, Some(&founder), company.clone(), &now3);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

