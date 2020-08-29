//! Membership is a link between a user and a company, which comes with certain
//! privileges (such as company ownership).
//!
//! See the [company member model.][1]
//!
//! [1]: ../../models/member/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, Permission as CompanyPermission},
        member::{Compensation, Member, MemberID, MemberClass},
        lib::{
            agent::Agent,
            basis_model::Model,
        },
        occupation::OccupationID,
        user::User,
    },
};
use url::Url;
use vf_rs::vf;

/// Create a new member.
pub fn create<T: Agent>(caller: &User, member: &Member, id: MemberID, agent_from: T, agent_to: Company, class: MemberClass, permissions: Vec<CompanyPermission>, agreement: Option<Url>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), agent_to.id(), CompanyPermission::MemberCreate)?;
    if !agent_from.is_active() {
        Err(Error::ObjectIsInactive("agent".into()))?;
    }
    if !agent_to.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    let model = Member::builder()
        .id(id)
        .inner(
            vf::AgentRelationship::builder()
                .subject(agent_from.agent_id())
                .object(agent_to.agent_id())
                .relationship(())
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .class(class)
        .permissions(permissions)
        .agreement(agreement)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a member.
pub fn update(caller: &User, member: &Member, mut subject: Member, occupation_id: Option<OccupationID>, agreement: Option<Url>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), &subject.company_id()?, CompanyPermission::MemberUpdate)?;

    if let Some(occupation_id) = occupation_id {
        match subject.class_mut() {
            MemberClass::Worker(worker) => {
                worker.set_occupation(occupation_id);
            }
            _ => Err(Error::MemberMustBeWorker)?,
        }
    }
    if agreement.is_some() {
        subject.set_agreement(agreement);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Set a member's company permissions.
pub fn set_permissions(caller: &User, member: &Member, mut subject: Member, permissions: Vec<CompanyPermission>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), &subject.company_id()?, CompanyPermission::MemberSetPermissions)?;

    subject.set_permissions(permissions);
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Set a member's compensation.
pub fn set_compensation(caller: &User, member: &Member, mut subject: Member, compensation: Compensation, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), &subject.company_id()?, CompanyPermission::MemberSetCompensation)?;

    match subject.class_mut() {
        MemberClass::Worker(worker) => {
            worker.set_compensation(Some(compensation));
        }
        _ => Err(Error::MemberMustBeWorker)?,
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a member.
pub fn delete(caller: &User, member: &Member, mut subject: Member, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), &subject.company_id()?, CompanyPermission::MemberDelete)?;
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("member".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            account::AccountID,
            company::CompanyID,
            member::*,
            lib::{
                agent::Agent,
                basis_model::Model,
            },
            user::UserID,
            testutils::{deleted_company_tester, make_user, make_company, make_member_worker},
        },
        util,
    };
    use rust_decimal_macros::*;
    use om2::{Measure, Unit};

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = MemberID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let occupation_id = OccupationID::create();
        let agreement: Url = "https://mydoc.com/work_agreement_1".parse().unwrap();
        let user = make_user(&UserID::create(), None, &now);
        let new_user = make_user(&UserID::create(), None, &now);
        let existing_member = make_member_worker(&MemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::MemberCreate], &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));

        let mods = create(&user, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], Some(agreement.clone()), true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();
        assert_eq!(member.id(), &id);
        assert_eq!(member.inner().subject(), &new_user.agent_id());
        assert_eq!(member.inner().object(), &company.agent_id());
        assert_eq!(member.occupation_id().unwrap(), &occupation_id);
        assert_eq!(member.permissions().len(), 0);
        assert_eq!(member.agreement(), &Some(agreement.clone()));
        assert_eq!(member.active(), &true);
        assert_eq!(member.created(), &now);
        assert_eq!(member.updated(), &now);
        assert_eq!(member.deleted(), &None);
        assert_eq!(member.is_active(), true);
        assert_eq!(member.is_deleted(), false);

        let user2 = make_user(user.id(), Some(vec![]), &now);
        let res = create(&user2, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], Some(agreement.clone()), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let user3 = make_user(&UserID::create(), None, &now);
        let res = create(&user3, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], Some(agreement.clone()), true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company: Company| {
            create(&user, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], Some(agreement.clone()), true, &now)
        });

        let mut new_user2 = new_user.clone();
        new_user2.set_deleted(Some(now.clone()));
        let res = create(&user, &existing_member, id.clone(), new_user2.clone(), company.clone(), new_class.clone(), vec![], Some(agreement.clone()), true, &now);
        assert_eq!(res, Err(Error::ObjectIsInactive("agent".into())));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = MemberID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let occupation_id = OccupationID::create();
        let agreement: Url = "https://mydoc.com/work_agreement_1".parse().unwrap();
        let user = make_user(&UserID::create(), None, &now);
        let new_user = make_user(&UserID::create(), None, &now);
        let mut existing_member = make_member_worker(&MemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::MemberCreate], &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));

        let mods = create(&user, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], None, true, &now).unwrap().into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();

        // fails because existing_member doesn't have update perm
        let now2 = util::time::now();
        let new_occupation = OccupationID::create();
        let res = update(&user, &existing_member, member.clone(), Some(new_occupation.clone()), None, None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        existing_member.set_permissions(vec![CompanyPermission::MemberUpdate]);
        let mods = update(&user, &existing_member, member.clone(), Some(new_occupation.clone()), Some(agreement.clone()), None, &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let member2 = mods[0].clone().expect_op::<Member>(Op::Update).unwrap();
        assert_eq!(member.id(), member2.id());
        assert_eq!(member.created(), member2.created());
        assert!(member.updated() != member2.updated());
        assert_eq!(member2.updated(), &now2);
        assert!(member.agreement() != member2.agreement());
        assert_eq!(member2.agreement(), &Some(agreement.clone()));
        assert_eq!(member2.active(), &true);
        assert_eq!(member2.occupation_id().unwrap(), &new_occupation);

        let res = update(&user, &member, member.clone(), Some(new_occupation.clone()), Some(agreement.clone()), None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let res = update(&new_user, &existing_member, member.clone(), Some(new_occupation.clone()), Some(agreement.clone()), None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member3 = member2.clone();
        member3.set_class(MemberClass::User(MemberUser::new()));
        let res = update(&user, &existing_member, member3.clone(), Some(new_occupation.clone()), Some(agreement.clone()), None, &now2);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
        member3.set_class(MemberClass::Company(MemberCompany::new()));
        let res = update(&user, &existing_member, member3.clone(), Some(new_occupation.clone()), Some(agreement.clone()), None, &now2);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
    }

    #[test]
    fn can_set_permissions() {
        let now = util::time::now();
        let id = MemberID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let occupation_id = OccupationID::create();
        let user = make_user(&UserID::create(), None, &now);
        let new_user = make_user(&UserID::create(), None, &now);
        let mut existing_member = make_member_worker(&MemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::MemberCreate], &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));

        let mods = create(&user, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], None, true, &now).unwrap().into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();

        // fails because existing_member doesn't have set_perms perm
        let now2 = util::time::now();
        let res = set_permissions(&user, &existing_member, member.clone(), vec![CompanyPermission::ResourceSpecCreate], &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        existing_member.set_permissions(vec![CompanyPermission::MemberSetPermissions]);
        let mods = set_permissions(&user, &existing_member, member.clone(), vec![CompanyPermission::ResourceSpecCreate], &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member2 = mods[0].clone().expect_op::<Member>(Op::Update).unwrap();
        assert_eq!(member2.permissions(), &vec![CompanyPermission::ResourceSpecCreate]);
        assert!(!member.can(&CompanyPermission::ResourceSpecCreate));
        assert!(member2.can(&CompanyPermission::ResourceSpecCreate));
        assert_eq!(member2.updated(), &now2);

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = set_permissions(&user2, &existing_member, member.clone(), vec![CompanyPermission::ResourceSpecCreate], &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member3 = member2.clone();
        member3.set_class(MemberClass::User(MemberUser::new()));
        let res = set_permissions(&user, &existing_member, member3.clone(), vec![CompanyPermission::ResourceSpecCreate], &now2);
        assert!(res.is_ok());
        member3.set_class(MemberClass::Company(MemberCompany::new()));
        let res = set_permissions(&user, &existing_member, member3.clone(), vec![CompanyPermission::ResourceSpecCreate], &now2);
        assert!(res.is_ok());
    }

    #[test]
    fn can_set_compensation() {
        let now = util::time::now();
        let id = MemberID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let occupation_id = OccupationID::create();
        let user = make_user(&UserID::create(), None, &now);
        let new_user = make_user(&UserID::create(), None, &now);
        let mut existing_member = make_member_worker(&MemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::MemberCreate], &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));

        let mods = create(&user, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], None, true, &now).unwrap().into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();

        let compensation = Compensation::new_hourly(32 as u32, AccountID::create());
        let now2 = util::time::now();
        // fails because existing_member doesn't have set_perms perm
        let res = set_compensation(&user, &existing_member, member.clone(), compensation.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        existing_member.set_permissions(vec![CompanyPermission::MemberSetCompensation]);
        let mods = set_compensation(&user, &existing_member, member.clone(), compensation.clone(), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member2 = mods[0].clone().expect_op::<Member>(Op::Update).unwrap();
        assert_eq!(member.compensation(), None);
        assert_eq!(member2.compensation().unwrap().wage(), &Measure::new(dec!(32), Unit::Hour));
        assert_eq!(member2.compensation().unwrap(), &compensation);
        assert_eq!(member2.updated(), &now2);

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = set_compensation(&user2, &existing_member, member.clone(), compensation.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member3 = member2.clone();
        member3.set_class(MemberClass::User(MemberUser::new()));
        let res = set_compensation(&user, &existing_member, member3.clone(), compensation.clone(), &now2);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
        member3.set_class(MemberClass::Company(MemberCompany::new()));
        let res = set_compensation(&user, &existing_member, member3.clone(), compensation.clone(), &now2);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = MemberID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let occupation_id = OccupationID::create();
        let user = make_user(&UserID::create(), None, &now);
        let new_user = make_user(&UserID::create(), None, &now);
        let mut existing_member = make_member_worker(&MemberID::create(), user.id(), company.id(), &OccupationID::create(), vec![CompanyPermission::MemberCreate], &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));

        let mods = create(&user, &existing_member, id.clone(), new_user.clone(), company.clone(), new_class.clone(), vec![], None, true, &now).unwrap().into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();

        let now2 = util::time::now();
        // fails because existing_member doesn't have memberdelete perms
        let res = delete(&user, &existing_member, member.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        existing_member.set_permissions(vec![CompanyPermission::MemberDelete]);
        let mods = delete(&user, &existing_member, member.clone(), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let member2 = mods[0].clone().expect_op::<Member>(Op::Delete).unwrap();
        assert_eq!(member2.deleted(), &Some(now2.clone()));
        assert!(member2.is_deleted());
        assert!(member2.active());
        assert!(!member2.is_active());

        let mut user2 = user.clone();
        user2.set_roles(vec![]);
        let res = delete(&user2, &existing_member, member.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        double_deleted_tester!(member, "member", |subject| delete(&user, &existing_member, subject, &now2));
    }
}

