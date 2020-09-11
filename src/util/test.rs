use chrono::{DateTime, Utc};
use crate::{
    access::Role,
    costs::Costs,
    error::{Error, Result},
    models::{
        Modifications,

        account::{Account, AccountID, Multisig},
        agreement::{Agreement, AgreementID},
        company::{Company, CompanyID, Permission as CompanyPermission},
        lib::{
            agent::AgentID,
            basis_model::Model,
        },
        member::*,
        occupation::OccupationID,
        process::{Process, ProcessID},
        process_spec::{ProcessSpec, ProcessSpecID},
        resource::{Resource, ResourceID},
        resource_spec::{ResourceSpec, ResourceSpecID},
        user::{User, UserID},
    },
    util,
};
use om2::Measure;
use rust_decimal::prelude::*;
use vf_rs::{vf, geo::SpatialThing};

#[derive(Clone, Debug, PartialEq, getset::Setters, derive_builder::Builder)]
#[builder(pattern = "owned", setter(into))]
#[getset(set = "pub(crate)")]
pub(crate) struct TestState<M1: Model, M2: Model> {
    #[builder(default)]
    pub(crate) user: Option<User>,
    #[builder(default)]
    pub(crate) member: Option<Member>,
    #[builder(default)]
    pub(crate) company: Option<Company>,
    #[builder(default)]
    pub(crate) model: Option<M1>,
    #[builder(default)]
    pub(crate) model2: Option<M2>,
    #[builder(default)]
    pub(crate) loc: Option<SpatialThing>
}

impl<M1: Model, M2: Model> TestState<M1, M2> {
    pub(crate) fn builder() -> TestStateBuilder<M1, M2> {
        TestStateBuilder {
            user: None,
            member: None,
            company: None,
            model: None,
            model2: None,
            loc: None,
        }
    }

    pub(crate) fn standard(permissions: Vec<CompanyPermission>, now: &DateTime<Utc>) -> TestState<M1, M2> {
        let company = make_company(&CompanyID::create(), "larry's chairs", now);
        let user = make_user(&UserID::create(), None, now);
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &OccupationID::create(), permissions, now);
        let loc = SpatialThing::builder()
            .mappable_address(Some("444 Checkmate lane, LOGIC and FACTS, MN, 33133".into()))
            .build().unwrap();
        Self::builder()
            .user(user)
            .member(member)
            .company(company)
            .loc(loc)
            .build().unwrap()
    }

    pub(crate) fn user(&self) -> &User {
        self.user.as_ref().unwrap()
    }

    pub(crate) fn member(&self) -> &Member {
        self.member.as_ref().unwrap()
    }

    pub(crate) fn company(&self) -> &Company {
        self.company.as_ref().unwrap()
    }

    pub(crate) fn model(&self) -> &M1 {
        self.model.as_ref().unwrap()
    }

    pub(crate) fn model2(&self) -> &M2 {
        self.model2.as_ref().unwrap()
    }

    pub(crate) fn loc(&self) -> &SpatialThing {
        self.loc.as_ref().unwrap()
    }

    pub(crate) fn user_mut(&mut self) -> &mut User {
        self.user.as_mut().unwrap()
    }

    pub(crate) fn member_mut(&mut self) -> &mut Member {
        self.member.as_mut().unwrap()
    }

    pub(crate) fn company_mut(&mut self) -> &mut Company {
        self.company.as_mut().unwrap()
    }

    pub(crate) fn model_mut(&mut self) -> &mut M1 {
        self.model.as_mut().unwrap()
    }

    pub(crate) fn model2_mut(&mut self) -> &mut M2 {
        self.model2.as_mut().unwrap()
    }

    #[allow(dead_code)]
    pub(crate) fn loc_mut(&mut self) -> &mut SpatialThing {
        self.loc.as_mut().unwrap()
    }
}

pub(crate) fn deleted_company_tester<M1, M2, F>(state: &TestState<M1, M2>, testfn: &F)
    where M1: Model,
          M2: Model,
          F: Fn(&TestState<M1, M2>) -> Result<Modifications> + Clone,
{
    let now = util::time::now();
    if state.company.is_none() {
        return;
    }

    let mut state1 = state.clone();
    state1.company_mut().set_deleted(None);
    state1.company_mut().set_active(true);
    let res = testfn(&state1);
    if !res.is_ok() {
        panic!("deleted company tester: expected Ok: {:?}", res);
    }

    let mut state2 = state.clone();
    state2.company_mut().set_deleted(Some(now.clone()));
    state2.company_mut().set_active(true);
    let res = testfn(&state2);
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));

    let mut state3 = state.clone();
    state3.company_mut().set_deleted(None);
    state3.company_mut().set_active(false);
    let res = testfn(&state3);
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));

    let mut state4 = state.clone();
    state4.company_mut().set_deleted(Some(now.clone()));
    state4.company_mut().set_active(false);
    let res = testfn(&state4);
    assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));
}

