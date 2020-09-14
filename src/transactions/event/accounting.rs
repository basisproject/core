//! Accounting allows adjustments of costs or resource measurements internally.
//!
//! For instance, if a company wanted to raise or lower some quantity of a
//! resource or move costs between processes or resources, this is where they
//! could do it.

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        event::{Event, EventID, EventProcessState, MoveType},
        company::{Company, Permission as CompanyPermission},
        member::Member,
        lib::basis_model::Model,
        process::Process,
        resource::Resource,
        user::User,
    },
    transactions::event::ResourceMover,
};
use om2::{Measure, NumericUnion};
use vf_rs::{vf, geo::SpatialThing};

/// Lower the quantity (both accounting and obhand) or a resource by a fixed
/// amount.
pub fn lower<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, resource_measure: T, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Lower)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let measure = {
        let unit = resource.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(resource_measure, unit)
    };
    let resource_id = resource.id().clone();

    let state = EventProcessState::builder()
        .resource(resource)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Lower)
                .has_point_in_time(now.clone())
                .note(note.map(|x| x.into()))
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
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

/// Move costs between internal processes.
///
/// This can be useful to send costs from one process to another, for instance
/// if a process has an excess of costs that should be moved somewhere else.
pub fn move_costs(caller: &User, member: &Member, company: &Company, id: EventID, process_from: Process, process_to: Process, move_costs: Costs, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::MoveCosts)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let process_from_id = process_from.id().clone();
    let process_to_id = process_to.id().clone();

    let state = EventProcessState::builder()
        .output_of(process_from)
        .input_of(process_to)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Move)
                .has_point_in_time(now.clone())
                .input_of(Some(process_to_id))
                .note(note)
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .output_of(Some(process_from_id))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(move_costs))
        .move_type(Some(MoveType::ProcessCosts))
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

/// Move a resource internally. This can split a resource into two, or move one
/// resource entirely into another one.
pub fn move_resource<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs: Costs, resource_measure: T, new_location: Option<SpatialThing>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::MoveResource)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let measure = {
        let unit = resource_from.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(resource_measure, unit)
    };
    let resource_from_id = resource_from.id().clone();

    let mut statebuilder = EventProcessState::builder()
        .resource(resource_from);
    let resource_to_id = match resource_to {
        ResourceMover::Create(resource_id) => resource_id,
        ResourceMover::Update(resource) => {
            let resource_id = resource.id().clone();
            statebuilder = statebuilder.to_resource(resource);
            resource_id
        }
    };

    let state = statebuilder
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Move)
                .at_location(new_location)
                .has_point_in_time(now.clone())
                .note(note)
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_from_id))
                .resource_quantity(Some(measure))
                .to_resource_inventoried_as(Some(resource_to_id))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .move_costs(Some(move_costs))
        .move_type(Some(MoveType::Resource))
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

