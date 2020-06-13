//! Work events record labor in the system. They are how labor costs (both waged
//! and hourly labor) get recorded in the system, and also act as the systemic
//! marker for paying company members. Record labor, get paid.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        event::{Event, EventID, EventProcessState},
        company::{Company, Permission as CompanyPermission},
        company_member::CompanyMember,
        process::Process,
        user::User,
    },
};
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
pub fn work(caller: &User, member: &CompanyMember, company: &Company, id: EventID, worker: CompanyMember, process: Process, wage_cost: Option<Decimal>, begin: DateTime<Utc>, end: DateTime<Utc>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    // if we're recording our own work event, we can just check the regular
    // `Work` permission, otherwise we need admin privs
    if member.id() == worker.id() {
        member.access_check(caller.id(), company.id(), CompanyPermission::Work)?;
    } else {
        member.access_check(caller.id(), company.id(), CompanyPermission::WorkAdmin)?;
    }
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }

    let effort = {
        let milliseconds = end.timestamp_millis() - begin.timestamp_millis();
        let hours = Decimal::from(milliseconds) / Decimal::from(1000 * 60 * 60);
        Measure::new(hours, Unit::Hour)
    };
    let costs = match wage_cost {
        Some(val) => Costs::new_with_labor(worker.inner().relationship().clone(), val),
        None => Costs::new(),
    };
    let process_id = process.id().clone();
    let member_id = worker.id().clone();
    let agreement_id = worker.agreement_id().clone();

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
                .agreed_in(agreement_id)
                .effort_quantity(Some(effort))
                .has_beginning(Some(begin))
                .has_end(Some(end))
                .input_of(Some(process_id))
                .provider(member_id)
                .receiver(company.id().clone())
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(costs))
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let evmods = event.process(state, now)?.modifications();
    let mut mods = Modifications::new();
    mods.push(Op::Create, event);
    for evmod in evmods {
        mods.push_raw(evmod);
    }
    Ok(mods)
}

