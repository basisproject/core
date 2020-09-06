//! An occupation is effectively a job title that we want to track in the labor
//! cost tracking.
//!
//! See the [occupation model.][1]
//!
//! [1]: ../../models/occupation/index.html

use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        lib::basis_model::Model,
        occupation::{Occupation, OccupationID},
        user::User,
        Modifications, Op,
    },
};
use chrono::{DateTime, Utc};
use vf_rs::vf;

/// Create a new `Occupation`.
pub fn create<T: Into<String>>(
    caller: &User,
    id: OccupationID,
    label: T,
    note: T,
    active: bool,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::OccupationCreate)?;
    let model = Occupation::builder()
        .id(id)
        .inner(
            vf::AgentRelationshipRole::builder()
                .note(Some(note.into()))
                .role_label(label)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?,
        )
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update an existing `Occupation`
pub fn update(
    caller: &User,
    mut subject: Occupation,
    label: Option<String>,
    note: Option<String>,
    active: Option<bool>,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::OccupationUpdate)?;
    if let Some(label) = label {
        subject.inner_mut().set_role_label(label);
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

/// Delete an `Occupation`
pub fn delete(
    caller: &User,
    mut subject: Occupation,
    now: &DateTime<Utc>,
) -> Result<Modifications> {
    caller.access_check(Permission::OccupationDelete)?;
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("occupation".into()))?;
    }
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{occupation::Occupation, Op},
        util::{
            self,
            test::{self, *},
        },
    };

    #[test]
    fn can_create() {
        let id = OccupationID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.user_mut().set_roles(vec![Role::SuperAdmin]);

        let testfn = |state: &TestState<Occupation, Occupation>| {
            create(
                state.user(),
                id.clone(),
                "machinist",
                "builds things",
                true,
                &now,
            )
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let occupation = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        assert_eq!(occupation.id(), &id);
        assert_eq!(occupation.inner().role_label(), "machinist");
        assert_eq!(occupation.inner().note(), &Some("builds things".into()));
        assert_eq!(occupation.active(), &true);

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let id = OccupationID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.user_mut().set_roles(vec![Role::SuperAdmin]);

        let mods = create(
            state.user(),
            id.clone(),
            "bone spurs in chief",
            "glorious leader",
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let occupation = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        state.model = Some(occupation);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Occupation, Occupation>| {
            update(
                state.user(),
                state.model().clone(),
                Some("coward".into()),
                None,
                None,
                &now2,
            )
        };

        // not truly an update but ok
        let mods = testfn(&state).unwrap().into_vec();
        let occupation2 = mods[0].clone().expect_op::<Occupation>(Op::Update).unwrap();
        assert_eq!(state.model().created(), occupation2.created());
        assert_eq!(occupation2.created(), &now);
        assert_eq!(occupation2.updated(), &now2);
        assert_eq!(occupation2.inner().role_label(), "coward");
        assert_eq!(occupation2.inner().note(), &Some("glorious leader".into()));

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = OccupationID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        state.user_mut().set_roles(vec![Role::SuperAdmin]);

        let mods = create(
            state.user(),
            id.clone(),
            "the best president",
            "false acquisitions",
            true,
            &now,
        )
        .unwrap()
        .into_vec();
        let occupation = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        state.model = Some(occupation);

        let now2 = util::time::now();
        let testfn = |state: &TestState<Occupation, Occupation>| {
            delete(state.user(), state.model().clone(), &now2)
        };
        test::double_deleted_tester(&state, "occupation", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let occupation2 = mods[0].clone().expect_op::<Occupation>(Op::Delete).unwrap();
        assert_eq!(occupation2.id(), &id);
        assert_eq!(occupation2.created(), &now);
        assert_eq!(occupation2.deleted(), &Some(now2));

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}
