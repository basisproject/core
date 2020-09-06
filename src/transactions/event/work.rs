//! Work events record labor in the system. They are how labor costs (both waged
//! and hourly labor) get attributed to processes (and as a result resources).
//! They also act as the systemic marker for paying company members. Record
//! labor, get paid.

use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        company::{Company, Permission as CompanyPermission},
        event::{Event, EventID, EventProcessState},
        lib::basis_model::Model,
        member::Member,
        process::Process,
        user::User,
        Modifications, Op,
    },
};
use chrono::{DateTime, Utc};
use om2::{Measure, Unit};
use rust_decimal::prelude::*;
use vf_rs::vf;

/// Create a new work event with the option of passing hourly data, wage data,
/// or both.
///
/// Most of the time you'll want to pass both wage (`wage_cost`) and hourly
/// (`begin`/`end`) data together, unless you're truly tracking them separately.
/// Sometimes you might not know or care to track detailed hourly data (as with
/// salary) but it can be estimated to some extent using data in the worker's
/// Member record.
///
/// Note that this creates a full work event with a defined start and end. This
/// function cannot create pending work events.
pub fn work(
    caller: &User,
    member: &Member,
    company: &Company,
    id: EventID,
    worker: Member,
    process: Process,
    wage_cost: Option<Decimal>,
    begin: DateTime<Utc>,
    end: DateTime<Utc>,
    note: Option<String>,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    // if we're recording our own work event, we can just check the regular
    // `Work` permission, otherwise we need admin privs
    if member.id() == worker.id() {
        member.access_check(caller.id(), company.id(), CompanyPermission::Work)?;
    } else {
        member.access_check(caller.id(), company.id(), CompanyPermission::WorkAdmin)?;
    }
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let effort = {
        let milliseconds = end.timestamp_millis() - begin.timestamp_millis();
        let hours = Decimal::from(milliseconds) / Decimal::from(1000 * 60 * 60);
        Measure::new(hours, Unit::Hour)
    };
    let occupation_id = worker
        .occupation_id()
        .ok_or(Error::MemberMustBeWorker)?
        .clone();
    let costs = match wage_cost {
        Some(val) => Costs::new_with_labor(occupation_id, val),
        None => Costs::new(),
    };
    let process_id = process.id().clone();
    let member_id = worker.id().clone();
    let agreement = worker.agreement().clone();

    let state = EventProcessState::builder()
        .input_of(process)
        .provider(worker)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Work)
                .agreed_in(agreement)
                .effort_quantity(Some(effort))
                .has_beginning(Some(begin))
                .has_end(Some(end))
                .input_of(Some(process_id))
                .note(note)
                .provider(member_id)
                .receiver(company.id().clone())
                .build()
                .map_err(|e| Error::BuilderFailed(e))?,
        )
        .move_costs(Some(costs))
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let evmods = event.process(state, now)?.into_vec();
    let mut mods = Modifications::new();
    mods.push(Op::Create, event);
    for evmod in evmods {
        mods.push_raw(evmod);
    }
    Ok(mods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::CompanyID,
            event::{Event, EventError, EventID},
            lib::agent::Agent,
            member::*,
            process::ProcessID,
        },
        util::test::{self, *},
    };
    use rust_decimal_macros::*;

    #[test]
    fn can_work() {
        let now: DateTime<Utc> = "2018-06-06T00:00:00Z".parse().unwrap();
        let now2: DateTime<Utc> = "2018-06-06T06:52:00Z".parse().unwrap();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Work], &now);
        let occupation_id = state.member().occupation_id().unwrap().clone();
        let worker = state.member().clone();
        let process = make_process(
            &ProcessID::create(),
            state.company().id(),
            "make widgets",
            &Costs::new_with_labor(occupation_id.clone(), dec!(177.5)),
            &now,
        );
        state.model = Some(worker);
        state.model2 = Some(process);

        let testfn = |state: &TestState<Member, Process>| {
            work(
                state.user(),
                state.member(),
                state.company(),
                id.clone(),
                state.model().clone(),
                state.model2().clone(),
                Some(dec!(78.4)),
                now.clone(),
                now2.clone(),
                Some("just doing some work".into()),
                &now2,
            )
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), state.member().agreement());
        assert_eq!(event.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(event.inner().has_end(), &Some(now2.clone()));
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("just doing some work".into()));
        assert_eq!(event.inner().provider().clone(), state.model().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(
            event.move_costs(),
            &Some(Costs::new_with_labor(occupation_id.clone(), dec!(78.4)))
        );
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now2);
        assert_eq!(event.updated(), &now2);

        let mut costs2 = Costs::new();
        costs2.track_labor(occupation_id.clone(), dec!(177.5) + dec!(78.4));
        costs2.track_labor_hours(occupation_id.clone(), dec!(6.8666666666666666666666666666));
        let process2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process2.id(), state.model2().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &costs2);

        // test worker != member
        let mut state2 = state.clone();
        state2.model_mut().set_id(MemberID::create());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        state2
            .member_mut()
            .set_permissions(vec![CompanyPermission::WorkAdmin]);
        let mods = testfn(&state2).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), state2.member().agreement());
        assert_eq!(event.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(event.inner().has_end(), &Some(now2.clone()));
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("just doing some work".into()));
        assert_eq!(event.inner().provider().clone(), state2.model().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(
            event.move_costs(),
            &Some(Costs::new_with_labor(occupation_id.clone(), dec!(78.4)))
        );
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now2);
        assert_eq!(event.updated(), &now2);

        let mut costs2 = Costs::new();
        costs2.track_labor(occupation_id.clone(), dec!(177.5) + dec!(78.4));
        costs2.track_labor_hours(occupation_id.clone(), dec!(6.8666666666666666666666666666));
        let process2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();

        assert_eq!(process2.id(), state.model2().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &costs2);

        // can't work into a process you don't own
        let mut state3 = state.clone();
        state3.model2_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

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
}
