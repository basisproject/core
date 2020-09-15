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
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        account::Account,
        company::{Company, CompanyID, Permission as CompanyPermission},
        event::Event,
        lib::basis_model::Model,
        member::{Member, MemberID, MemberClass},
        process::{Process, ProcessID},
        user::User,
    },
};
use rust_decimal::prelude::*;
use std::collections::HashMap;
use std::convert::TryInto;
use vf_rs::vf;

/// An object that is passed into a `company::create()` transaction that
/// describes the founding member of the company.
#[derive(Clone, Debug, PartialEq)]
pub struct Founder {
    /// The ID of the member we're creating
    id: MemberID,
    /// Founder member class
    class: MemberClass,
    /// Whether the founcing member is active on creation or not
    active: bool,
}

impl Founder {
    /// Create a new founder
    pub fn new(founder_id: MemberID, founder_class: MemberClass, active: bool) -> Self {
        Founder {
            id: founder_id,
            class: founder_class,
            active,
        }
    }
}

/// Creates a new company
pub fn create<T: Into<String>>(caller: &User, id: CompanyID, company_name: T, company_email: T, company_active: bool, founder: Founder, now: &DateTime<Utc>) -> Result<Modifications> {
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
        .max_costs(Decimal::zero())
        .total_costs(Costs::new())
        .active(company_active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let Founder { id: founder_id, class: founder_class, active: founder_active } = founder;
    let founder = Member::builder()
        .id(founder_id)
        .inner(
            vf::AgentRelationship::builder()
                .subject(caller.id().clone())
                .object(id.clone())
                .relationship(())
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .class(founder_class)
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
pub fn update(caller: &User, member: &Member, mut subject: Company, name: Option<String>, email: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdate)?;
    member.access_check(caller.id(), subject.id(), CompanyPermission::CompanyUpdate)?;
    if subject.is_deleted() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
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

/// Run payroll on a company.
///
/// Takes a set of `work` events, a hash map of MemberID -> Account pairs, and
/// a hash map of ProcessID -> Process pairs and returns any modifications done
/// to the subject Company, Processes, and Accounts.
pub fn payroll(caller: &User, member: &Member, mut subject: Company, mut accounts: HashMap<MemberID, Account>, mut processes: HashMap<ProcessID, Process>, work_events: &Vec<Event>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyPayroll)?;
    member.access_check(caller.id(), subject.id(), CompanyPermission::Payroll)?;
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    let mut mod_company = false;
    let mut mod_account: HashMap<MemberID, ()> = HashMap::new();
    let mut mod_process: HashMap<ProcessID, ()> = HashMap::new();
    let err_mf = |msg| { || Error::MissingFields(vec![msg]) };
    // loop once, make any mods we want, and track the objects we modified in
    // a sorted btree hash
    for work in work_events {
        let costs = work.move_costs().clone().ok_or_else(err_mf("move_costs".into()))?;
        if costs.is_zero() {
            continue;
        }
        let member_id: MemberID = work.inner().provider().clone().try_into()?;
        let process_id = work.inner().input_of().clone().ok_or_else(err_mf("process.inner.input_of".into()))?;
        let account = accounts.get_mut(&member_id).ok_or_else(err_mf(format!("accounts::{}", member_id.as_str())))?;
        let process = processes.get_mut(&process_id).ok_or_else(err_mf(format!("processes::{}", process_id.as_str())))?;
        subject.increase_costs(costs.clone())?;
        account.adjust_balance(costs.credits().clone())?;
        process.set_costs(process.costs().clone() + costs.clone());
        subject.set_updated(now.clone());
        account.set_updated(now.clone());
        process.set_updated(now.clone());

        mod_company = true;
        mod_account.insert(member_id.clone(), ());
        mod_process.insert(process_id.clone(), ());
    }
    let mut mods = Modifications::new();
    if mod_company {
        mods.push(Op::Update, subject);
    }
    // loop again, pulling out any modified objects, and creating Ops for them.
    // we do a double-loop so that the order of the updates returned is
    // *deterministic* based on the order of the work events passed in.
    for work in work_events {
        let member_id: MemberID = work.inner().provider().clone().try_into()?;
        let process_id = work.inner().input_of().clone().ok_or_else(err_mf("process.inner.input_of".into()))?;
        if mod_account.contains_key(&member_id) {
            if let Some(account) = accounts.remove(&member_id) {
                mods.push(Op::Update, account);
            }
        }
        if mod_process.contains_key(&process_id) {
            if let Some(process) = processes.remove(&process_id) {
                mods.push(Op::Update, process);
            }
        }
    }
    Ok(mods)
}

/// Delete a private company
pub fn delete(caller: &User, member: &Member, mut subject: Company, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyDelete)?;
    member.access_check(caller.id(), subject.id(), CompanyPermission::CompanyDelete)?;
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }
    if subject.total_costs().is_gt_0() {
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
            Op,
            account::AccountID,
            event::EventID,
            lib::agent::Agent,
            member::{MemberClass, MemberWorker},
            occupation::OccupationID,
            user::UserID,
        },
        util::{self, test::{self, *}},
    };

    #[test]
    fn can_create() {
        let id = CompanyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let founder = Founder::new(state.member().id().clone(), MemberClass::Worker(MemberWorker::new(occupation_id.clone(), None)), true);
        state.company = None;
        state.member = None;

        let testfn = |state: &TestState<Company, Company>| {
            // just makin' some widgets, huh? that's cool. hey, I made a widget once,
            // it was actually pretty fun. hey if you're free later maybe we could
            // make some widgets togethe...oh, you're busy? oh ok, that's cool, no
            // problem. hey, maybe next time.
            create(state.user(), id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder.clone(), &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 2);

        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let member = mods[1].clone().expect_op::<Member>(Op::Create).unwrap();
        assert_eq!(company.id(), &id);
        assert_eq!(company.inner().name(), "jerry's widgets");
        assert_eq!(company.email(), "jerry@widgets.expert");
        assert_eq!(company.active(), &true);
        assert_eq!(company.created(), &now);
        assert_eq!(company.updated(), &now);
        assert_eq!(member.id(), &founder.id);
        assert_eq!(member.inner().subject(), &state.user().agent_id());
        assert_eq!(member.inner().object(), &id.clone().into());
        assert_eq!(member.occupation_id(), Some(&occupation_id));
        assert_eq!(member.permissions(), &vec![CompanyPermission::All]);
        assert_eq!(member.active(), &true);
        assert_eq!(member.created(), &now);
        assert_eq!(member.updated(), &now);
    }

    #[test]
    fn can_update() {
        let id = CompanyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let founder = Founder::new(state.member().id().clone(), MemberClass::Worker(MemberWorker::new(occupation_id, None)), true);

        let mods = create(state.user(), id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder.clone(), &now).unwrap().into_vec();
        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let founder = mods[1].clone().expect_op::<Member>(Op::Create).unwrap();
        state.member = Some(founder);
        state.company = Some(company);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Company, Company>| {
            update(state.user(), state.member(), state.company().clone(), Some("Cool Widgets Ltd".into()), None, Some(false), &now2)
        };
        test::permissions_checks(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let company2 = mods[0].clone().expect_op::<Company>(Op::Update).unwrap();
        assert_eq!(company2.id(), state.company().id());
        assert_eq!(company2.inner().name(), "Cool Widgets Ltd");
        assert_eq!(company2.email(), "jerry@widgets.expert");
        assert_eq!(company2.active(), &false);
        assert_eq!(company2.created(), &now);
        assert_eq!(company2.updated(), &now2);

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state3 = state.clone();
        state3.company_mut().set_deleted(Some(now2.clone()));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::ObjectIsInactive("company".into())));
    }

    #[test]
    fn can_payroll() {
        let id = CompanyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let founder = Founder::new(state.member().id().clone(), MemberClass::Worker(MemberWorker::new(occupation_id, None)), true);
        let mods = create(state.user(), id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder.clone(), &now).unwrap().into_vec();
        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let founder = mods[1].clone().expect_op::<Member>(Op::Create).unwrap();
        state.member = Some(founder);
        state.company = Some(company);
        state.company_mut().set_max_costs(num!(2000));

        let mut accounts = HashMap::new();
        let mut processes = HashMap::new();
        let mut work_events = Vec::new();

        let process1 = make_process(&ProcessID::create(), state.company().id(), "herding", &Costs::new(), &now);
        let process2 = make_process(&ProcessID::create(), state.company().id(), "herding", &Costs::new(), &now);
        let process_ids = vec![process1.id().clone(), process2.id().clone()];
        processes.insert(process1.id().clone(), process1);
        processes.insert(process2.id().clone(), process2);
        for i in 0..3 {
            let user = make_user(&UserID::create(), None, &now);
            let member = make_member_worker(&MemberID::create(), user.id(), state.company().id(), &OccupationID::new("bantha herder"), vec![CompanyPermission::Work], &now);
            let account = make_account(&AccountID::create(), user.id(), num!(0), format!("{}'s account", user.id().as_str()), &now);
            let process_id = if i < 1 { process_ids[0].clone() } else { process_ids[1].clone() };
            accounts.insert(member.id().clone(), account);
            for ii in 0..2 {
                let start = "2020-01-01T08:00:00.001-08:00".parse().unwrap();
                let end = "2020-01-01T16:34:00.001-08:00".parse().unwrap();
                let wage = rust_decimal::Decimal::from(10 + (i + 1) + (ii + 1));
                let mods = crate::transactions::event::work::work(&user, &member, state.company(), EventID::create(), member.clone(), processes.get(&process_id).unwrap().clone(), Some(wage), start, end, Some("working".into()), &now).unwrap().into_vec();
                let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
                work_events.push(event);
            }
        }
        let now2 = util::time::now();
        let testfn_inner = |state: &TestState<Company, Company>, accounts, processes| {
            payroll(state.user(), state.member(), state.company().clone(), accounts, processes, &work_events, &now2)
        };
        let testfn = |state: &TestState<Company, Company>| {
            testfn_inner(state, accounts.clone(), processes.clone())
        };
        test::permissions_checks(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 6);
        let company2 = mods[0].clone().expect_op::<Company>(Op::Update).unwrap();
        let account1_2 = mods[1].clone().expect_op::<Account>(Op::Update).unwrap();
        let process1_2 = mods[2].clone().expect_op::<Process>(Op::Update).unwrap();
        let account2_2 = mods[3].clone().expect_op::<Account>(Op::Update).unwrap();
        let process2_2 = mods[4].clone().expect_op::<Process>(Op::Update).unwrap();
        let account3_2 = mods[5].clone().expect_op::<Account>(Op::Update).unwrap();

        assert_eq!(company2.total_costs(), &Costs::new_with_labor("bantha herder", 81));
        assert_eq!(account1_2.balance(), &num!(25));
        assert_eq!(account2_2.balance(), &num!(27));
        assert_eq!(account3_2.balance(), &num!(29));
        assert_eq!(process1_2.costs(), &Costs::new_with_labor("bantha herder", 25));
        assert_eq!(process2_2.costs(), &Costs::new_with_labor("bantha herder", 27 + 29));

        let mut state2 = state.clone();
        state2.company_mut().set_max_costs(num!(81));
        let res = testfn(&state2);
        assert!(res.is_ok());

        let mut state3 = state.clone();
        state3.company_mut().set_max_costs(num!(80));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::MaxCostsReached));

        let mut state4 = state.clone();
        state4.company_mut().set_deleted(Some(now2.clone()));
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));

        let mut accounts2 = accounts.clone();
        let key = accounts2.keys().collect::<Vec<_>>()[0].clone();
        accounts2.remove(&key).unwrap();
        let res = testfn_inner(&state, accounts2, processes.clone());
        assert_eq!(res, Err(Error::MissingFields(vec![format!("accounts::{}", key.as_str())])));

        let mut processes2 = processes.clone();
        let key = processes2.keys().collect::<Vec<_>>()[0].clone();
        processes2.remove(&key).unwrap();
        let res = testfn_inner(&state, accounts, processes2.clone());
        assert_eq!(res, Err(Error::MissingFields(vec![format!("processes::{}", key.as_str())])));
    }

    #[test]
    fn can_delete() {
        let id = CompanyID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let occupation_id = OccupationID::new("CEO THE BEST CEO EVERYONE SAYS SO");
        let founder = Founder::new(state.member().id().clone(), MemberClass::Worker(MemberWorker::new(occupation_id, None)), true);
        let mods = create(state.user(), id.clone(), "jerry's widgets", "jerry@widgets.expert", true, founder, &now).unwrap().into_vec();
        let company = mods[0].clone().expect_op::<Company>(Op::Create).unwrap();
        let member = mods[1].clone().expect_op::<Member>(Op::Create).unwrap();
        state.company = Some(company);
        state.member = Some(member);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Company, Company>| {
            // note we prefer the model here, and fallback onto the company. the
            // reason is that we want to use the company for our tests until we
            // get to the double-delete test, which operates on the model itself
            // (which is a general assumption but generally works well).
            delete(state.user(), state.member(), state.model.clone().unwrap_or(state.company().clone()), &now2)
        };
        test::permissions_checks(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let company2 = mods[0].clone().expect_op::<Company>(Op::Delete).unwrap();
        assert_eq!(company2.created(), &now);
        assert_eq!(company2.updated(), &now);
        assert_eq!(company2.deleted(), &Some(now2));

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state3 = state.clone();
        state3.model = state.company.clone();
        state3.model_mut().set_total_costs(Costs::new_with_labor("zookeeper", num!(50.2)));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::CannotEraseCosts));

        // set the model inot the state, which makes testfn use the model
        // instead of the company for the `subject` param, making our test
        // actually mean something.
        let mut state4 = state.clone();
        state4.model = Some(state.company().clone());
        test::double_deleted_tester(&state4, "company", &testfn);
    }
}

