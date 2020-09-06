//! A process specification is a blueprint for a process. Each process is an
//! *instance* of a process specification.
//!
//! For instance, if you make five widgets today, you might have five processes
//! for each widget, but one process specification called "build widgets" that
//! those five processes reference.
//!
//! See the [process spec model.][1]
//!
//! [1]: ../../models/process_spec/index.html

use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        company::{Company, Permission as CompanyPermission},
        lib::basis_model::Model,
        member::Member,
        process_spec::{ProcessSpec, ProcessSpecID},
        user::User,
        Modifications, Op,
    },
};
use chrono::{DateTime, Utc};
use vf_rs::vf;

/// Create a new ProcessSpec
pub fn create<T: Into<String>>(
    caller: &User,
    member: &Member,
    company: &Company,
    id: ProcessSpecID,
    name: T,
    note: T,
    active: bool,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcessSpecs)?;
    member.access_check(
        caller.id(),
        company.id(),
        CompanyPermission::ProcessSpecCreate,
    )?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    let model = ProcessSpec::builder()
        .id(id)
        .inner(
            vf::ProcessSpecification::builder()
                .name(name)
                .note(Some(note.into()))
                .build()
                .map_err(|e| Error::BuilderFailed(e))?,
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
pub fn update(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: ProcessSpec,
    name: Option<String>,
    note: Option<String>,
    active: Option<bool>,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcessSpecs)?;
    member.access_check(
        caller.id(),
        company.id(),
        CompanyPermission::ProcessSpecUpdate,
    )?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if let Some(name) = name {
        subject.inner_mut().set_name(name);
    }
    if let Some(note) = note {
        subject.inner_mut().set_note(Some(note));
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a resource spec
pub fn delete(
    caller: &User,
    member: &Member,
    company: &Company,
    mut subject: ProcessSpec,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::CompanyUpdateProcessSpecs)?;
    member.access_check(
        caller.id(),
        company.id(),
        CompanyPermission::ProcessSpecDelete,
    )?;
    if !company.is_active() {
        Err(Error::ObjectIsInactive("company".into()))?;
    }
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("process_spec".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        models::process_spec::{ProcessSpec, ProcessSpecID},
        util::{
            self,
            test::{self, *},
        },
    };

    #[test]
    fn can_create() {
        let now = util::time::now();
        let id = ProcessSpecID::create();
        let state = TestState::standard(vec![CompanyPermission::ProcessSpecCreate], &now);

        let testfn = |state: &TestState<ProcessSpec, ProcessSpec>| {
            create(
                state.user(),
                state.member(),
                state.company(),
                id.clone(),
                "SEIZE THE MEANS OF PRODUCTION",
                "our first process",
                true,
                &now,
            )
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let recspec = mods[0]
            .clone()
            .expect_op::<ProcessSpec>(Op::Create)
            .unwrap();
        assert_eq!(recspec.id(), &id);
        assert_eq!(recspec.inner().name(), "SEIZE THE MEANS OF PRODUCTION");
        assert_eq!(recspec.inner().note(), &Some("our first process".into()));
        assert_eq!(recspec.company_id(), state.company().id());
        assert_eq!(recspec.active(), &true);
        assert_eq!(recspec.created(), &now);
        assert_eq!(recspec.updated(), &now);
        assert_eq!(recspec.deleted(), &None);
    }

    #[test]
    fn can_update() {
        let now = util::time::now();
        let id = ProcessSpecID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::ProcessSpecCreate,
                CompanyPermission::ProcessSpecUpdate,
            ],
            &now,
        );
        let mods = create(
            state.user(),
            state.member(),
            state.company(),
            id.clone(),
            "SEIZE THE MEANS OF PRODUCTION",
            "our first process",
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let procspec = mods[0]
            .clone()
            .expect_op::<ProcessSpec>(Op::Create)
            .unwrap();
        state.model = Some(procspec);

        let now2 = util::time::now();
        let testfn = |state: &TestState<ProcessSpec, ProcessSpec>| {
            update(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                Some("best widget".into()),
                None,
                Some(false),
                &now2,
            )
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let procspec2 = mods[0]
            .clone()
            .expect_op::<ProcessSpec>(Op::Update)
            .unwrap();
        assert_eq!(procspec2.id(), &id);
        assert_eq!(procspec2.inner().name(), "best widget");
        assert_eq!(procspec2.inner().note(), &Some("our first process".into()));
        assert_eq!(procspec2.company_id(), state.company().id());
        assert_eq!(procspec2.active(), &false);
        assert_eq!(procspec2.created(), &now);
        assert_eq!(procspec2.updated(), &now2);
        assert_eq!(procspec2.deleted(), &None);
    }

    #[test]
    fn can_delete() {
        let now = util::time::now();
        let id = ProcessSpecID::create();
        let mut state = TestState::standard(
            vec![
                CompanyPermission::ProcessSpecCreate,
                CompanyPermission::ProcessSpecDelete,
            ],
            &now,
        );
        let mods = create(
            state.user(),
            state.member(),
            state.company(),
            id.clone(),
            "SEIZE THE MEANS OF PRODUCTION",
            "our first process",
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let procspec = mods[0]
            .clone()
            .expect_op::<ProcessSpec>(Op::Create)
            .unwrap();
        state.model = Some(procspec);

        let now2 = util::time::now();
        let testfn = |state: &TestState<ProcessSpec, ProcessSpec>| {
            delete(
                state.user(),
                state.member(),
                state.company(),
                state.model().clone(),
                &now2,
            )
        };
        test::standard_transaction_tests(&state, &testfn);
        test::double_deleted_tester(&state, "process_spec", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let procspec2 = mods[0]
            .clone()
            .expect_op::<ProcessSpec>(Op::Delete)
            .unwrap();
        assert_eq!(procspec2.id(), &id);
        assert_eq!(procspec2.inner().name(), "SEIZE THE MEANS OF PRODUCTION");
        assert_eq!(procspec2.company_id(), state.company().id());
        assert_eq!(procspec2.active(), &true);
        assert_eq!(procspec2.created(), &now);
        assert_eq!(procspec2.updated(), &now);
        assert_eq!(procspec2.deleted(), &Some(now2.clone()));
    }
}
