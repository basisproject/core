//! Modification deals with altering resources in-place without changing
//! quantities.
//!
//! For instance, modification could describe a repair of a vehicle or large
//! machine.

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
        lib::basis_model::Deletable,
        process::Process,
        resource::Resource,
        user::User,
    },
};
use om2::{Measure, NumericUnion};
use vf_rs::vf;

/// Accept a resource (for repair).
///
/// Effectively, you `accept` a resource into a repair process, and the output
/// of that process would be `modify`.
pub fn accept<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, resource: Resource, process: Process, resource_measure: T, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Accept)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }

    let measure = {
        let unit = resource.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(resource_measure, unit)
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
                .action(vf::Action::Accept)
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

/// Modify (repair) a resource.
///
/// Effectively, you `accept` a resource into a repair process, and the output
/// of that process would be `modify`.
pub fn modify<T: Into<NumericUnion>>(caller: &User, member: &Member, company: &Company, id: EventID, process: Process, resource: Resource, move_costs: Costs, resource_measure: T, note: Option<String>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Modify)?;
    if company.is_deleted() {
        Err(Error::ObjectIsDeleted("company".into()))?;
    }

    let measure = {
        let unit = resource.get_unit().ok_or(Error::ResourceMeasureMissing)?;
        Measure::new(resource_measure, unit)
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
                .action(vf::Action::Modify)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            company::CompanyID,
            member::MemberID,
            event::{EventError, EventID},
            lib::agent::Agent,
            occupation::OccupationID,
            process::ProcessID,
            resource::ResourceID,
            testutils::{make_user, make_company, make_member_worker, make_process, make_resource},
            user::UserID,
        },
        util,
    };
    use om2::{Measure, Unit};
    use rust_decimal_macros::*;

    #[test]
    fn can_accept() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("mechanic");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("widget"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_resource("steel", 157), &now);
        let process = make_process(&ProcessID::create(), company.id(), "make widgets", &Costs::new(), &now);

        let res = accept(&user, &member, &company, id.clone(), resource.clone(), process.clone(), 3, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Accept]);
        let mods = accept(&user, &member, &company, id.clone(), resource.clone(), process.clone(), 3, Some("memo lol".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 2);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let resource2 = mods[1].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &Some(process.id().clone()));
        assert_eq!(event.inner().note(), &Some("memo lol".into()));
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.inner().resource_quantity(), &Some(Measure::new(3, Unit::One)));
        assert_eq!(event.move_costs(), &Some(Costs::new()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource2.in_custody_of(), &company.agent_id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(15), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(12), Unit::One)));
        assert_eq!(resource2.costs(), &Costs::new_with_resource("steel", 157));

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = accept(&user2, &member, &company, id.clone(), resource.clone(), process.clone(), 3, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = accept(&user, &member2, &company, id.clone(), resource.clone(), process.clone(), 3, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now.clone()));
        let res = accept(&user, &member, &company2, id.clone(), resource.clone(), process.clone(), 3, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));

        // can't accept into a process you don't own
        let mut process3 = process.clone();
        process3.set_company_id(CompanyID::new("zing"));
        let res = accept(&user, &member, &company, id.clone(), resource.clone(), process3.clone(), 3, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource *can* accept it
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = accept(&user, &member, &company, id.clone(), resource3.clone(), process.clone(), 3, Some("memo lol".into()), &now);
        assert!(res.is_ok());

        // a company that doesn't have possession of a resource can't accept it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = accept(&user, &member, &company, id.clone(), resource4.clone(), process.clone(), 3, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }

    #[test]
    fn can_modify() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("mechanic");
        let member = make_member_worker(&MemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("car"), company.id(), &Measure::new(dec!(3), Unit::One), &Costs::new_with_resource("steel", 157), &now);
        let costs = Costs::new_with_labor(occupation_id.clone(), dec!(102.3));
        let process = make_process(&ProcessID::create(), company.id(), "repair car", &costs, &now);

        let res = modify(&user, &member, &company, id.clone(), process.clone(), resource.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Modify]);
        let mods = modify(&user, &member, &company, id.clone(), process.clone(), resource.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let process2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        let resource2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &None);
        assert_eq!(event.inner().note(), &Some("memo lol".into()));
        assert_eq!(event.inner().output_of(), &Some(process.id().clone()));
        assert_eq!(event.inner().provider().clone(), company.agent_id());
        assert_eq!(event.inner().receiver().clone(), company.agent_id());
        assert_eq!(event.move_costs(), &Some(process.costs().clone()));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        assert_eq!(process2.id(), process.id());
        assert_eq!(process2.company_id(), company.id());
        assert_eq!(process2.inner().name(), "repair car");
        assert_eq!(process2.costs(), &Costs::new());

        let mut costs2 = Costs::new();
        costs2.track_labor(occupation_id.clone(), dec!(102.3));
        costs2.track_resource("steel", 157);
        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().primary_accountable(), &Some(company.agent_id()));
        assert_eq!(resource2.in_custody_of(), &company.agent_id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(3), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(15), Unit::One)));
        assert_eq!(resource2.costs(), &costs2);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = modify(&user2, &member, &company, id.clone(), process.clone(), resource.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = modify(&user, &member2, &company, id.clone(), process.clone(), resource.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now.clone()));
        let res = modify(&user, &member, &company2, id.clone(), process.clone(), resource.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::ObjectIsDeleted("company".into())));

        // can't modify from a process you don't own
        let mut process3 = process.clone();
        process3.set_company_id(CompanyID::new("zing"));
        let res = modify(&user, &member, &company, id.clone(), process3.clone(), resource.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource can't modify it
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = modify(&user, &member, &company, id.clone(), process.clone(), resource3.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't modify it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = modify(&user, &member, &company, id.clone(), process.clone(), resource4.clone(), process.costs().clone(), 12, Some("memo lol".into()), &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }
}

