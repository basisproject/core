//! A resource is a tangible asset. It can represent a chair, a house, a forest,
//! a widget, a barrel of crude oil, etc.
//!
//! Resources are instances of a [resource_spec][1]. If the resource
//! specification is a product description on an online shop, the resource is
//! the actual delivered good that you receive when you order it.
//!
//! See the [resource model.][2]
//!
//! [1]: ../resource_spec/index.html
//! [2]: ../../models/resource/index.html

use crate::{
    access::Permission,
    costs::Costs,
    error::{Error, Result},
    models::{
        company::{Company, Permission as CompanyPermission},
        lib::{agent::Agent, basis_model::Model},
        member::Member,
        resource::{Resource, ResourceID},
        resource_spec::ResourceSpecID,
        user::User,
        Modifications, Op,
    },
};
use chrono::{DateTime, Utc};
use om2::Unit;
use url::Url;
use vf_rs::{dfc, vf};

/// Create a new resource
pub fn create(
    caller: &User,
    member: &Member,
    company: &Company,
    id: ResourceID,
    spec_id: ResourceSpecID,
    lot: Option<dfc::ProductBatch>,
    name: Option<String>,
    tracking_id: Option<String>,
    classifications: Vec<Url>,
    note: Option<String>,
    unit_of_effort: Option<Unit>,
    active: bool,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResources)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceCreate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    let model = Resource::builder()
        .id(id)
        .inner(
            vf::EconomicResource::builder()
                .classified_as(classifications)
                .conforms_to(spec_id)
                .lot(lot)
                .name(name)
                .note(note)
                .primary_accountable(Some(company.agent_id()))
                .tracking_identifier(tracking_id)
                .unit_of_effort(unit_of_effort)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?,
        )
        .in_custody_of(company.id().clone())
        .costs(Costs::new())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a resource
