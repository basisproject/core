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
        lib::basis_model::Model,
        member::{Member, MemberID, MemberClass},
        user::User,
    },
};
use rust_decimal::prelude::*;
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
pub fn update(caller: &User, member: Option<&Member>, mut subject: Company, name: Option<String>, email: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdate)?;
    caller.access_check(Permission::CompanyAdminUpdate)
        .or_else(|_| member.ok_or(Error::InsufficientPrivileges)?.access_check(caller.id(), subject.id(), CompanyPermission::CompanyUpdate))?;
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

/// Delete a private company
pub fn delete(caller: &User, member: Option<&Member>, mut subject: Company, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyDelete)?;
    caller.access_check(Permission::CompanyAdminDelete)
        .or_else(|_| member.ok_or(Error::InsufficientPrivileges)?.access_check(caller.id(), subject.id(), CompanyPermission::CompanyDelete))?;
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
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
        let testfn_inner = |state: &TestState<Company, Company>, member: Option<&Member>| {
            update(state.user(), member, state.company().clone(), Some("Cool Widgets Ltd".into()), None, Some(false), &now2)
        };
        let testfn = |state: &TestState<Company, Company>| {
            testfn_inner(state, Some(state.member()))
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

        let res = testfn_inner(&state, None);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
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
        let testfn_inner = |state: &TestState<Company, Company>, member: Option<&Member>| {
            // note we prefer the model here, and fallback onto the company. the
            // reason is that we want to use the company for our tests until we
            // get to the double-delete test, which operates on the model itself
            // (which is a general assumption but generally works well).
            delete(state.user(), member, state.model.clone().unwrap_or(state.company().clone()), &now2)
        };
        let testfn = |state: &TestState<Company, Company>| {
            testfn_inner(&state, Some(state.member()))
        };
        test::permissions_checks(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let company2 = mods[0].clone().expect_op::<Company>(Op::Delete).unwrap();
        assert_eq!(company2.created(), &now);
        assert_eq!(company2.updated(), &now);
        assert_eq!(company2.deleted(), &Some(now2));

        let res = testfn_inner(&state, None);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state2 = state.clone();
        state2.user_mut().set_id(UserID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        // set the model inot the state, which makes testfn use the model
        // instead of the company for the `subject` param, making our test
        // actually mean something.
        let mut state2 = state.clone();
        state2.model = Some(state.company().clone());
        test::double_deleted_tester(&state2, "company", &testfn);
    }
}

