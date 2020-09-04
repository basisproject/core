//! A resource specification is a general description of a tangible asset.
//!
//! Every [resource][1] is an instance of a resource specification. For
//! instance, a *resource specification* might be a product listing page for a
//! "Haworth Zody" chair, and the *resource* is the cheap knock-off counterfeit
//! that Amazon ships to you when you order it.
//!
//! See the [resource spec model.][2]
//!
//! [1]: ../resource/index.html
//! [2]: ../../models/resource_spec/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        company::{Company, Permission as CompanyPermission},
        member::Member,
        lib::basis_model::Model,
        resource_spec::{ResourceSpec, ResourceSpecID},
        user::User,
    },
};
use om2::Unit;
use url::Url;
use vf_rs::vf;

/// Create a new ResourceSpec
pub fn create<T: Into<String>>(caller: &User, member: &Member, company: &Company, id: ResourceSpecID, name: T, note: T, classifications: Vec<Url>, default_unit_of_effort: Option<Unit>, default_unit_of_resource: Option<Unit>, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResourceSpecs)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceSpecCreate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    let model = ResourceSpec::builder()
        .id(id)
        .inner(
            vf::ResourceSpecification::builder()
                .default_unit_of_effort(default_unit_of_effort)
                .default_unit_of_resource(default_unit_of_resource)
                .name(name)
                .note(Some(note.into()))
                .resource_classified_as(classifications)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .company_id(company.id().clone())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a resource spec
pub fn update(caller: &User, member: &Member, company: &Company, mut subject: ResourceSpec, name: Option<String>, note: Option<String>, classifications: Option<Vec<Url>>, default_unit_of_effort: Option<Unit>, default_unit_of_resource: Option<Unit>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResourceSpecs)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceSpecUpdate)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(note) = note {
        subject.inner_mut().set_note(Some(note));
    }
    if let Some(classifications) = classifications {
        subject.inner_mut().set_resource_classified_as(classifications);
    }
    if default_unit_of_effort.is_some() {
        subject.inner_mut().set_default_unit_of_effort(default_unit_of_effort);
    }
    if default_unit_of_resource.is_some() {
        subject.inner_mut().set_default_unit_of_resource(default_unit_of_resource);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a resource spec
pub fn delete(caller: &User, member: &Member, company: &Company, mut subject: ResourceSpec, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateResourceSpecs)?;
    member.access_check(caller.id(), company.id(), CompanyPermission::ResourceSpecDelete)?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("resource_spec".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::{
            resource_spec::{ResourceSpec, ResourceSpecID},
        },
        util::{self, test::{self, *}},
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = ResourceSpecID::create();
        let state = TestState::standard(vec![CompanyPermission::ResourceSpecCreate], &now);

        let testfn = |state: &TestState<ResourceSpec, ResourceSpec>| {
            create(state.user(), state.member(), state.company(), id.clone(), "Beans", "yummy", vec!["https://www.wikidata.org/wiki/Q379813".parse().unwrap()], Some(Unit::Hour), Some(Unit::Kilogram), true, &now)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec = mods[0].clone().expect_op::<ResourceSpec>(Op::Create).unwrap();
        assert_eq!(recspec.id(), &id);
        assert_eq!(recspec.inner().default_unit_of_effort(), &Some(Unit::Hour));
        assert_eq!(recspec.inner().default_unit_of_resource(), &Some(Unit::Kilogram));
        assert_eq!(recspec.inner().name(), "Beans");
        assert_eq!(recspec.inner().note(), &Some("yummy".into()));
        assert_eq!(recspec.inner().resource_classified_as(), &vec!["https://www.wikidata.org/wiki/Q379813".parse().unwrap()]);
        assert_eq!(recspec.company_id(), state.company().id());
        assert_eq!(recspec.active(), &true);
        assert_eq!(recspec.created(), &now);
        assert_eq!(recspec.updated(), &now);
        assert_eq!(recspec.deleted(), &None);
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = ResourceSpecID::create();
        let mut state = TestState::standard(vec![CompanyPermission::ResourceSpecCreate, CompanyPermission::ResourceSpecUpdate], &now);
        let mods = create(state.user(), state.member(), state.company(), id.clone(), "Beans", "yummy", vec!["https://www.wikidata.org/wiki/Q379813".parse().unwrap()], Some(Unit::Hour), Some(Unit::Kilogram), true, &now).unwrap().into_vec();
        let recspec = mods[0].clone().expect_op::<ResourceSpec>(Op::Create).unwrap();
        state.model = Some(recspec);

        let now2 = util::time::now();
        let testfn = |state: &TestState<ResourceSpec, ResourceSpec>| {
            update(state.user(), state.member(), state.company(), state.model().clone(), Some("best widget".into()), None, None, Some(Unit::WattHour), None, Some(false), &now2)
        };
        test::standard_transaction_tests(&state, &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec2 = mods[0].clone().expect_op::<ResourceSpec>(Op::Update).unwrap();
        assert_eq!(recspec2.id(), &id);
        assert_eq!(recspec2.inner().default_unit_of_effort(), &Some(Unit::WattHour));
        assert_eq!(recspec2.inner().default_unit_of_resource(), &Some(Unit::Kilogram));
        assert_eq!(recspec2.inner().name(), "best widget");
        assert_eq!(recspec2.inner().note(), &Some("yummy".into()));
        assert_eq!(recspec2.inner().resource_classified_as(), &vec!["https://www.wikidata.org/wiki/Q379813".parse().unwrap()]);
        assert_eq!(recspec2.company_id(), state.company().id());
        assert_eq!(recspec2.active(), &false);
        assert_eq!(recspec2.created(), &now);
        assert_eq!(recspec2.updated(), &now2);
        assert_eq!(recspec2.deleted(), &None);
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = ResourceSpecID::create();
        let mut state = TestState::standard(vec![CompanyPermission::ResourceSpecCreate, CompanyPermission::ResourceSpecDelete], &now);
        let mods = create(state.user(), state.member(), state.company(), id.clone(), "Beans", "yummy", vec!["https://www.wikidata.org/wiki/Q379813".parse().unwrap()], Some(Unit::Hour), Some(Unit::Kilogram), true, &now).unwrap().into_vec();
        let recspec = mods[0].clone().expect_op::<ResourceSpec>(Op::Create).unwrap();
        state.model = Some(recspec);

        let now2 = util::time::now();
        let testfn = |state: &TestState<ResourceSpec, ResourceSpec>| {
            delete(state.user(), state.member(), state.company(), state.model().clone(), &now2)
        };
        test::standard_transaction_tests(&state, &testfn);
        test::double_deleted_tester(&state, "resource_spec", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec2 = mods[0].clone().expect_op::<ResourceSpec>(Op::Delete).unwrap();
        assert_eq!(recspec2.id(), &id);
        assert_eq!(recspec2.inner().default_unit_of_effort(), &Some(Unit::Hour));
        assert_eq!(recspec2.inner().default_unit_of_resource(), &Some(Unit::Kilogram));
        assert_eq!(recspec2.inner().name(), "Beans");
        assert_eq!(recspec2.inner().resource_classified_as(), &vec!["https://www.wikidata.org/wiki/Q379813".parse().unwrap()]);
        assert_eq!(recspec2.company_id(), state.company().id());
        assert_eq!(recspec2.active(), &true);
        assert_eq!(recspec2.created(), &now);
        assert_eq!(recspec2.updated(), &now);
        assert_eq!(recspec2.deleted(), &Some(now2.clone()));
    }
}

