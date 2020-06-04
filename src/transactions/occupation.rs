use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        occupation::{Occupation, OccupationID},
        user::User,
    },
};
use vf_rs::vf;

/// Create a new occupation
pub fn create<T: Into<String>>(caller: &User, id: OccupationID, label: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::OccupationCreate)?;
    let model = Occupation::builder()
        .id(id)
        .inner(
            vf::AgentRelationshipRole::builder()
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
pub fn update(caller: &User, mut subject: Occupation, label: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::OccupationUpdate)?;
    if let Some(label) = label {
        subject.inner_mut().set_role_label(label);
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
        },
        transactions::tests::make_user,
        util,
    };

    #[test]
    fn can_create() {
        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), &now, Some(vec![Role::SuperAdmin]));
        let mods = create(&user, id.clone(), "machinist", true, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let model = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        assert_eq!(model.id(), &id);
        assert_eq!(model.inner().role_label(), "machinist");

        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), &now, Some(vec![Role::User]));

        let res = create(&user, id.clone(), "dog psychic", true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), &now, Some(vec![Role::SuperAdmin]));
        let mods = create(&user, id.clone(), "bone spurs in chief", true, &now).unwrap().into_modifications();

        let subject = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        assert_eq!(subject.inner().role_label(), "bone spurs in chief");
        assert_eq!(subject.active(), &true);

        let now2 = util::time::now();
        // not truly an update but ok
        let mods = update(&user, subject.clone(), Some("coward".into()), None, &now2).unwrap().into_modifications();
        let subject2 = mods[0].clone().expect_op::<Occupation>(Op::Update).unwrap();
        assert_eq!(subject.created(), subject2.created());
        assert_eq!(subject2.created(), &now);
        assert_eq!(subject2.updated(), &now2);
        assert_eq!(subject2.inner().role_label(), "coward");

        let user = make_user(&UserID::create(), &now, None);
        let res = update(&user, subject.clone(), Some("the best president the best president the best president president unpresidented FALSE ACQUISITIONS".into()), None, &now2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = OccupationID::create();
        let now = util::time::now();
        let user = make_user(&UserID::create(), &now, Some(vec![Role::SuperAdmin]));
        let mods = create(&user, id.clone(), "the best president", true, &now).unwrap().into_modifications();
        let subject = mods[0].clone().expect_op::<Occupation>(Op::Create).unwrap();
        let mods = delete(&user, subject.clone(), &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let subject2 = mods[0].clone().expect_op::<Occupation>(Op::Delete).unwrap();
        assert_eq!(subject2.id(), &id);

        let user = make_user(&UserID::create(), &now, None);
        let res = delete(&user, subject2, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

