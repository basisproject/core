use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        lib::agent::AgentID,
        process::{Process, ProcessID},
        process_spec::ProcessSpecID,
        user::User,
    },
};
use url::Url;
use vf_rs::vf;

/// Create a new process
pub fn create<T: Into<String>>(caller: &User, member: &CompanyMember, company: &Company, id: ProcessID, spec_id: ProcessSpecID, name: T, note: T, classifications: Vec<Url>, has_beginning: Option<DateTime<Utc>>, has_end: Option<DateTime<Utc>>, in_scope_of: Vec<AgentID>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcesses)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ProcessCreate)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    let model = Process::builder()
        .id(id)
        .inner(
            vf::Process::builder()
                .based_on(Some(spec_id))
                .classified_as(classifications)
                .has_beginning(has_beginning)
                .has_end(has_end)
                .in_scope_of(in_scope_of)
                .name(name)
                .note(Some(note.into()))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .company_id(company.id().clone())
        .costs(Costs::new())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a process
pub fn update(caller: &User, member: &CompanyMember, company: &Company, mut subject: Process, name: Option<String>, note: Option<String>, classifications: Option<Vec<Url>>, finished: Option<bool>, has_beginning: Option<DateTime<Utc>>, has_end: Option<DateTime<Utc>>, in_scope_of: Option<Vec<AgentID>>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcesses)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ProcessUpdate)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if note.is_some() {
        subject.inner_mut().set_note(note);
    }
    if let Some(classifications) = classifications {
        subject.inner_mut().set_classified_as(classifications);
    }
    if finished.is_some() {
        subject.inner_mut().set_finished(finished);
    }
    if has_beginning.is_some() {
        subject.inner_mut().set_has_beginning(has_beginning);
    }
    if has_end.is_some() {
        subject.inner_mut().set_has_end(has_end);
    }
    if let Some(in_scope_of) = in_scope_of {
        subject.inner_mut().set_in_scope_of(in_scope_of);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a process
pub fn delete(caller: &User, member: &CompanyMember, company: &Company, mut subject: Process, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcesses)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ProcessDelete)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    if !subject.costs().is_zero() {
        Err(Error::CannotEraseCosts)?;
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
            lib::agent::Agent,
            occupation::OccupationID,
            process_spec::ProcessSpecID,
            testutils::{make_user, make_company, make_member, make_process_spec},
            user::UserID,
        },
        util,
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = ProcessID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ProcessCreate], &now);
        let spec = make_process_spec(&ProcessSpecID::create(), company.id(), "Make Gazelle Freestyle", true, &now);

        let mods = create(&user, &member, &company, id.clone(), spec.id().clone(), "Gazelle Freestyle Marathon", "tony making me build five of these stupid things", vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()], Some(now.clone()), None, vec![], true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let process = mods[0].clone().expect_op::<Process>(Op::Create).unwrap();
        assert_eq!(process.id(), &id);
        assert_eq!(process.inner().based_on(), &Some(spec.id().clone()));
        assert_eq!(process.inner().classified_as(), &vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()]);
        assert_eq!(process.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(process.inner().has_end(), &None);
        assert_eq!(process.inner().in_scope_of(), &vec![]);
        assert_eq!(process.inner().name(), "Gazelle Freestyle Marathon");
        assert_eq!(process.inner().note(), &Some("tony making me build five of these stupid things".into()));
        assert_eq!(process.company_id(), company.id());
        assert!(process.costs().is_zero());
        assert_eq!(process.active(), &true);
        assert_eq!(process.created(), &now);
        assert_eq!(process.updated(), &now);
        assert_eq!(process.deleted(), &None);

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::ProcessDelete]);
        let res = create(&user, &member2, &company, id.clone(), spec.id().clone(), "Gazelle Freestyle Marathon", "tony making me build five of these stupid things", vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()], Some(now.clone()), None, vec![], true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = create(&user2, &member, &company, id.clone(), spec.id().clone(), "Gazelle Freestyle Marathon", "tony making me build five of these stupid things", vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()], Some(now.clone()), None, vec![], true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now.clone()));
        let res = create(&user, &member, &company2, id.clone(), spec.id().clone(), "Gazelle Freestyle Marathon", "tony making me build five of these stupid things", vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()], Some(now.clone()), None, vec![], true, &now);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = ProcessID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let mut member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ProcessCreate], &now);
        let spec = make_process_spec(&ProcessSpecID::create(), company.id(), "Make Gazelle Freestyle", true, &now);
        let mods = create(&user, &member, &company, id.clone(), spec.id().clone(), "Gazelle Freestyle Marathon", "tony making me build five of these stupid things", vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()], Some(now.clone()), None, vec![], true, &now).unwrap().into_vec();
        let process = mods[0].clone().expect_op::<Process>(Op::Create).unwrap();

        let res = update(&user, &member, &company, process.clone(), Some("Make a GaZeLLe fReeStYlE".into()), None, None, Some(true), None, Some(now.clone()), Some(vec![company.agent_id()]), Some(false), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        member.set_permissions(vec![CompanyPermission::ProcessUpdate]);
        let now2 = util::time::now();
        let mods = update(&user, &member, &company, process.clone(), Some("Make a GaZeLLe fReeStYlE".into()), None, None, Some(true), None, Some(now2.clone()), Some(vec![company.agent_id()]), Some(false), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let process2 = mods[0].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process2.id(), &id);
        assert_eq!(process2.inner().based_on(), &Some(spec.id().clone()));
        assert_eq!(process2.inner().classified_as(), &vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()]);
        assert_eq!(process2.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(process2.inner().has_end(), &Some(now2.clone()));
        assert_eq!(process2.inner().in_scope_of(), &vec![company.agent_id()]);
        assert_eq!(process2.inner().name(), "Make a GaZeLLe fReeStYlE");
        assert_eq!(process2.inner().note(), &Some("tony making me build five of these stupid things".into()));
        assert_eq!(process2.company_id(), company.id());
        assert!(process2.costs().is_zero());
        assert_eq!(process2.active(), &false);
        assert_eq!(process2.created(), &now);
        assert_eq!(process2.updated(), &now2);
        assert_eq!(process2.deleted(), &None);

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = update(&user2, &member, &company, process.clone(), Some("Make a GaZeLLe fReeStYlE".into()), None, None, Some(true), None, Some(now2.clone()), Some(vec![company.agent_id()]), Some(false), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = update(&user, &member, &company2, process.clone(), Some("Make a GaZeLLe fReeStYlE".into()), None, None, Some(true), None, Some(now2.clone()), Some(vec![company.agent_id()]), Some(false), &now2);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = ProcessID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let mut member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::ProcessCreate], &now);
        let spec = make_process_spec(&ProcessSpecID::create(), company.id(), "Make Gazelle Freestyle", true, &now);
        let mods = create(&user, &member, &company, id.clone(), spec.id().clone(), "Gazelle Freestyle Marathon", "tony making me build five of these stupid things", vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()], Some(now.clone()), None, vec![], true, &now).unwrap().into_vec();
        let process = mods[0].clone().expect_op::<Process>(Op::Create).unwrap();

        let now2 = util::time::now();
        let res = delete(&user, &member, &company, process.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        member.set_permissions(vec![CompanyPermission::ProcessDelete]);
        let mods = delete(&user, &member, &company, process.clone(), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let process2 = mods[0].clone().expect_op::<Process>(Op::Delete).unwrap();
        assert_eq!(process2.id(), &id);
        assert_eq!(process2.inner().based_on(), &Some(spec.id().clone()));
        assert_eq!(process2.inner().classified_as(), &vec!["https://www.wikidata.org/wiki/Q1141557".parse().unwrap()]);
        assert_eq!(process2.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(process2.inner().has_end(), &None);
        assert_eq!(process2.inner().in_scope_of(), &vec![]);
        assert_eq!(process2.inner().name(), "Gazelle Freestyle Marathon");
        assert_eq!(process2.inner().note(), &Some("tony making me build five of these stupid things".into()));
        assert_eq!(process2.company_id(), company.id());
        assert!(process2.costs().is_zero());
        assert_eq!(process2.active(), &true);
        assert_eq!(process2.created(), &now);
        assert_eq!(process2.updated(), &now);
        assert_eq!(process2.deleted(), &Some(now2.clone()));

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = delete(&user2, &member, &company, process.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = delete(&user, &member, &company2, process.clone(), &now2);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));
    }
}

