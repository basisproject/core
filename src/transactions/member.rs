//! Membership is a link between a user and a company, which comes with certain
//! privileges (such as company ownership).
//!
//! See the [company member model.][1]
//!
//! [1]: ../../models/member/index.html

use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        company::{Company, Permission as CompanyPermission},
        lib::{agent::Agent, basis_model::Model},
        member::{Compensation, Member, MemberClass, MemberID},
        occupation::OccupationID,
        user::User,
        Modifications, Op,
    },
};
use chrono::{DateTime, Utc};
use url::Url;
use vf_rs::vf;

/// Create a new member.
pub fn create<T: Agent>(
    caller: &User,
    member: &Member,
    id: MemberID,
    agent_from: T,
    agent_to: Company,
    class: MemberClass,
    permissions: Vec<CompanyPermission>,
    agreement: Option<Url>,
    active: bool,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
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
                .map_err(|e| Error::BuilderFailed(e))?,
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
pub fn update(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: Member,
    occupation_id: Option<OccupationID>,
    agreement: Option<Url>,
    active: Option<bool>,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::MemberUpdate)?;
    if company.id() != &subject.company_id()? {
        Err(Error::InsufficientPrivileges)?;
    }
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

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
pub fn set_permissions(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: Member,
    permissions: Vec<CompanyPermission>,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(
        caller.id(),
        company.id(),
        CompanyPermission::MemberSetPermissions,
    )?;
    if company.id() != &subject.company_id()? {
        Err(Error::InsufficientPrivileges)?;
    }
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    subject.set_permissions(permissions);
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Set a member's compensation.
pub fn set_compensation(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: Member,
    compensation: Compensation,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(
        caller.id(),
        company.id(),
        CompanyPermission::MemberSetCompensation,
    )?;
    if company.id() != &subject.company_id()? {
        Err(Error::InsufficientPrivileges)?;
    }
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

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
pub fn delete(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: Member,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateMembers)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::MemberDelete)?;
    if company.id() != &subject.company_id()? {
        Err(Error::InsufficientPrivileges)?;
    }
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
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
            lib::{agent::Agent, basis_model::Model},
            member::*,
            user::UserID,
        },
        util::{
            self,
            test::{self, *},
        },
    };
    use om2::{Measure, Unit};
    use rust_decimal_macros::*;

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = MemberID::create();
        let mut state = TestState::standard(vec![CompanyPermission::MemberCreate], &now);
        let occupation_id = OccupationID::create();
        let agreement: Url = "https://mydoc.com/work_agreement_1".parse().unwrap();
        let new_user = make_user(&UserID::create(), None, &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));
        state.model = Some(new_user);

        let testfn = |state: &TestState<User, Member>| {
            create(
                state.user(),
                state.member(),
                id.clone(),
                state.model().clone(),
                state.company().clone(),
                new_class.clone(),
                vec![],
                Some(agreement.clone()),
                true,
                &now,
            )
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();
        assert_eq!(member.id(), &id);
        assert_eq!(member.inner().subject(), &state.model().agent_id());
        assert_eq!(member.inner().object(), &state.company().agent_id());
        assert_eq!(member.occupation_id().unwrap(), &occupation_id);
        assert_eq!(member.permissions().len(), 0);
        assert_eq!(member.agreement(), &Some(agreement.clone()));
        assert_eq!(member.active(), &true);
        assert_eq!(member.created(), &now);
        assert_eq!(member.updated(), &now);
        assert_eq!(member.deleted(), &None);
        assert_eq!(member.is_active(), true);
        assert_eq!(member.is_deleted(), false);

        let mut state2 = state.clone();
        state2.model_mut().set_deleted(Some(now.clone()));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::ObjectIsInactive("agent".into())));
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = MemberID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::MemberCreate,
                CompanyPermission::MemberUpdate,
            ],
            &now,
        );
        let occupation_id = OccupationID::create();
        let agreement: Url = "https://mydoc.com/work_agreement_1".parse().unwrap();
        let new_user = make_user(&UserID::create(), None, &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));
        let mods = create(
            state.user(),
            state.member(),
            id.clone(),
            new_user.clone(),
            state.company().clone(),
            new_class.clone(),
            vec![],
            None,
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();
        state.model = Some(member);

        let now2 = util::time::now();
        let new_occupation = OccupationID::create();
        let testfn = |state: &TestState<Member, Member>| {
            update(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                Some(new_occupation.clone()),
                Some(agreement.clone()),
                None,
                &now2,
            )
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member2 = mods[0].clone().expect_op::<Member>(Op::Update).unwrap();
        assert_eq!(state.model().id(), member2.id());
        assert_eq!(state.model().created(), member2.created());
        assert!(state.model().updated() != member2.updated());
        assert_eq!(member2.updated(), &now2);
        assert!(state.model().agreement() != member2.agreement());
        assert_eq!(member2.agreement(), &Some(agreement.clone()));
        assert_eq!(member2.active(), &true);
        assert_eq!(member2.occupation_id().unwrap(), &new_occupation);

        let mut state2 = state.clone();
        state2.member = state.model.clone();
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state3 = state.clone();
        state3.user = Some(new_user.clone());
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state4 = state.clone();
        state4
            .model_mut()
            .set_class(MemberClass::User(MemberUser::new()));
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
        state4
            .model_mut()
            .set_class(MemberClass::Company(MemberCompany::new()));
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
    }

    #[test]
    fn can_set_permissions() {
        let now = util::time::now();
        let id = MemberID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::MemberCreate,
                CompanyPermission::MemberSetPermissions,
            ],
            &now,
        );
        let occupation_id = OccupationID::create();
        let new_user = make_user(&UserID::create(), None, &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));
        let mods = create(
            state.user(),
            state.member(),
            id.clone(),
            new_user.clone(),
            state.company().clone(),
            new_class.clone(),
            vec![],
            None,
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();
        state.model = Some(member);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Member, Member>| {
            set_permissions(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                vec![CompanyPermission::ResourceSpecCreate],
                &now2,
            )
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member2 = mods[0].clone().expect_op::<Member>(Op::Update).unwrap();
        assert_eq!(
            member2.permissions(),
            &vec![CompanyPermission::ResourceSpecCreate]
        );
        assert!(!state.model().can(&CompanyPermission::ResourceSpecCreate));
        assert!(member2.can(&CompanyPermission::ResourceSpecCreate));
        assert_eq!(member2.updated(), &now2);

        let mut state2 = state.clone();
        state2
            .model_mut()
            .set_class(MemberClass::User(MemberUser::new()));
        let res = testfn(&state2);
        assert!(res.is_ok());
        state2
            .model_mut()
            .set_class(MemberClass::Company(MemberCompany::new()));
        let res = testfn(&state2);
        assert!(res.is_ok());
    }

    #[test]
    fn can_set_compensation() {
        let now = util::time::now();
        let id = MemberID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::MemberCreate,
                CompanyPermission::MemberSetCompensation,
            ],
            &now,
        );
        let occupation_id = OccupationID::create();
        let new_user = make_user(&UserID::create(), None, &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));
        let mods = create(
            state.user(),
            state.member(),
            id.clone(),
            new_user.clone(),
            state.company().clone(),
            new_class.clone(),
            vec![],
            None,
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();
        state.model = Some(member);

        let compensation = Compensation::new_hourly(32 as u32, AccountID::create());
        let now2 = util::time::now();
        let testfn = |state: &TestState<Member, Member>| {
            set_compensation(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                compensation.clone(),
                &now2,
            )
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member2 = mods[0].clone().expect_op::<Member>(Op::Update).unwrap();
        assert_eq!(state.model().compensation(), None);
        assert_eq!(
            member2.compensation().unwrap().wage(),
            &Measure::new(dec!(32), Unit::Hour)
        );
        assert_eq!(member2.compensation().unwrap(), &compensation);
        assert_eq!(member2.updated(), &now2);

        let mut state2 = state.clone();
        state2
            .model_mut()
            .set_class(MemberClass::User(MemberUser::new()));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
        state2
            .model_mut()
            .set_class(MemberClass::Company(MemberCompany::new()));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::MemberMustBeWorker));
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = MemberID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::MemberCreate,
                CompanyPermission::MemberDelete,
            ],
            &now,
        );
        let occupation_id = OccupationID::create();
        let new_user = make_user(&UserID::create(), None, &now);
        let new_class = MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None));
        let mods = create(
            state.user(),
            state.member(),
            id.clone(),
            new_user.clone(),
            state.company().clone(),
            new_class.clone(),
            vec![],
            None,
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let member = mods[0].clone().expect_op::<Member>(Op::Create).unwrap();
        state.model = Some(member);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Member, Member>| {
            delete(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                &now2,
            )
        };
        test::standard_transaction_tests(&state, &testfn);
        test::double_deleted_tester(&state, "member", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let member2 = mods[0].clone().expect_op::<Member>(Op::Delete).unwrap();
        assert_eq!(member2.deleted(), &Some(now2.clone()));
        assert!(member2.is_deleted());
        assert!(member2.active());
        assert!(!member2.is_active());
    }
}
