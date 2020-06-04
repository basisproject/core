use chrono::{DateTime, Utc};
use crate::{
    access::{Permission, Role},
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        user::{User, UserID},
    },
};

/// Create a user
pub fn create<T: Into<String>>(caller: &User, id: UserID, roles: Vec<Role>, email: T, name: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::UserCreate)?;
    let model = User::builder()
        .id(id)
        .roles(roles)
        .email(email)
        .name(name)
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Update a user object
pub fn update(caller: &User, mut subject: User, email: Option<String>, name: Option<String>, active: Option<bool>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::UserAdminUpdate)
        .or_else(|_| {
            caller.access_check(Permission::UserUpdate)
                .and_then(|_| {
                    if caller.id() == subject.id() {
                        Ok(())
                    } else {
                        Err(Error::InsufficientPrivileges)
                    }
                })
        })?;
    if let Some(email) = email {
        subject.set_email(email);
    }
    if let Some(name) = name {
        subject.set_name(name);
    }
    if let Some(active) = active {
        subject.set_active(active);
    }
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Update a user's roles
pub fn set_roles(caller: &User, mut subject: User, roles: Vec<Role>, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::UserSetRoles)?;
    subject.set_roles(roles);
    subject.set_updated(now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

/// Delete a user
pub fn delete(caller: &User, mut subject: User, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::UserDelete)?;
    subject.set_deleted(Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
            user::User,
        },
        transactions::tests::make_user,
        util,
    };

    #[test]
    fn can_create() {
        let id = UserID::create();
        let now = util::time::now();
        let user = make_user(&id, &now, Some(vec![Role::IdentityAdmin]));
        let mods = create(&user, id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let model = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
        assert_eq!(model.id(), &id);
        assert_eq!(model.email(), "zing@lyonbros.com");
        assert_eq!(model.name(), "leonard");
        assert_eq!(model.active(), &true);

        let id = UserID::create();
        let now = util::time::now();
        let user = make_user(&id, &now, Some(vec![Role::User]));

        let res = create(&user, id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let id = UserID::create();
        let now = util::time::now();
        let user = make_user(&id, &now, Some(vec![Role::IdentityAdmin]));
        let mods = create(&user, id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now).unwrap().into_modifications();

        let subject = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
        assert_eq!(subject.email(), "zing@lyonbros.com");
        assert_eq!(subject.name(), "leonard");
        assert_eq!(subject.active(), &true);

        let mods = update(&user, subject, Some("obvious_day@camp.stupid".into()), None, None, &now).unwrap().into_modifications();
        let subject2 = mods[0].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(subject2.email(), "obvious_day@camp.stupid");
        assert_eq!(subject2.name(), "leonard");
        assert_eq!(subject2.active(), &true);

        let mods = update(&subject2.clone(), subject2, None, None, Some(false), &now).unwrap().into_modifications();
        let subject3 = mods[0].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(subject3.email(), "obvious_day@camp.stupid");
        assert_eq!(subject3.name(), "leonard");
        assert_eq!(subject3.active(), &false);

        let res = update(&subject3.clone(), subject3, None, None, Some(false), &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_set_roles() {
        let id = UserID::create();
        let now = util::time::now();
        let mut user = make_user(&id, &now, Some(vec![Role::IdentityAdmin]));
        user.set_active(false);

        // inactive users should not be able to run mods
        let res = set_roles(&user, user.clone(), vec![Role::IdentityAdmin], &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        // set back to active and continue lol
        user.set_active(true);
        let mods = set_roles(&user, user.clone(), vec![Role::User], &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let user = mods[0].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(user.id(), &id);
        assert_eq!(user.roles(), &vec![Role::User]);

        // the user changed their roles to not allow setting roles, so when they
        // try to set their roles back to identity admin it shuould fail lol
        // sucker.
        match set_roles(&user, user.clone(), vec![Role::IdentityAdmin], &now) {
            Err(Error::InsufficientPrivileges) => {}
            _ => panic!("should have failed"),
        }
    }

    #[test]
    fn can_delete() {
        let id = UserID::create();
        let now = util::time::now();
        let user = make_user(&id, &now, Some(vec![Role::IdentityAdmin]));
        let mods = delete(&user.clone(), user, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let deleted = mods[0].clone().expect_op::<User>(Op::Delete).unwrap();
        assert_eq!(deleted.deleted(), &Some(now.clone()));

        let res = delete(&deleted.clone(), deleted, &now);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