/// Raise the quantity (both accounting and onhand) or a resource by a fixed
/// amount.
pub fn raise<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, resource_measure: T, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Raise)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let measure = {
        let unit = resource.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(resource_measure, unit)
    };
    let resource_id = resource.id().clone();

    let state = EventProcessState::builder()
        .resource(resource)
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    let event = Event::builder()
        .id(id)
        .inner(
            vf::EconomicEvent::builder()
                .action(vf::Action::Raise)
                .has_point_in_time(now.clone())
                .note(note)
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
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
            lib::agent::Agent,
            company::CompanyID,
            event::{EventID, EventError},
            occupation::OccupationID,
            process::{Process, ProcessID},
            resource::ResourceID,
        },
        util::{self, test::{self, *}},
    };
    use om2::Unit;
    use rust_decimal_macros::*;

    #[test]
    fn can_lower() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Lower], &now);
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        state.model = Some(resource);

        let testfn = |state: &TestState<Resource, Resource>| {
            lower(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), 8, Some("a note".into()), &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("a note".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &None);
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(7), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(7), Unit::One)));
        assert_eq!(resource2.costs(), state.model().costs());

        // a company that doesn't own a resource can't lower it
        let mut state2 = state.clone();
        state2.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have possession of a resource can't lower it
        let mut state3 = state.clone();
        state3.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_move_costs() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::MoveCosts], &now);
        let occupation_id = OccupationID::new("lawyer");
        let process_from = make_process(&ProcessID::create(), state.company().id(), "various lawyerings", &Costs::new_with_labor(occupation_id.clone(), dec!(177.25)), &now);
        let process_to = make_process(&ProcessID::create(), state.company().id(), "overflow labor", &Costs::new_with_labor(occupation_id.clone(), dec!(804)), &now);
        let costs_to_move = process_from.costs().clone() * dec!(0.45);
        state.model = Some(process_from);
        state.model2 = Some(process_to);

        let testfn = |state: &TestState<Process, Process>| {
            move_costs(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), costs_to_move.clone(), Some("my note".into()), &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let process_from2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        let process_to2 = mods[2].clone().expect_op::<Process>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("my note".into()));
        assert_eq!(event.inner().output_of(), &Some(state.model().id().clone()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.inner().resource_quantity(), &None);
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("lawyer", dec!(177.25) - dec!(100));
        assert_eq!(process_from2.id(), state.model().id());
        assert_eq!(process_from2.company_id(), state.company().id());
        assert_eq!(process_from2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(process_to2.id(), state.model2().id());
        assert_eq!(process_to2.company_id(), state.company().id());
        assert_eq!(process_to2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // can't move costs from a process you don't own
        let mut state2 = state.clone();
        state2.model_mut().set_company_id(CompanyID::new("zing").into());
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // can't move costs into a process you don't own
        let mut state3 = state.clone();
        state3.model2_mut().set_company_id(CompanyID::new("zing").into());
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn can_move_resource() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::MoveResource], &now);
        let resource = make_resource(&ResourceID::new("plank"), state.company().id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), state.company().id(), &Measure::new(dec!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);
        let costs_to_move = resource.costs().clone() * (dec!(8) / dec!(15));
        state.model = Some(resource);
        state.model2 = Some(resource_to);

        let testfn_inner = |state: &TestState<Resource, Resource>, mover: ResourceMover| {
            move_resource(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), mover, costs_to_move.clone(), 8, Some(state.loc().clone()), Some("lol".into()), &now)
        };
        let testfn_update = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, ResourceMover::Update(state.model2().clone()))
        };
        let testfn_create = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, ResourceMover::Create(state.model2().id().clone()))
        };
        test::standard_transaction_tests(&state, &testfn_update);
        test::standard_transaction_tests(&state, &testfn_create);

        // test ResourceMover::Update()
        let mods = testfn_update(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource_from2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().at_location(), &Some(state.loc().clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("lol".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource_from2.id(), state.model().id());
        assert_eq!(resource_from2.inner().primary_accountable(), &Some(state.company().agent_id()));
        assert_eq!(resource_from2.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from2.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from2.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource_from2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource_to2.id(), state.model2().id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(state.company().agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource_to2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // test ResourceMover::Create()
        let mods = testfn_create(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource_from3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("lol".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource_from3.id(), state.model().id());
        assert_eq!(resource_from3.inner().primary_accountable(), &Some(state.company().agent_id()));
        assert_eq!(resource_from3.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from3.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from3.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource_from3.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource_created.id(), state.model2().id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(state.company().agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource_created.costs(), &(costs_to_move.clone()));

        // can't move into a resource you don't own
        let mut state2 = state.clone();
        state2.model2_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = testfn_update(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't own a resource can't move it OBVIOUSLY
        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn_update(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));
        let res = testfn_create(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't move it
        let mut state4 = state.clone();
        state4.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn_update(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
        let res = testfn_create(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_raise() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Raise], &now);
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        state.model = Some(resource);

        let testfn = |state: &TestState<Resource, Resource>| {
            raise(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), 8, Some("toot".into()), &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("toot".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &None);
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(23), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(23), Unit::One)));
        assert_eq!(resource2.costs(), state.model().costs());

        // a company that doesn't own a resource can't raise it
        let mut state2 = state.clone();
        state2.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have possession of a resource can't raise it
        let mut state3 = state.clone();
        state3.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }
}

