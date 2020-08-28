//! Work events record labor in the system. They are how labor costs (both waged
//! and hourly labor) get attributed to processes (and as a result resources).
//! They also act as the systemic marker for paying company members. Record
//! labor, get paid.

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
        lib::basis_model::Deletable,
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
pub fn work(caller: &User, member: &CompanyMember, company: &Company, id: EventID, worker: CompanyMember, process: Process, wage_cost: Option<Decimal>, begin: DateTime<Utc>, end: DateTime<Utc>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    // if we're recording our own work event, we can just check the regular
    // `Work` permission, otherwise we need admin privs
    if member.id() == worker.id() {
        member.access_check(caller.id(), company.id(), CompanyPermission::Work)?;
    } else {
        member.access_check(caller.id(), company.id(), CompanyPermission::WorkAdmin)?;
    }
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }

    let effort = {
        let milliseconds = end.timestamp_millis() - begin.timestamp_millis();
        let hours = Decimal::from(milliseconds) / Decimal::from(1000 * 60 * 60);
        Measure::new(hours, Unit::Hour)
    };
    let occupation_id = worker.occupation_id().ok_or(Error::MemberMustBeWorker)?.clone();
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
                .map_err(|e| Error::BuilderFailed(e))?
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
            company_member::CompanyMemberID,
            event::{Event, EventID, EventError},
            lib::agent::Agent,
            occupation::OccupationID,
            process::ProcessID,
            testutils::{make_user, make_company, make_member_worker, make_process},
            user::UserID,
        },
    };
    use rust_decimal_macros::*;

    #[test]
    fn can_work() {
        let now: DateTime<Utc> = "2018-06-06T00:00:00Z".parse().unwrap();
        let now2: DateTime<Utc> = "2018-06-06T06:52:00Z".parse().unwrap();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&CompanyMemberID::create(), user.id(), company.id(), &occupation_id, vec![CompanyPermission::Work], &now);
        let worker = member.clone();
        let process = make_process(&ProcessID::create(), company.id(), "make widgets", &Costs::new_with_labor(occupation_id.clone(), dec!(177.5)), &now);

        let mods = work(&user, &member, &company, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), member.agreement());
        assert_eq!(event.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(event.inner().has_end(), &Some(now2.clone()));
        assert_eq!(event.inner().input_of(), &Some(process.id().clone()));
        assert_eq!(event.inner().note(), &Some("just doing some work".into()));
        assert_eq!(event.inner().provider().clone(), worker.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
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
        let res = work(&user2, &member, &company, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = work(&user, &member2, &company, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = work(&user, &member, &company2, id.clone(), worker.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));

        // test worker != member
        let mut worker2 = worker.clone();
        worker2.set_id(CompanyMemberID::create());
        let res = work(&user, &member, &company, id.clone(), worker2.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![CompanyPermission::WorkAdmin]);
        let mods = work(&user, &member2, &company, id.clone(), worker2.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), None, &now2).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), member2.agreement());
        assert_eq!(event.inner().has_beginning(), &Some(now.clone()));
        assert_eq!(event.inner().has_end(), &Some(now2.clone()));
        assert_eq!(event.inner().input_of(), &Some(process.id().clone()));
        assert_eq!(event.inner().note(), &None);
        assert_eq!(event.inner().provider().clone(), worker2.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
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
        let res = work(&user2, &member2, &company, id.clone(), worker2.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member3 = member2.clone();
        member3.set_permissions(vec![]);
        let res = work(&user, &member3, &company, id.clone(), worker2.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now2.clone()));
        let res = work(&user, &member2, &company2, id.clone(), worker2.clone(), process.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));

        // can't work into a process you don't own
        let mut process3 = process.clone();
        process3.set_company_id(CompanyID::new("zing"));
        let res = work(&user, &member2, &company, id.clone(), worker2.clone(), process3.clone(), Some(dec!(78.4)), now.clone(), now2.clone(), Some("just doing some work".into()), &now2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }
}