pub(crate) fn permissions_checks<M1, M2, F>(state: &TestState<M1, M2>, testfn: &F)
    where M1: Model,
          M2: Model,
          F: Fn(&TestState<M1, M2>) -> Result<Modifications> + Clone,
{
    // test that a user with no permissions cannot perform this action
    let mut state1 = state.clone();
    state1.user_mut().set_roles(vec![]);
    let res = testfn(&state1);
    assert_eq!(res, Err(Error::InsufficientPrivileges));

    if state.member.is_some() {
        // test that a member with no permissions cannot perform this action
        let mut state2 = state.clone();
        state2.member_mut().set_permissions(vec![]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        // test that when a user's id and member's agent id don't match we cannot
        // perform this action
        let mut state3 = state.clone();
        state3.user_mut().set_id(UserID::new("gee-i-hope-nobody-else-uses-this-exact-id-in-a-test-lol"));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

pub(crate) fn double_deleted_tester<M1, M2, F, S>(state: &TestState<M1, M2>, tystr: S, testfn: &F)
    where M1: Model,
          M2: Model,
          F: Fn(&TestState<M1, M2>) -> Result<Modifications> + Clone,
          S: Into<String>,
{
    let mut state1 = state.clone();
    state1.model_mut().set_deleted(Some(util::time::now()));
    let res = testfn(&state1);
    assert_eq!(res, Err(Error::ObjectIsDeleted(tystr.into())));
}

pub(crate) fn standard_transaction_tests<M1, M2, F>(state: &TestState<M1, M2>, testfn: &F)
    where M1: Model,
          M2: Model,
          F: Fn(&TestState<M1, M2>) -> Result<Modifications> + Clone,
{
    deleted_company_tester(state, testfn);
    permissions_checks(state, testfn);
}

pub fn make_account<T: Into<String>, D: Into<Decimal>>(id: &AccountID, user_id: &UserID, balance: D, name: T, now: &DateTime<Utc>) -> Account {
    Account::builder()
        .id(id.clone())
        .user_ids(vec![user_id.clone()])
        .multisig(vec![Multisig::new(1)])
        .name(name.into())
        .description("THIS IS MY ACCOUNT. IF YOU SHOUT A STATEMENT IT MAKES IT MORE TRUE. ASK RON.")
        .balance(balance.into())
        .ubi(false)
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_agreement<T: Into<String>>(id: &AgreementID, participants: &Vec<AgentID>, name: T, note: T, now: &DateTime<Utc>) -> Agreement {
    Agreement::builder()
        .id(id.clone())
        .inner(
            vf::Agreement::builder()
                .created(now.clone())
                .name(Some(name.into()))
                .note(Some(note.into()))
                .build().unwrap()
        )
        .participants(participants.clone())
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_company<T: Into<String>>(id: &CompanyID, name: T, now: &DateTime<Utc>) -> Company {
    Company::builder()
        .id(id.clone())
        .inner(vf::Agent::builder().name(name).build().unwrap())
        .email("jerry@widgets.biz")
        .active(true)
        .max_costs(Decimal::zero())
        .total_costs(Costs::new())
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_member_worker(member_id: &MemberID, user_id: &UserID, company_id: &CompanyID, occupation_id: &OccupationID, permissions: Vec<CompanyPermission>, now: &DateTime<Utc>) -> Member {
    Member::builder()
        .id(member_id.clone())
        .inner(
            vf::AgentRelationship::builder()
                .subject(user_id.clone())
                .object(company_id.clone())
                .relationship(())
                .build().unwrap()
        )
        .class(MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None)))
        .permissions(permissions)
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_process<T: Into<String>>(id: &ProcessID, company_id: &CompanyID, name: T, costs: &Costs, now: &DateTime<Utc>) -> Process {
    Process::builder()
        .id(id.clone())
        .inner(vf::Process::builder().name(name).build().unwrap())
        .company_id(company_id.clone())
        .costs(costs.clone())
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_process_spec<T: Into<String>>(id: &ProcessSpecID, company_id: &CompanyID, name: T, active: bool, now: &DateTime<Utc>) -> ProcessSpec {
    ProcessSpec::builder()
        .id(id.clone())
        .inner(
            vf::ProcessSpecification::builder()
                .name(name)
                .build().unwrap()
        )
        .company_id(company_id.clone())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_resource(id: &ResourceID, company_id: &CompanyID, quantity: &Measure, costs: &Costs, now: &DateTime<Utc>) -> Resource {
    Resource::builder()
        .id(id.clone())
        .inner(
            vf::EconomicResource::builder()
                .accounting_quantity(Some(quantity.clone()))
                .onhand_quantity(Some(quantity.clone()))
                .primary_accountable(Some(company_id.clone().into()))
                .conforms_to("6969")
                .build().unwrap()
        )
        .in_custody_of(company_id.clone())
        .costs(costs.clone())
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_resource_spec<T: Into<String>>(id: &ResourceSpecID, company_id: &CompanyID, name: T, now: &DateTime<Utc>) -> ResourceSpec {
    ResourceSpec::builder()
        .id(id.clone())
        .inner(
            vf::ResourceSpecification::builder()
                .name(name)
                .build().unwrap()
        )
        .company_id(company_id.clone())
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}

pub fn make_user(user_id: &UserID, roles: Option<Vec<Role>>, now: &DateTime<Utc>) -> User {
    User::builder()
        .id(user_id.clone())
        .roles(roles.unwrap_or(vec![Role::User]))
        .email("surely@hotmail.com")   // don't call me shirley
        .name("buzzin' frog")
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build().unwrap()
}
