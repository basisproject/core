//! The transfer transactions are geared towards passing ownership, custody, or
//! both from one agent to another.
//!
//! If you're looking for internal transfers, see the [accounting transactions.][1]
//!
//! [1]: ../accounting/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    costs::Costs,
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
};
use om2::{Measure, NumericUnion};
use url::Url;
use vf_rs::vf;

/// Transfer a resource (custody and ownership) from one company to another,
/// moving a set of costs with it.
pub fn transfer<T: Into<NumericUnion>>(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs: Costs, move_measure: T, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
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
pub fn transfer_all_rights<T: Into<NumericUnion>>(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs: Costs, move_measure: T, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
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
pub fn transfer_custody<T: Into<NumericUnion>>(caller: &User, member: &Member, company_from: &Company, company_to: &Company, agreement: &Agreement, id: EventID, resource_from: Resource, resource_to: ResourceMover, move_costs: Costs, move_measure: T, agreed_in: Option<Url>, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
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
        models::{
            agreement::AgreementID,
            company::CompanyID,
            member::MemberID,
            event::{EventID, EventError},
            lib::agent::Agent,
            occupation::OccupationID,
            resource::ResourceID,
            testutils::{deleted_company_tester, make_agreement, make_user, make_company, make_member_worker, make_resource},
            user::UserID,
        },
        util,
    };
    use om2::Unit;
    use rust_decimal_macros::*;

    #[test]
    fn can_transfer() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's planks", &now);
        let company2 = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company.agent_id(), company2.agent_id()], "order 1234", "gotta get some planks", &now);
        let agreed_in: Url = "https://legalzoom.com/standard-boilerplate-hereto-notwithstanding-each-of-them-damage-to-the-hood-ornament-alone".parse().unwrap();
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("plank"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company2.id(), &Measure::new(dec!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);

        let res = transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(agreed_in.clone()), Some("giving jinkey some post-capitalist planks".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Transfer]);
        // test ResourceMover::Update()
        let mods = transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(agreed_in.clone()), Some("giving jinkey some post-capitalist planks".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("giving jinkey some post-capitalist planks".into()));
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company2.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource2.in_custody_of(), &company.agent_id());
        assert_eq!(resource2.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23) + dec!(2));
        assert_eq!(resource_to2.id(), resource_to.id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company2.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company2.agent_id());
        assert_eq!(resource_to2.costs(), &costs2);

        // test ResourceMover::Create()
        let mods = transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Create(resource_to.id().clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company2.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource3.id(), resource.id());
        assert_eq!(resource3.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource3.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource3.in_custody_of(), &company.agent_id());
        assert_eq!(resource3.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23));
        assert_eq!(resource_created.id(), resource_to.id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company2.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company2.agent_id());
        assert_eq!(resource_created.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = transfer(&user2, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = transfer(&user, &member2, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company: Company| {
            transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now)
        });
        deleted_company_tester(company2.clone(), &now, |company_to: Company| {
            transfer(&user, &member, &company, &company_to, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now)
        });

        // can't transfer into a resource you don't own
        let mut resource_to3 = resource_to.clone();
        resource_to3.inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to3.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't own a resource can't transfer it OBVIOUSLY
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource3.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't transfer it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = transfer(&user, &member, &company, &company2, &agreement, id.clone(), resource4.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company2.agent_id()]);
        let res = transfer(&user, &member, &company, &company2, &agreement2, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_transfer_all_rights() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's planks", &now);
        let company2 = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company.agent_id(), company2.agent_id()], "order 1234", "gotta get some planks", &now);
        let agreed_in: Url = "https://legalzoom.com/is-it-too-much-to-ask-for-todays-pedestrian-to-wear-at-least-one-piece-of-reflective-clothing".parse().unwrap();
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("plank"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company2.id(), &Measure::new(dec!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);

        let res = transfer_all_rights(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Transfer]);
        // test ResourceMover::Update()
        let mods = transfer_all_rights(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(agreed_in.clone()), Some("note blah blah".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("note blah blah".into()));
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company2.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(15), Unit::One)));
        assert_eq!(resource2.in_custody_of(), &company.agent_id());
        assert_eq!(resource2.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23) + dec!(2));
        assert_eq!(resource_to2.id(), resource_to.id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company2.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(dec!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company2.agent_id());
        assert_eq!(resource_to2.costs(), &costs2);

        // test ResourceMover::Create()
        let mods = transfer_all_rights(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Create(resource_to.id().clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company2.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource3.id(), resource.id());
        assert_eq!(resource3.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource3.inner().onhand_quantity(), &Some(Measure::new(dec!(15), Unit::One)));
        assert_eq!(resource3.in_custody_of(), &company.agent_id());
        assert_eq!(resource3.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23));
        assert_eq!(resource_created.id(), resource_to.id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company2.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(dec!(0), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company.agent_id());
        assert_eq!(resource_created.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = transfer_all_rights(&user2, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = transfer_all_rights(&user, &member2, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company_from: Company| {
            transfer_all_rights(&user, &member, &company_from, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now)
        });
        deleted_company_tester(company2.clone(), &now, |company_to: Company| {
            transfer_all_rights(&user, &member, &company, &company_to, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now)
        });

        // can't transfer into a resource you don't own
        let mut resource_to3 = resource_to.clone();
        resource_to3.inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = transfer_all_rights(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to3.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't own a resource can't transfer it OBVIOUSLY
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = transfer_all_rights(&user, &member, &company, &company2, &agreement, id.clone(), resource3.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company2.agent_id()]);
        let res = transfer_all_rights(&user, &member, &company, &company2, &agreement2, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_transfer_custody() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's planks", &now);
        let company2 = make_company(&CompanyID::create(), "jinkey's skateboards", &now);
        let agreement = make_agreement(&AgreementID::create(), &vec![company.agent_id(), company2.agent_id()], "order 1234", "gotta get some planks", &now);
        let agreed_in: Url = "https://legaldoom.com/trade-secrets-trade-secrets".parse().unwrap();
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("plank"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company2.id(), &Measure::new(dec!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);

        let res = transfer_custody(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Transfer]);
        // test ResourceMover::Update()
        let mods = transfer_custody(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(agreed_in.clone()), Some("nomnomnom".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &Some(agreed_in.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("nomnomnom".into()));
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().realization_of(), &Some(agreement.id().clone()));
        assert_eq!(event.inner().receiver().clone(), company2.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(15), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource2.in_custody_of(), &company.agent_id());
        assert_eq!(resource2.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23) + dec!(2));
        assert_eq!(resource_to2.id(), resource_to.id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company2.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(dec!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company2.agent_id());
        assert_eq!(resource_to2.costs(), &costs2);

        // test ResourceMover::Create()
        let mods = transfer_custody(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Create(resource_to.id().clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company2.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource3.id(), resource.id());
        assert_eq!(resource3.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource3.inner().accounting_quantity(), &Some(Measure::new(dec!(15), Unit::One)));
        assert_eq!(resource3.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource3.in_custody_of(), &company.agent_id());
        assert_eq!(resource3.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23));
        assert_eq!(resource_created.id(), resource_to.id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(dec!(0), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company2.agent_id());
        assert_eq!(resource_created.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = transfer_custody(&user2, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = transfer_custody(&user, &member2, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company_from: Company| {
            transfer_custody(&user, &member, &company_from, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now)
        });
        deleted_company_tester(company2.clone(), &now, |company_to: Company| {
            transfer_custody(&user, &member, &company, &company_to, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now)
        });

        // can't override a resource you don't own
        let mut resource_to3 = resource_to.clone();
        resource_to3.inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = transfer_custody(&user, &member, &company, &company2, &agreement, id.clone(), resource.clone(), ResourceMover::Update(resource_to3.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // can't transfer custody of a resource you don't have custody of
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = transfer_custody(&user, &member, &company, &company2, &agreement, id.clone(), resource4.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

        // can't add an event unless both parties are participants in the agreement
        let mut agreement2 = agreement.clone();
        agreement2.set_participants(vec![company2.agent_id()]);
        let res = transfer_custody(&user, &member, &company, &company2, &agreement2, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, None, None, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

