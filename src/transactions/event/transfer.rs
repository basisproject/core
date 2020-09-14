//! The transfer transactions are geared towards passing ownership, custody, or
//! both from one agent to another.
//!
//! If you're looking for internal transfers, see the [accounting transactions.][1]
//!
//! [1]: ../accounting/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        agreement::Agreement,
        event::{Event, EventID, EventProcessState},
        lib::{
            agent::Agent,
            basis_model::Model,
        },
        company::{Company, Permission as CompanyPermission},
        member::Member,
        resource::Resource,
        user::User,
    },
    transactions::event::ResourceMover,
    util::number::Ratio,
};
use om2::{Measure, NumericUnion};
use url::Url;
use vf_rs::vf;

/// Transfer a resource (custody and ownership) from one company to another,
/// moving a set of costs with it.
pub fn transfer<T: Into<NumericUnion>>(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs_ratio: Ratio, move_measure: T, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company_from.id(), CompanyPermission::Transfer)?;
    if !company_from.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !company_to.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !agreement.has_participant(&company_from.agent_id()) || !agreement.has_participant(&company_from.agent_id()) {
        // can't create an event for an agreement you are not party to
        Err(Error::InsufficientPrivileges)?;
    }
    let measure = {
        let unit = resource_from.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(move_measure, unit)
    };

    let resource_id = resource_from.id().clone();
    let move_costs = resource_from.costs().clone() * move_costs_ratio;

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
                .action(vf::Action::Transfer)
                .agreed_in(agreed_in)
                .has_point_in_time(now.clone())
                .note(note)
                .provider(company_from.id().clone())
                .realization_of(Some(agreement.id().clone()))
                .receiver(company_to.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
                .to_resource_inventoried_as(Some(resource_to_id))
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

/// Transfer ownership (but not custody) of a resource from one company to
/// another, moving a set of costs with it.
pub fn transfer_all_rights<T: Into<NumericUnion>>(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs_ratio: Ratio, move_measure: T, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company_from.id(), CompanyPermission::TransferAllRights)?;
    if !company_from.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !company_to.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !agreement.has_participant(&company_from.agent_id()) || !agreement.has_participant(&company_from.agent_id()) {
        // can't create an event for an agreement you are not party to
        Err(Error::InsufficientPrivileges)?;
    }
    let measure = {
        let unit = resource_from.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(move_measure, unit)
    };

    let resource_id = resource_from.id().clone();
    let move_costs = resource_from.costs().clone() * move_costs_ratio;

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
                .action(vf::Action::TransferAllRights)
                .agreed_in(agreed_in)
                .has_point_in_time(now.clone())
                .note(note)
                .provider(company_from.id().clone())
                .realization_of(Some(agreement.id().clone()))
                .receiver(company_to.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
                .to_resource_inventoried_as(Some(resource_to_id))
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

/// Transfer custody (but not ownership) of a resource from one company to
/// another, moving a set of costs with it.
pub fn transfer_custody<T: Into<NumericUnion>>(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs_ratio: Ratio, move_measure: T, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company_from.id(), CompanyPermission::TransferCustody)?;
    if !company_from.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !company_to.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if !agreement.has_participant(&company_from.agent_id()) || !agreement.has_participant(&company_from.agent_id()) {
        // can't create an event for an agreement you are not party to
        Err(Error::InsufficientPrivileges)?;
    }
    let measure = {
        let unit = resource_from.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(move_measure, unit)
    };

    let resource_id = resource_from.id().clone();
    let move_costs = resource_from.costs().clone() * move_costs_ratio;

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
                .action(vf::Action::TransferCustody)
                .agreed_in(agreed_in)
                .has_point_in_time(now.clone())
                .note(note)
                .provider(company_from.id().clone())
                .realization_of(Some(agreement.id().clone()))
                .receiver(company_to.id().clone())
                .resource_inventoried_as(Some(resource_id))
                .resource_quantity(Some(measure))
                .to_resource_inventoried_as(Some(resource_to_id))
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
        costs::Costs,
        models::{
            agreement::AgreementID,
            company::CompanyID,
            event::{EventID, EventError},
            lib::agent::Agent,
            resource::ResourceID,
        },
        util::{self, test::{self, *}},
    };
    use om2::Unit;

    #[test]
    fn can_transfer() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::Transfer], &now);
        let company_from = state.company().clone();
        let company_to = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 1234", "gotta get some planks", &now);
        let agreed_in: Url = "https://legalzoom.com/standard-boilerplate-hereto-notwithstanding-each-of-them-damage-to-the-hood-ornament-alone".parse().unwrap();
        let resource_from = make_resource(&ResourceID::new("plank"), company_from.id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company_to.id(), &Measure::new(num!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);
        let move_costs_ratio = Ratio::new(num!(0.67777777)).unwrap();
        let costs_to_move = resource_from.costs().clone() * move_costs_ratio.clone();
        state.model = Some(resource_from);
        state.model2 = Some(resource_to);

        let testfn_inner = |state: &TestState<Resource, Resource>, company_from: &Company, company_to: &Company, agreement: &Agreement, resource_to: ResourceMover| {
            transfer(state.user(), state.member(), &company_from, &company_to, &agreement, id.clone(), state.model().clone(), resource_to, move_costs_ratio.clone(), 8, Some(agreed_in.clone()), Some("giving jinkey some post-capitalist planks".into()), &now)
        };
        let testfn_update = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, state.company(), &company_to, &agreement, ResourceMover::Update(state.model2().clone()))
        };
        let testfn_create = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, state.company(), &company_to, &agreement, ResourceMover::Create(state.model2().id().clone()))
        };
        let testfn_update_to = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, &company_from, state.company(), &agreement, ResourceMover::Update(state.model2().clone()))
        };
        test::standard_transaction_tests(&state, &testfn_update);
        test::standard_transaction_tests(&state, &testfn_create);

        // test ResourceMover::Update()
        let mods = testfn_update(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("giving jinkey some post-capitalist planks".into()));
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource2.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource_to2.id(), state.model2().id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company_to.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(num!(8) + num!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(num!(8) + num!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company_to.agent_id());
        assert_eq!(resource_to2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // test ResourceMover::Create()
        let mods = testfn_create(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("giving jinkey some post-capitalist planks".into()));
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource3.id(), state.model().id());
        assert_eq!(resource3.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource3.inner().onhand_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource3.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource3.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource_created.id(), state.model2().id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company_to.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(num!(8), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(num!(8), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company_to.agent_id());
        assert_eq!(resource_created.costs(), &costs_to_move);

        // can't transfer into a resource you don't own
        let mut state2 = state.clone();
        state2.model2_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = testfn_update(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't own a resource can't transfer it OBVIOUSLY
        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn_update(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));
        let res = testfn_create(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't transfer it
        let mut state4 = state.clone();
        state4.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn_update(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
        let res = testfn_create(&state4);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company_to.agent_id()]);
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2, ResourceMover::Update(state.model2().clone()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2, ResourceMover::Create(state.model2().id().clone()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state5 = state.clone();
        state5.company = Some(company_to.clone());
        test::deleted_company_tester(&state5, &testfn_update_to);
    }

    #[test]
    fn can_transfer_all_rights() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::TransferAllRights], &now);
        let company_from = state.company().clone();
        let company_to = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 1234", "gotta get some planks", &now);
        let agreed_in: Url = "https://legalzoom.com/is-it-too-much-to-ask-for-todays-pedestrian-to-wear-at-least-one-piece-of-reflective-clothing".parse().unwrap();
        let resource_from = make_resource(&ResourceID::new("plank"), company_from.id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company_to.id(), &Measure::new(num!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);
        let move_costs_ratio = Ratio::new(num!(0.1555232)).unwrap();
        let costs_to_move = resource_from.costs().clone() * move_costs_ratio.clone();
        state.model = Some(resource_from);
        state.model2 = Some(resource_to);

        let testfn_inner = |state: &TestState<Resource, Resource>, company_from: &Company, company_to: &Company, agreement: &Agreement, resource_to: ResourceMover| {
            transfer_all_rights(state.user(), state.member(), &company_from, &company_to, &agreement, id.clone(), state.model().clone(), resource_to, move_costs_ratio.clone(), 8, Some(agreed_in.clone()), Some("note blah blah".into()), &now)
        };
        let testfn_update = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, state.company(), &company_to, &agreement, ResourceMover::Update(state.model2().clone()))
        };
        let testfn_create = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, state.company(), &company_to, &agreement, ResourceMover::Create(state.model2().id().clone()))
        };
        let testfn_update_to = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, &company_from, state.company(), &agreement, ResourceMover::Update(state.model2().clone()))
        };
        test::standard_transaction_tests(&state, &testfn_update);
        test::standard_transaction_tests(&state, &testfn_create);

        // test ResourceMover::Update()
        let mods = testfn_update(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("note blah blah".into()));
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource_to2.id(), state.model2().id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company_to.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(num!(8) + num!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(num!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company_to.agent_id());
        assert_eq!(resource_to2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // test ResourceMover::Create()
        let mods = testfn_create(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("note blah blah".into()));
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", num!(157) - num!(23));
        assert_eq!(resource3.id(), state.model().id());
        assert_eq!(resource3.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource3.inner().onhand_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource3.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource3.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", num!(23));
        assert_eq!(resource_created.id(), state.model2().id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company_to.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(num!(8), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(num!(0), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource_created.costs(), &costs_to_move);

        // can't transfer into a resource you don't own
        let mut state2 = state.clone();
        state2.model2_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = testfn_update(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't own a resource can't transfer it OBVIOUSLY
        let mut state3 = state.clone();
        state3.model_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = testfn_update(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));
        let res = testfn_create(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company_to.agent_id()]);
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2, ResourceMover::Update(state.model2().clone()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2, ResourceMover::Create(state.model2().id().clone()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state5 = state.clone();
        state5.company = Some(company_to.clone());
        test::deleted_company_tester(&state5, &testfn_update_to);
    }

    #[test]
    fn can_transfer_custody() {
        let now = util::time::now();
        let id = EventID::create();
        let mut state = TestState::standard(vec![CompanyPermission::TransferCustody], &now);
        let company_from = state.company().clone();
        let company_to = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company_from.agent_id(), company_to.agent_id()], "order 1234", "gotta get some planks", &now);
        let agreed_in: Url = "https://legaldoom.com/trade-secrets-trade-secrets".parse().unwrap();
        let resource_from = make_resource(&ResourceID::new("plank"), company_from.id(), &Measure::new(num!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company_to.id(), &Measure::new(num!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);
        let move_costs_ratio = Ratio::new(num!(8) / num!(15)).unwrap();
        let costs_to_move = resource_from.costs().clone() * move_costs_ratio.clone();
        state.model = Some(resource_from);
        state.model2 = Some(resource_to);

        let testfn_inner = |state: &TestState<Resource, Resource>, company_from: &Company, company_to: &Company, agreement: &Agreement, resource_to: ResourceMover| {
            transfer_custody(state.user(), state.member(), &company_from, &company_to, &agreement, id.clone(), state.model().clone(), resource_to, move_costs_ratio.clone(), 8, Some(agreed_in.clone()), Some("nomnomnom".into()), &now)
        };
        let testfn_update = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, state.company(), &company_to, &agreement, ResourceMover::Update(state.model2().clone()))
        };
        let testfn_create = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, state.company(), &company_to, &agreement, ResourceMover::Create(state.model2().id().clone()))
        };
        let testfn_update_to = |state: &TestState<Resource, Resource>| {
            testfn_inner(state, &company_from, state.company(), &agreement, ResourceMover::Update(state.model2().clone()))
        };
        test::standard_transaction_tests(&state, &testfn_update);
        test::standard_transaction_tests(&state, &testfn_create);

        // test ResourceMover::Update()
        let mods = testfn_update(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("nomnomnom".into()));
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), state.model().id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource2.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource2.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        assert_eq!(resource_to2.id(), state.model2().id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company_to.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(num!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(num!(8) + num!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company_to.agent_id());
        assert_eq!(resource_to2.costs(), &(state.model2().costs().clone() + costs_to_move.clone()));

        // test ResourceMover::Create()
        let mods = testfn_create(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().provider().clone(), company_from.agent_id());
        assert_eq!(event.inner().receiver().clone(), company_to.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(costs_to_move.clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", num!(157) - num!(23));
        assert_eq!(resource3.id(), state.model().id());
        assert_eq!(resource3.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(num!(15), Unit::One)));
        assert_eq!(resource3.inner().onhand_quantity(), &Some(Measure::new(num!(15) - num!(8), Unit::One)));
        assert_eq!(resource3.in_custody_of(), &company_from.agent_id());
        assert_eq!(resource3.costs(), &(state.model().costs().clone() - costs_to_move.clone()));

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", num!(23));
        assert_eq!(resource_created.id(), state.model2().id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company_from.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(num!(0), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(num!(8), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company_to.agent_id());
        assert_eq!(resource_created.costs(), &costs_to_move);

        // can't override a resource you don't own
        let mut state2 = state.clone();
        state2.model2_mut().inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = testfn_update(&state2);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // can't transfer custody of a resource you don't have custody of
        let mut state3 = state.clone();
        state3.model_mut().set_in_custody_of(CompanyID::new("ziggy").into());
        let res = testfn_update(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
        let res = testfn_create(&state3);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company_to.agent_id()]);
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2, ResourceMover::Update(state.model2().clone()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));
        let res = testfn_inner(&state, &company_from, &company_to, &agreement2, ResourceMover::Create(state.model2().id().clone()));
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut state5 = state.clone();
        state5.company = Some(company_to.clone());
        test::deleted_company_tester(&state5, &testfn_update_to);
    }
}

