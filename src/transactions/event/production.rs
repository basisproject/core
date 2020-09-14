//! Production is about using, consuming, and producing resources. The `use` and
//! `consume` actions are inputs to the productive process and `produce` is the
//! output the creates a resource.

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
use om2::{Measure, NumericUnion};
use vf_rs::vf;

/// Cite a resource in a process, for instance a design specification.
///
/// This is used for creating a link between a process and a specification of
/// some sort. The resource cited is neither "used" or "consumed" but remains
/// available.
///
/// Note that the resource *can* have a cost, and those costs can be moved by
/// citing. For instance, if it took a year of research to derive a formula,
/// the costs of that research would be imbued in the formula.
pub fn cite(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, process: Process, move_costs: Costs, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Cite)?;
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
                .action(vf::Action::Cite)
                .has_point_in_time(now.clone())
                .input_of(Some(process_id))
                .note(note)
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

/// Consume some or all of a resource, transferring some or all of its costs
/// into a process.
///
/// If you make widgets out of steel, then steel is the resource, and the
/// process would be the fabrication that "consumes" steel (with the output,
/// ie `produce`, of a widget).
pub fn consume<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, process: Process, move_costs: Costs, move_measure: T, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Consume)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let measure = {
        let unit = resource.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(move_measure, unit)
    };

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
                .action(vf::Action::Consume)
                .has_point_in_time(now.clone())
                .input_of(Some(process_id))
                .note(note)
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
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


