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

/// Raise the quantity (both accounting and obhand) or a resource by a fixed
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
            member::MemberID,
            event::{EventID, EventError},
            occupation::OccupationID,
            process::{Process, ProcessID},
            resource::ResourceID,
            testutils::{deleted_company_tester, make_user, make_company, make_member_worker, make_process, make_resource},
            user::UserID,
        },
        util::{self, test},
    };
    use om2::Unit;
    use rust_decimal_macros::*;

    #[test]
    fn can_lower() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![CompanyPermission::Lower], &now);
        let resource = make_resource(&ResourceID::new("widget"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);

        let testfn = |user, member, company, _: Option<Event>| {
            lower(&user, &member, &company, id.clone(), resource.clone(), 8, Some("a note".into()), &now)
        };
        test::standard_transaction_tests(user.clone(), member.clone(), company.clone(), None, testfn.clone());

        let mods = testfn(user.clone(), member.clone(), company.clone(), None).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("a note".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &None);
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(7), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(7), Unit::One)));
        assert_eq!(resource2.costs(), resource.costs());

        // a company that doesn't own a resource can't lower it
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = lower(&user, &member, &company, id.clone(), resource3.clone(), 8, Some("a note".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have possession of a resource can't lower it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = lower(&user, &member, &company, id.clone(), resource4.clone(), 8, Some("a note".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_move_costs() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's planks", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("lawyer");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let process_from = make_process(&ProcessID::create(), company.id(), "various lawyerings", &Costs::new_with_labor(occupation_id.clone(), dec!(177.25)), &now);
        let process_to = make_process(&ProcessID::create(), company.id(), "overflow labor", &Costs::new_with_labor(occupation_id.clone(), dec!(804)), &now);

        let res = move_costs(&user, &member, &company, id.clone(), process_from.clone(), process_to.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::MoveCosts]);
        // test ResourceMover::Update()
        let mods = move_costs(&user, &member, &company, id.clone(), process_from.clone(), process_to.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let process_from2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        let process_to2 = mods[2].clone().expect_op::<Process>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &Some(process_to.id().clone()));
        assert_eq!(event.inner().note(), &Some("my note".into()));
        assert_eq!(event.inner().output_of(), &Some(process_from.id().clone()));
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.inner().resource_quantity(), &None);
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("lawyer", 100)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("lawyer", dec!(177.25) - dec!(100));
        assert_eq!(process_from2.id(), process_from.id());
        assert_eq!(process_from2.company_id(), company.id());
        assert_eq!(process_from2.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("lawyer", dec!(804) + dec!(100));
        assert_eq!(process_to2.id(), process_to.id());
        assert_eq!(process_to2.company_id(), company.id());
        assert_eq!(process_to2.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = move_costs(&user2, &member, &company, id.clone(), process_from.clone(), process_to.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = move_costs(&user, &member2, &company, id.clone(), process_from.clone(), process_to.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company: Company| {
            move_costs(&user, &member, &company, id.clone(), process_from.clone(), process_to.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now)
        });

        // can't move costs from a process you don't own
        let mut process_from3 = process_from.clone();
        process_from3.set_company_id(CompanyID::new("zing").into());
        let res = move_costs(&user, &member, &company, id.clone(), process_from3.clone(), process_to.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // can't move costs into a process you don't own
        let mut process_to3 = process_to.clone();
        process_to3.set_company_id(CompanyID::new("zing").into());
        let res = move_costs(&user, &member, &company, id.clone(), process_from.clone(), process_to3.clone(), Costs::new_with_labor("lawyer", 100), Some("my note".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));
    }

    #[test]
    fn can_move_resource() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's planks", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("plank"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let resource_to = make_resource(&ResourceID::new("plank"), company.id(), &Measure::new(dec!(3), Unit::One), &Costs::new_with_labor("homemaker", 2), &now);
        let loc = SpatialThing::builder()
            .mappable_address(Some("1212 Uranus lane, DERRSOYBOY, KY, 33133".into()))
            .build().unwrap();

        let res = move_resource(&user, &member, &company, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::MoveResource]);
        // test ResourceMover::Update()
        let mods = move_resource(&user, &member, &company, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource_from2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_to2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().at_location(), &Some(loc.clone()));
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("lol".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource_from2.id(), resource.id());
        assert_eq!(resource_from2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource_from2.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from2.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from2.in_custody_of(), &company.agent_id());
        assert_eq!(resource_from2.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23) + dec!(2));
        assert_eq!(resource_to2.id(), resource_to.id());
        assert_eq!(resource_to2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource_to2.inner().accounting_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.inner().onhand_quantity(), &Some(Measure::new(dec!(8) + dec!(3), Unit::One)));
        assert_eq!(resource_to2.in_custody_of(), &company.agent_id());
        assert_eq!(resource_to2.costs(), &costs2);

        // test ResourceMover::Create()
        let mods = move_resource(&user, &member, &company, id.clone(), resource.clone(), ResourceMover::Create(resource_to.id().clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), None, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource_from3 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();
        let resource_created = mods[2].clone().expect_op::<Resource>(Op::Create).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &None);
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource_from3.id(), resource.id());
        assert_eq!(resource_from3.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource_from3.inner().accounting_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from3.inner().onhand_quantity(), &Some(Measure::new(dec!(15) - dec!(8), Unit::One)));
        assert_eq!(resource_from3.in_custody_of(), &company.agent_id());
        assert_eq!(resource_from3.costs(), &costs2);

        let mut costs2 = Costs::new();
        costs2.track_labor("homemaker", dec!(23));
        assert_eq!(resource_created.id(), resource_to.id());
        assert_eq!(resource_created.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource_created.inner().accounting_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.inner().onhand_quantity(), &Some(Measure::new(dec!(8), Unit::One)));
        assert_eq!(resource_created.in_custody_of(), &company.agent_id());
        assert_eq!(resource_created.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = move_resource(&user2, &member, &company, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = move_resource(&user, &member2, &company, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company: Company| {
            move_resource(&user, &member, &company, id.clone(), resource.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now)
        });

        // can't move into a resource you don't own
        let mut resource_to3 = resource_to.clone();
        resource_to3.inner_mut().set_primary_accountable(Some(CompanyID::new("zing").into()));
        let res = move_resource(&user, &member, &company, id.clone(), resource.clone(), ResourceMover::Update(resource_to3.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't own a resource can't move it OBVIOUSLY
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = move_resource(&user, &member, &company, id.clone(), resource3.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't move it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = move_resource(&user, &member, &company, id.clone(), resource4.clone(), ResourceMover::Update(resource_to.clone()), Costs::new_with_labor("homemaker", 23), 8, Some(loc.clone()), Some("lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_raise() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("widget"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);

        let res = raise(&user, &member, &company, id.clone(), resource.clone(), 8, Some("toot".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Raise]);
        let mods = raise(&user, &member, &company, id.clone(), resource.clone(), 8, Some("toot".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("toot".into()));
        assert_eq!(event.inner().output_of(), &None);
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(8, Unit::One)));
        assert_eq!(event.move_costs(), &None);
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(23), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(23), Unit::One)));
        assert_eq!(resource2.costs(), resource.costs());

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = raise(&user2, &member, &company, id.clone(), resource.clone(), 8, Some("toot".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = raise(&user, &member2, &company, id.clone(), resource.clone(), 8, Some("toot".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        deleted_company_tester(company.clone(), &now, |company: Company| {
            raise(&user, &member, &company, id.clone(), resource.clone(), 8, Some("toot".into()), &now)
        });

        // a company that doesn't own a resource can't raise it
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = raise(&user, &member, &company, id.clone(), resource3.clone(), 8, Some("toot".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have possession of a resource can't raise it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = raise(&user, &member, &company, id.clone(), resource4.clone(), 8, Some("toot".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));

    }
}

