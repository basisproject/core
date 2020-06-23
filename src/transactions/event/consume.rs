//! Consume "uses up" (consumes) some or all of a resource, transferring its
//! costs into a process.
//!
//! If you make widgets out of steel, then steel is the resource, and the
//! process would be the fabrication that "consumes" steel (with the output,
//! ie `produce`, of a widget).

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
        resource::Resource,
        user::User,
    },
};
use om2::{Measure, NumericUnion};
use vf_rs::vf;

/// Consume some or all of a resource, transferring some or all of its costs
/// into a process.
pub fn consume<T: Into<NumericUnion>>(caller: &User, member: &CompanyMember, company: &Company, id: EventID, resource: Resource, process: Process, move_costs: Costs, move_measure: T, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::EventCreate)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::Consume)?;
    if company.is_deleted() {
        Err(Error::CompanyIsDeleted)?;
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
            company::{CompanyID, CompanyType},
            company_member::CompanyMemberID,
            event::{EventID, EventError},
            occupation::OccupationID,
            process::ProcessID,
            resource::ResourceID,
            testutils::{make_user, make_company, make_member, make_process, make_resource},
            user::UserID,
        },
        util,
    };
    use om2::Unit;
    use rust_decimal_macros::*;

    #[test]
    fn can_consume() {
        let now = util::time::now();
        let id = EventID::create();
        let company = make_company(&CompanyID::create(), CompanyType::Private, "jerry's widgets", &now);
        let user = make_user(&UserID::create(), None, &now);
        let occupation_id = OccupationID::new("machinist");
        let member = make_member(&CompanyMemberID::create(), user.id(), company.id(), &occupation_id, vec![], &now);
        let resource = make_resource(&ResourceID::new("widget"), company.id(), &Measure::new(dec!(15), Unit::One), &Costs::new_with_labor("homemaker", 157), &now);
        let mut costs = Costs::new();
        costs.track_labor(occupation_id.clone(), dec!(42.2));
        costs.track_labor("homemaker", dec!(13.6));
        let process = make_process(&ProcessID::create(), company.id(), "make widgets", &costs, &now);

        let res = consume(&user, &member, &company, id.clone(), resource.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member = member.clone();
        member.set_permissions(vec![CompanyPermission::Consume]);
        let mods = consume(&user, &member, &company, id.clone(), resource.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 3);
        let event = mods[0].clone().expect_op::<Event>(Op::Create).unwrap();
        let process2 = mods[1].clone().expect_op::<Process>(Op::Update).unwrap();
        let resource2 = mods[2].clone().expect_op::<Resource>(Op::Update).unwrap();

        assert_eq!(event.id(), &id);
        assert_eq!(event.inner().agreed_in(), &None);
        assert_eq!(event.inner().has_point_in_time(), &Some(now.clone()));
        assert_eq!(event.inner().input_of(), &Some(process.id().clone()));
        assert_eq!(event.inner().provider().clone(), company.id().clone().into());
        assert_eq!(event.inner().receiver().clone(), company.id().clone().into());
        assert_eq!(event.move_costs(), &Some(Costs::new_with_labor("homemaker", 23)));
        assert_eq!(event.active(), &true);
        assert_eq!(event.created(), &now);
        assert_eq!(event.updated(), &now);

        let mut costs2 = Costs::new();
        costs2.track_labor(occupation_id.clone(), dec!(42.2));
        costs2.track_labor("homemaker", dec!(23) + dec!(13.6));
        assert_eq!(process2.id(), process.id());
        assert_eq!(process2.company_id(), company.id());
        assert_eq!(process2.inner().name(), "make widgets");
        assert_eq!(process2.costs(), &costs2);

        let mut costs3 = Costs::new();
        costs3.track_labor("homemaker", dec!(157) - dec!(23));
        assert_eq!(resource2.id(), resource.id());
        assert_eq!(resource2.inner().accounting_quantity(), &Some(Measure::new(dec!(7), Unit::One)));
        assert_eq!(resource2.inner().onhand_quantity(), &Some(Measure::new(dec!(7), Unit::One)));
        assert_eq!(resource2.costs(), &costs3);

        let user2 = make_user(&UserID::create(), Some(vec![]), &now);
        let res = consume(&user2, &member, &company, id.clone(), resource.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut member2 = member.clone();
        member2.set_permissions(vec![]);
        let res = consume(&user, &member2, &company, id.clone(), resource.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        let mut company2 = company.clone();
        company2.set_deleted(Some(now.clone()));
        let res = consume(&user, &member, &company2, id.clone(), resource.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::CompanyIsDeleted));

        // can't consume into a process you don't own
        let mut process3 = process.clone();
        process3.set_company_id(CompanyID::new("zing"));
        let res = consume(&user, &member, &company, id.clone(), resource.clone(), process3.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::Event(EventError::ProcessOwnerMismatch)));

        // a company that doesn't own a resource can't consume it
        let mut resource3 = resource.clone();
        resource3.inner_mut().set_primary_accountable(Some(CompanyID::new("ziggy").into()));
        let res = consume(&user, &member, &company, id.clone(), resource3.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceOwnerMismatch)));

        // a company that doesn't have posession of a resource can't consume it
        let mut resource4 = resource.clone();
        resource4.set_in_custody_of(CompanyID::new("ziggy").into());
        let res = consume(&user, &member, &company, id.clone(), resource4.clone(), process.clone(), Costs::new_with_labor("homemaker", 23), 8, &now);
        assert_eq!(res, Err(Error::Event(EventError::ResourceCustodyMismatch)));
    }
}