pub fn update(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: Resource,
    lot: Option<dfc::ProductBatch>,
    name: Option<String>,
    tracking_id: Option<String>,
    classifications: Option<Vec<Url>>,
    note: Option<String>,
    unit_of_effort: Option<Unit>,
    active: Option<bool>,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResources)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceUpdate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if lot.is_some() {
        subject.inner_mut().set_lot(lot);
    }
    if name.is_some() {
        subject.inner_mut().set_name(name);
    }
    if tracking_id.is_some() {
        subject.inner_mut().set_tracking_identifier(tracking_id);
    }
    if let Some(classifications) = classifications {
        subject.inner_mut().set_classified_as(classifications);
    }
    if note.is_some() {
        subject.inner_mut().set_note(note);
    }
    if unit_of_effort.is_some() {
        subject.inner_mut().set_unit_of_effort(unit_of_effort);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a resource
pub fn delete(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: Resource,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResources)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceDelete)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("resource".into()))?;
    }
    if !subject.costs().is_zero() {
        Err(Error::CannotEraseCosts)?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::resource_spec::ResourceSpecID,
        util::{
            self,
            test::{self, *},
        },
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = ResourceID::create();
        let state = TestState::standard(vec![CompanyPermission::ResourceCreate], &now);
        let spec = make_resource_spec(
            &ResourceSpecID::create(),
            state.company().id(),
            "widgets, baby",
            &now,
        );
        let lot = dfc::ProductBatch::builder()
            .batch_number("123")
            .build()
            .unwrap();

        let testfn = |state: &TestState<Resource, Resource>| {
            create(
                state.user(),
                state.member(),
                state.company(),
                id.clone(),
                spec.id().clone(),
                Some(lot.clone()),
                Some("widget batch".into()),
                None,
                vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()],
                Some("niceee".into()),
                Some(Unit::Hour),
                true,
                &now,
            )
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let resource = mods[0].clone().expect_op::<Resource>(Op::Create).unwrap();
        assert_eq!(resource.id(), &id);
        assert_eq!(resource.inner().name(), &Some("widget batch".into()));
        assert_eq!(resource.inner().lot(), &Some(lot.clone()));
        assert_eq!(
            resource.inner().classified_as(),
            &vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()]
        );
        assert_eq!(
            resource.inner().primary_accountable(),
            &Some(state.company().agent_id())
        );
        assert_eq!(resource.inner().tracking_identifier(), &None);
        assert_eq!(resource.inner().note(), &Some("niceee".into()));
        assert_eq!(resource.inner().unit_of_effort(), &Some(Unit::Hour));
        assert_eq!(resource.in_custody_of(), &state.company().agent_id());
        assert!(resource.costs().is_zero());
        assert_eq!(resource.active(), &true);
        assert_eq!(resource.created(), &now);
        assert_eq!(resource.updated(), &now);
        assert_eq!(resource.deleted(), &None);
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = ResourceID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::ResourceCreate,
                CompanyPermission::ResourceUpdate,
            ],
            &now,
        );
        let spec = make_resource_spec(
            &ResourceSpecID::create(),
            state.company().id(),
            "widgets, baby",
            &now,
        );
        let lot = dfc::ProductBatch::builder()
            .batch_number("123")
            .build()
            .unwrap();
        let mods = create(
            state.user(),
            state.member(),
            state.company(),
            id.clone(),
            spec.id().clone(),
            Some(lot.clone()),
            Some("widget batch".into()),
            None,
            vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()],
            Some("niceee".into()),
            Some(Unit::Hour),
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let resource = mods[0].clone().expect_op::<Resource>(Op::Create).unwrap();
        state.model = Some(resource);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Resource, Resource>| {
            update(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                None,
                Some("better widgets".into()),
                Some("444-computers-and-equipment".into()),
                None,
                None,
                Some(Unit::WattHour),
                Some(false),
                &now2,
            )
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let resource2 = mods[0].clone().expect_op::<Resource>(Op::Update).unwrap();
        assert_eq!(resource2.id(), &id);
        assert_eq!(resource2.inner().name(), &Some("better widgets".into()));
        assert_eq!(resource2.inner().lot(), &Some(lot.clone()));
        assert_eq!(
            resource2.inner().classified_as(),
            &vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()]
        );
        assert_eq!(
            resource2.inner().primary_accountable(),
            &Some(state.company().agent_id())
        );
        assert_eq!(
            resource2.inner().tracking_identifier(),
            &Some("444-computers-and-equipment".into())
        );
        assert_eq!(resource2.inner().note(), &Some("niceee".into()));
        assert_eq!(resource2.inner().unit_of_effort(), &Some(Unit::WattHour));
        assert_eq!(resource2.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource2.active(), &false);
        assert_eq!(resource2.created(), &now);
        assert_eq!(resource2.updated(), &now2);
        assert_eq!(resource2.deleted(), &None);
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = ResourceID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::ResourceCreate,
                CompanyPermission::ResourceDelete,
            ],
            &now,
        );
        let spec = make_resource_spec(
            &ResourceSpecID::create(),
            state.company().id(),
            "widgets, baby",
            &now,
        );
        let lot = dfc::ProductBatch::builder()
            .batch_number("123")
            .build()
            .unwrap();
        let mods = create(
            state.user(),
            state.member(),
            state.company(),
            id.clone(),
            spec.id().clone(),
            Some(lot.clone()),
            Some("widget batch".into()),
            None,
            vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()],
            Some("niceee".into()),
            Some(Unit::Hour),
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let resource = mods[0].clone().expect_op::<Resource>(Op::Create).unwrap();
        state.model = Some(resource);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Resource, Resource>| {
            delete(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                &now2,
            )
        };
        test::standard_transaction_tests(&state, &testfn);
        test::double_deleted_tester(&state, "resource", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let resource2 = mods[0].clone().expect_op::<Resource>(Op::Delete).unwrap();
        assert_eq!(resource2.id(), &id);
        assert_eq!(resource2.inner().name(), &Some("widget batch".into()));
        assert_eq!(resource2.inner().lot(), &Some(lot.clone()));
        assert_eq!(
            resource2.inner().classified_as(),
            &vec!["https://www.wikidata.org/wiki/Q605117".parse().unwrap()]
        );
        assert_eq!(
            resource2.inner().primary_accountable(),
            &Some(state.company().agent_id())
        );
        assert_eq!(resource2.inner().tracking_identifier(), &None);
        assert_eq!(resource2.inner().note(), &Some("niceee".into()));
        assert_eq!(resource2.inner().unit_of_effort(), &Some(Unit::Hour));
        assert_eq!(resource2.in_custody_of(), &state.company().agent_id());
        assert_eq!(resource2.active(), &true);
        assert_eq!(resource2.created(), &now);
        assert_eq!(resource2.updated(), &now);
        assert_eq!(resource2.deleted(), &Some(now2.clone()));
    }
}
