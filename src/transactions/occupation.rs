//! An occupation is effectively a job title that we want to track in the labor
//! cost tracking.
//!
//! See the [occupation model.][1]
//!
//! [1]: ../../models/occupation/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        lib::basis_model::Model,
        occupation::{Occupation, OccupationID},
        user::User,
    },
};
use vf_rs::vf;

/// Create a new `Occupation`.
pub fn create<T: Into<String>>(caller: &User, id: OccupationID, label: T, note: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::OccupationCreate)?;
    let model = Occupation::builder()
        .id(id)
        .inner(
            vf::AgentRelationshipRole::builder()
                .note(Some(note.into()))
                .role_label(label)
                .build()
                .map_err(|e| Error::BuilderFailed(e))?
        )
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update an existing `Occupation`
pub fn update(caller: &User, mut subject: Occupation, label: Option<String>, note: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
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
pub fn delete(caller: &User, mut subject: Occupation, now: &DateTime<Utc>) -> Result<Modifications> {
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
        models::{
            Op,

            occupation::Occupation,
            user::UserID,
            testutils::make_user,
        },
        util,
    };

    #[test]
    fn can_create() {
        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), Some(vec![Role::SuperAdmin]), &now);
        let mods = create(&user, id.clone(), "machinist", "builds things", true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let model = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        assert_eq!(model.id(), &id);
        assert_eq!(model.inner().role_label(), "machinist");
        assert_eq!(model.inner().note(), &Some("builds things".into()));

        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), Some(vec![Role::User]), &now);

        let res = create(&user, id.clone(), "dog psychic", "i sense that you are angry.", true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), Some(vec![Role::SuperAdmin]), &now);
        let mods = create(&user, id.clone(), "bone spurs in chief", "glorious leader", true, &now).unwrap().into_vec();

        let subject = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        assert_eq!(subject.inner().role_label(), "bone spurs in chief");
        assert_eq!(subject.active(), &true);

        let now2 = util::time::now();
        // not truly an update but ok
        let mods = update(&user, subject.clone(), Some("coward".into()), None, None, &now2).unwrap().into_vec();
        let subject2 = mods[0].clone().expect_op::<Occupation>(Op::Update).unwrap();
        assert_eq!(subject.created(), subject2.created());
        assert_eq!(subject2.created(), &now);
        assert_eq!(subject2.updated(), &now2);
        assert_eq!(subject2.inner().role_label(), "coward");
        assert_eq!(subject2.inner().note(), &Some("glorious leader".into()));

        let user = make_user(&UserID::create(), None, &now);
        let res = update(&user, subject.clone(), Some("the best president the best president the best president president unpresidented FALSE ACQUISITIONS".into()), None, None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), Some(vec![Role::SuperAdmin]), &now);
        let mods = create(&user, id.clone(), "the best president", "false acquisitions", true, &now).unwrap().into_vec();
        let subject = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        let mods = delete(&user, subject.clone(), &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let subject2 = mods[0].clone().expect_op::<Occupation>(Op::Delete).unwrap();
        assert_eq!(subject2.id(), &id);

        let user2 = make_user(&UserID::create(), None, &now);
        let res = delete(&user2, subject2, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        double_deleted_tester!(subject, "occupation", |subject| delete(&user, subject, &now));
    }
}

