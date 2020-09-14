//! Delivery is about physically moving resources between agents.
//!
//! For instance, if shipping a box of widgets between two companies, a shipping
//! company would use the actions in this module to describe the process and
//! account for the costs along the way.

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
        member::Member,
        lib::basis_model::Model,
        process::Process,
        resource::Resource,
        user::User,
    },
};
use vf_rs::{vf, geo::SpatialThing};

/// Signifies that a delivery has been dropped off at the desired location. Note
/// that custody remains with the deliverer until a `transfer-custody` event is
/// created.
///
/// This operates on a whole resource.
pub fn dropoff(caller: &User, member: &Member, company: &Company, id: EventID, process: Process, resource: Resource, move_costs: Costs, new_location: Option<SpatialThing>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Dropoff)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let process_id = process.id().clone();
    let resource_id = resource.id().clone();

    let state = EventProcessState::builder()
        .output_of(process)
        .resource(resource)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Dropoff)
                .at_location(new_location)
                .has_point_in_time(now.clone())
                .note(note)
                .output_of(Some(process_id))
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(move_costs))
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

/// Signifies that a delivery has been picked up from its origin. Note that
/// custody must have been transfered to the picker-upper previously (via the
/// `transfer-custody` event).
///
/// This operates on a whole resource.
pub fn pickup(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, process: Process, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Pickup)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let process_id = process.id().clone();
    let resource_id = resource.id().clone();

    let state = EventProcessState::builder()
        .input_of(process)
        .resource(resource)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Pickup)
                .has_point_in_time(now.clone())
                .input_of(Some(process_id))
                .note(note)
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(Costs::new()))
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
            event::{EventError, EventID},
            lib::agent::Agent,
            occupation::OccupationID,
            process::ProcessID,
            resource::ResourceID,
        },
        util::{self, test::{self, *}},
    };
    use om2::{Measure, Unit};

    #[test]
    fn can_dropoff() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Dropoff], &now);
        let occupation_id = OccupationID::new("trucker");
        let costs = Costs::new_with_labor(occupation_id.clone(), num!(42.2));
        let process = make_process(&ProcessID::create(), state.company().id(), "deliver widgets", &costs, &now);
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("machinist", 157), &now);
        state.model = Some(process);
        state.model2 = Some(resource);

        let testfn = |state: &TestState<Process, Resource>| {
            dropoff(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), state.model().costs().clone(), Some(state.loc().clone()), Some("memo".into()), &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let process2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        let resource2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().output_of(), &Some(state.model().id().clone()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.move_costs(), &Some(state.model().costs().clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(process2.id(), state.model().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "deliver widgets");
        assert_eq!(process2.costs(), &Costs::new());

        let mut costs2 = Costs::new();
        costs2.track_labor(occupation_id.clone(), num!(42.2));
        costs2.track_labor("machinist", 157);
        assert_eq!(resource2.id(), state.model2().id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(state.company().agent_id()));
        assert_eq!(resource2.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(event.inner().note(), &Some("memo".into()));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.inner().current_location(), &Some(state.loc().clone()));
        assert_eq!(resource2.costs(), &costs2);

        // can't dropoff from a process you don't own
        let mut state2 = state.clone();
        state2.model_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        let mut state3 = state.clone();
        state3.model2_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state3);
        assert!(res.is_ok());

        // a company that doesn't have posession of a resource can't drop it off
        let mut state4 = state.clone();
        state4.model2_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_pickup() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Pickup], &now);
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let process = make_process(&ProcessID::create(), state.company().id(), "make widgets", &Costs::new(), &now);
        state.model = Some(resource);
        state.model2 = Some(process);

        let testfn = |state: &TestState<Resource, Process>| {
            pickup(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), Some("memo".into()), &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("memo".into()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.move_costs(), &Some(Costs::new()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut state2 = state.clone();
        state2.model2_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state3);
        assert!(res.is_ok());

        // a company that doesn't have posession of a resource can't pick it up
        let mut state4 = state.clone();
        state4.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }
}