/*
/// Create a new work event with wage and hour data bundled together. This
/// variant creates a completed/finalized event, and should only be used if the
/// labor is already completed. If you want to create a pending/in-progress work
/// event, use [work_wage_and_hours_begin][begin].
///
/// Most of the time, this is what you want, probably. If compensation is
/// hourly, then this is definitely the right option. If compensation is salary,
/// then this is likely still the right option. The only reason you wouldn't
/// want to use `work_wage_and_hours` is if paying someone a salary but also
/// tracking their hourly labor (which isn't too common).
///
/// [finalize]: fn.work_wage.html
/// [begin]: fn.work_wage_and_hours_begin.html
pub fn work_wage_and_hours(caller: &User, member: &CompanyMember, company: &Company, id: EventID, worker: CompanyMember, process: Process, wage_cost: Decimal, begin: DateTime<Utc>, end: DateTime<Utc>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    // if we're recording our own work event, we can just check the regular
    // `Work` permission, otherwise we need admin privs
    if member.id() == worker.id() {
        member.access_check(caller.id(), company.id(), CompanyPermission::Work)?;
    } else {
        member.access_check(caller.id(), company.id(), CompanyPermission::WorkAdmin)?;
    }
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    let process_id = process.id().clone();
    let state = EventProcessState::builder()
        .input_of(process)
        .provider(worker.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let effort = {
        let milliseconds = end.timestamp_millis() - begin.timestamp_millis();
        let hours = Decimal::from(milliseconds) / Decimal::from(1000 * 60 * 60);
        Some(Measure::new(hours, Unit::Hour))
    };
    let point_in_time = match (begin.as_ref(), end.as_ref()) {
        (None, None) => Some(now.clone()),
        _ => None,
    };
    let costs = if end.is_some() {
        let wage_cost = wage_cost.ok_or(TransactionError::MissingWorkCosts)?;
        Costs::new_with_labor(worker.inner().relationship().clone(), wage_cost)
    } else {
        Costs::new()
    };
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Work)
                .agreed_in(worker.agreement_id().clone())
                .effort_quantity(effort)
                .has_beginning(begin)
                .has_end(end)
                .has_point_in_time(point_in_time)
                .input_of(process_id)
                .provider(worker.id().clone())
                .receiver(company.id().clone())
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(costs))
        .active(true)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let evmods = event.process(state, now)?.modifications();
    let mut mods = Modifications::new();
    mods.push(Op::Create, event);
    for evmod in evmods {
        mods.push_raw(evmod);
    }
    Ok(mods)
}

/// Finalize a pending work event.
///
/// Use this after creating a work event that has a `begin` time but no `end`.
pub fn finalize(caller: &User, member: &CompanyMember, company: &Company, mut subject: Event, worker: CompanyMember, wage_cost: Option<Decimal>, end: DateTime<Utc>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    // the event's `provider` object MUST match the `worker` we're passing in!
    if subject.inner().provider() != &worker.id().clone().into() {
        Err(Error::MismatchedObject("event provider and given worker do not match".into()))?;
    }
    if member.id() != worker.id() {
        // if you are editing a work event you did not create, you must have
        // WorkAdmin permissions
        member.access_check(caller.id(), company.id(), CompanyPermission::WorkAdmin)
    }
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
    }
    if subject.inner().has_point_in_time().is_some() || subject.inner().has_end().is_some() {
        // this will not stand, man.
        Err(TransactionError::EventAlreadyFinalized)?;
    }

    let effort = {
        let begin = subject.inner().has_beginning().as_ref().ok_or(TransactionError::EventMissingBeginning)?;
        let milliseconds = end.timestamp_millis() - begin.timestamp_millis();
        let hours = Decimal::from(milliseconds) / Decimal::from(1000 * 60 * 60);
        Some(Measure::new(hours, Unit::Hour))
    };

    let costs = wage_cost.map(|x| Costs::new_with_labor(worker.inner().relationship().clone(), x));
    if costs.is_some() {
        subject.set_move_costs(costs);
    }
    if effort.is_some() {
        subject.inner_mut().set_effort_quantity(effort);
    }
    subject.set_updated(now.clone());

    let evmods = subject.process(state, now)?.modifications();
    let mut mods = Modifications::new();
    mods.push(Op::Update, subject);
    for evmod in evmods {
        mods.push_raw(evmod);
    }
    Ok(mods)
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::{CompanyID, CompanyType},
            company_member::CompanyMemberID,
            event::{Event, EventID},
            occupation::OccupationID,
            process::ProcessID,
            testutils::{make_user, make_company, make_member, make_process},
            user::UserID,
        },
    };
    use rust_decimal_macros::*;

    #[test]
    fn can_work() {
        let now: DateTime<Utc> = "2018-06-06T00:00:00Z".parse().unwrap();
        let now2: DateTime<Utc> = "2018-06-06T06:52:00Z".parse().unwrap();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &occupation_id, vec![CompanyPermission::Work], &now);
        let worker = member.clone();
        let process = make_process(&ProcessID::create(), company.id(), "make widgets", &Costs::new_with_labor(occupation_id.clone(), dec!(177.5)), &now);

        let mods = work(&user, &member, &company, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), &now2).unwrap().into_modifications();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), member.agreement_id());
        assert_eq!(event.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(event.inner().has_end(), &Some(now2.clone()));
        assert_eq!(event.inner().input_of(), &Some(process.id().clone()));
        assert_eq!(event.inner().provider().clone(), member.id().clone().into());
        assert_eq!(event.inner().receiver().clone(), company.id().clone().into());
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor(occupation_id.clone(), dec!(78.4))));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now2);
        assert_eq!(event.updated(), &now2);
        let mut costs2 = Costs::new();
        costs2.track_labor(occupation_id.clone(), dec!(177.5) + dec!(78.4));
        costs2.track_labor_hours(occupation_id.clone(), dec!(6.8666666666666666666666666666));
        let process2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        assert_eq!(process2.id(), process.id());
        assert_eq!(process2.company_id(), company.id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = work(&user2, &member, &company, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = work(&user, &member2, &company, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = work(&user, &member, &company2, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), &now2);
        assert_eq!(res, Err(Error::CompanyIsDeleted));
    }
}

