use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        process_spec::{ProcessSpec, ProcessSpecID},
        user::User,
    },
};
use vf_rs::vf;

/// Create a new ProcessSpec
pub fn create<T: Into<String>>(caller: &User, member: &CompanyMember, company: &Company, id: ProcessSpecID, name: T, note: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcessSpecs)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ProcessSpecCreate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    let model = ProcessSpec::builder()
        .id(id)
        .inner(
            vf::ProcessSpecification::builder()
                .name(name)
                .note(Some(note.into()))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .company_id(company.id().clone())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a resource spec
pub fn update(caller: &User, member: &CompanyMember, company: &Company, mut subject: ProcessSpec, name: Option<String>, note: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcessSpecs)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ProcessSpecUpdate)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(note) = note {
        subject.inner_mut().set_note(Some(note));
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a resource spec
pub fn delete(caller: &User, member: &CompanyMember, company: &Company, mut subject: ProcessSpec, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcessSpecs)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ProcessSpecDelete)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::{CompanyID, CompanyType},
            company_member::CompanyMemberID,
            occupation::OccupationID,
            process_spec::{ProcessSpec, ProcessSpecID},
            testutils::{make_user, make_company, make_member},
            user::UserID,
        },
        util,
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = ProcessSpecID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ProcessSpecCreate], &now);

        let mods = create(&user, &member, &company, id.clone(), "SEIZE THE MEANS OF PRODUCTION", "our first process", true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec = mods[0].clone().expect_op::<ProcessSpec>(Op::Create).unwrap();
        assert_eq!(recspec.id(), &id);
        assert_eq!(recspec.inner().name(), "SEIZE THE MEANS OF PRODUCTION");
        assert_eq!(recspec.inner().note(), &Some("our first process".into()));
        assert_eq!(recspec.company_id(), company.id());
        assert_eq!(recspec.active(), &true);
        assert_eq!(recspec.created(), &now);
        assert_eq!(recspec.updated(), &now);
        assert_eq!(recspec.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::ProcessSpecDelete]);
        let res = create(&user, &member2, &company, id.clone(), "SEIZE THE MEANS OF PRODUCTION", "our first process", true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = create(&user2, &member, &company, id.clone(), "SEIZE THE MEANS OF PRODUCTION", "our first process", true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now.clone()));
        let res = create(&user, &member, &company2, id.clone(), "SEIZE THE MEANS OF PRODUCTION", "our first process", true, &now);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = ProcessSpecID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let mut member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ProcessSpecCreate], &now);
        let mods = create(&user, &member, &company, id.clone(), "SEIZE THE MEANS OF PRODUCTION", "our first process", true, &now).unwrap().into_vec();
        let recspec = mods[0].clone().expect_op::<ProcessSpec>(Op::Create).unwrap();

        let res = update(&user, &member, &company, recspec.clone(), Some("best widget".into()), None, Some(false), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        member.set_permissions(vec![CompanyPermission::ProcessSpecUpdate]);
        let now2 = util::time::now();
        let mods = update(&user, &member, &company, recspec.clone(), Some("best widget".into()), None, Some(false), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec2 = mods[0].clone().expect_op::<ProcessSpec>(Op::Update).unwrap();
        assert_eq!(recspec2.id(), &id);
        assert_eq!(recspec2.inner().name(), "best widget");
        assert_eq!(recspec2.inner().note(), &Some("our first process".into()));
        assert_eq!(recspec2.company_id(), company.id());
        assert_eq!(recspec2.active(), &false);
        assert_eq!(recspec2.created(), &now);
        assert_eq!(recspec2.updated(), &now2);
        assert_eq!(recspec2.deleted(), &None);

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = update(&user2, &member, &company, recspec.clone(), Some("best widget".into()), None, Some(false), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = update(&user, &member, &company2, recspec.clone(), Some("best widget".into()), None, Some(false), &now2);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = ProcessSpecID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let mut member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ProcessSpecCreate], &now);
        let mods = create(&user, &member, &company, id.clone(), "SEIZE THE MEANS OF PRODUCTION", "our first process", true, &now).unwrap().into_vec();
        let recspec = mods[0].clone().expect_op::<ProcessSpec>(Op::Create).unwrap();

        let now2 = util::time::now();
        let res = delete(&user, &member, &company, recspec.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        member.set_permissions(vec![CompanyPermission::ProcessSpecDelete]);
        let mods = delete(&user, &member, &company, recspec.clone(), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec2 = mods[0].clone().expect_op::<ProcessSpec>(Op::Delete).unwrap();
        assert_eq!(recspec2.id(), &id);
        assert_eq!(recspec2.inner().name(), "SEIZE THE MEANS OF PRODUCTION");
        assert_eq!(recspec2.company_id(), company.id());
        assert_eq!(recspec2.active(), &true);
        assert_eq!(recspec2.created(), &now);
        assert_eq!(recspec2.updated(), &now);
        assert_eq!(recspec2.deleted(), &Some(now2.clone()));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = delete(&user2, &member, &company, recspec.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = delete(&user, &member, &company2, recspec.clone(), &now2);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }
}