/// Produce a resource, transferring some or all of the costs of the originating
/// process into the resulting resource.
///
/// For instance, a process might `consume` steel and have a `work` input and
/// then `produce` a widget.
pub fn produce<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, process: Process, resource: Resource, move_costs: Costs, produce_measure: T, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Produce)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }

    let measure = {
        let unit = resource.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(produce_measure, unit)
    };

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
                .action(vf::Action::Produce)
                .has_point_in_time(now.clone())
                .note(note)
                .output_of(Some(process_id))
                .provider(company.id().clone())
                .receiver(company.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
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

/// Use a resource, transferring some or all of its costs into the process it's
/// being used for.
///
/// Use describes the act of using a resource as part of a process without
/// materially changing that resource.
///
/// For instance, you might `use` a stamping maching if making pie tins. You
/// don't consume the stamping machine but rather use it as part of the process
/// (almost in the sense of labor input).
///
/// `Use` can move costs from the resource into the process. For instance, if a
/// 3D printer has a cost of X and has a projected lifetime of 1000 hours of use
/// then using the 3D printer for 3 hours might move `0.003 * X` into the
/// process it's an input to. `Use` is the action that facilitates ammortization
/// of resources over a useful period of time or number of uses.
///
/// If you're trying to express some resource being "used up" (for instance
/// screws being used to build a chair) then you'll probably want `consume`
/// instead of `use`.
pub fn useeee(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, process: Process, move_costs: Costs, effort_quantity: Option<Measure>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Use)?;
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
                .action(vf::Action::Use)
                .effort_quantity(effort_quantity)
                .has_point_in_time(now.clone())
                .input_of(Some(process_id))
                .note(note)
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
    use om2::Unit;

    #[test]
    fn can_cite() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Cite], &now);
        let occupation_id = OccupationID::new("machinist");
        let mut costs = Costs::new();
        costs.track_labor(occupation_id.clone(), num!(42.2));
        costs.track_labor("homemaker", num!(13.6));
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let process = make_process(&ProcessID::create(), state.company().id(), "make widgets", &costs, &now);
        let costs_to_move = resource.costs().clone() * num!(0.02);
        state.model = Some(resource);
        state.model2 = Some(process);

        let testfn = |state: &TestState<Resource, Process>| {
            cite(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), costs_to_move.clone(), Some("memo".into()), &now)
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
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("memo".into()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(process2.id(), state.model2().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // can't consume into a process you don't own
        let mut state2 = state.clone();
        state2.model2_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource can't consume it
        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't consume it
        let mut state4 = state.clone();
        state4.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_consume() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Consume], &now);
        let occupation_id = OccupationID::new("machinist");
        let mut costs = Costs::new();
        costs.track_labor(occupation_id.clone(), num!(42.2));
        costs.track_labor("homemaker", num!(13.6));
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_costs = resource.costs().clone();
        let move_costs = resource_costs.clone() * (num!(8) / num!(15));
        let process = make_process(&ProcessID::create(), state.company().id(), "make widgets", &costs, &now);
        state.model = Some(resource);
        state.model2 = Some(process);

        let testfn = |state: &TestState<Resource, Process>| {
            consume(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), move_costs.clone(), 8, Some("memo".into()), &now)
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
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("memo".into()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.move_costs(), &Some(move_costs.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(process2.id(), state.model2().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &(costs.clone() + move_costs.clone()));

        let mut costs3 = Costs::new();
        costs3.track_labor("homemaker", num!(157) - num!(23));
        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(7), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(7), Unit::One)));
        assert_eq!(resource2.costs(), &(resource_costs.clone() - move_costs.clone()));

        // can't consume into a process you don't own
        let mut state2 = state.clone();
        state2.model2_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource can't consume it
        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't consume it
        let mut state4 = state.clone();
        state4.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_produce() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Produce], &now);
        let occupation_id = OccupationID::new("machinist");
        let mut costs = Costs::new();
        costs.track_labor(occupation_id.clone(), num!(42.2));
        costs.track_labor("homemaker", num!(89.3));
        let process = make_process(&ProcessID::create(), state.company().id(), "make widgets", &costs, &now);
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let costs_to_move = process.costs().clone() * num!(0.5777);
        state.model = Some(process);
        state.model2 = Some(resource);

        let testfn = |state: &TestState<Process, Resource>| {
            produce(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), costs_to_move.clone(), 8, Some("memo".into()), &now)
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
        assert_eq!(event.inner().note(), &Some("memo".into()));
        assert_eq!(event.inner().output_of(), &Some(state.model().id().clone()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(process2.id(), state.model().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource2.id(), state.model2().id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(23), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(23), Unit::One)));
        assert_eq!(resource2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // can't produce from a process you don't own
        let mut state2 = state.clone();
        state2.model_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource can't consume it
        let mut state3 = state.clone();
        state3.model2_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't consume it
        let mut state4 = state.clone();
        state4.model2_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_use() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Use], &now);
        let occupation_id = OccupationID::new("machinist");
        let mut costs = Costs::new();
        costs.track_labor(occupation_id.clone(), num!(42.2));
        costs.track_labor("homemaker", num!(13.6));
        let resource = make_resource(&ResourceID::new("widget"), state.company().id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let process = make_process(&ProcessID::create(), state.company().id(), "make widgets", &costs, &now);
        let costs_to_move = resource.costs().clone() * (num!(8) / num!(15));
        state.model = Some(resource);
        state.model2 = Some(process);

        let testfn = |state: &TestState<Resource, Process>| {
            useeee(state.user(), state.member(), state.company(), id.clone(), state.model().clone(), state.model2().clone(), costs_to_move.clone(), Some(Measure::new(8, Unit::Hour)), Some("memo".into()), &now)
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
        assert_eq!(event.inner().input_of(), &Some(state.model2().id().clone()));
        assert_eq!(event.inner().note(), &Some("memo".into()));
        assert_eq!(event.inner().provider().clone(), state.company().agent_id());
        assert_eq!(event.inner().receiver().clone(), state.company().agent_id());
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(process2.id(), state.model2().id());
        assert_eq!(process2.company_id(), state.company().id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // can't useeee into a process you don't own
        let mut state2 = state.clone();
        state2.model2_mut().set_company_id(CompanyID::new("zing"));
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource can't use it
        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't use it
        let mut state4 = state.clone();
        state4.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }
}

