//! A user represents a person with membership in the system.
//!
//! See the [user model][1].
//!
//! [1]: ../../models/user/index.html

use chrono::{DateTime, Utc};
use crate::{
    access::{self, Permission, Role},
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        lib::basis_model::Model,
        user::{User, UserID},
    },
};

/// Create a user (private implementation, meant to be wrapped).
fn create_inner<T: Into<String>>(id: UserID, roles: Vec<Role>, email: T, name: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
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

/// Create a new user with a `Role::User` role. No permissions required.
pub fn create<T: Into<String>>(id: UserID, email: T, name: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    access::guest_check(Permission::UserCreate)?;
    create_inner(id, vec![Role::User], email, name, active, now)
}

/// Create a new user with a specific set of permissions using a current user as
/// the originator. Effectively an admin create. Requires the 
/// `Permission::UserCreate` permission.
pub fn create_permissioned<T: Into<String>>(caller: &User, id: UserID, roles: Vec<Role>, email: T, name: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    caller.access_check(Permission::UserAdminCreate)?;
    create_inner(id, roles, email, name, active, now)
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
    if subject.is_deleted() {
        Err(Error::ObjectIsDeleted("user".into()))?;
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
            user::User,
        },
        util::{self, test::{self, *}},
    };

    #[test]
    fn can_create() {
        let id = UserID::create();
        let now = util::time::now();
        let mods = create(id.clone(), "zing@lyonbros.com", "leonard", true, &now).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let model = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
        assert_eq!(model.id(), &id);
        assert_eq!(model.email(), "zing@lyonbros.com");
        assert_eq!(model.name(), "leonard");
        assert_eq!(model.active(), &true);
    }

    #[test]
    fn can_create_permissioned() {
        let id = UserID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let user = make_user(&id, Some(vec![Role::IdentityAdmin]), &now);
        state.user = Some(user);

        let testfn = |state: &TestState<User, User>| {
            create_permissioned(state.user(), id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now)
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let model = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
        assert_eq!(model.id(), &id);
        assert_eq!(model.email(), "zing@lyonbros.com");
        assert_eq!(model.name(), "leonard");
        assert_eq!(model.active(), &true);

        let mut state2 = state.clone();
        state2.user_mut().set_roles(vec![Role::User]);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_update() {
        let id = UserID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let user = make_user(&id, Some(vec![Role::IdentityAdmin]), &now);
        let mods = create_permissioned(&user, id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now).unwrap().into_vec();
        let new_user = mods[0].clone().expect_op::<User>(Op::Create).unwrap();
        state.user = Some(user);
        state.model = Some(new_user);

        let now2 = util::time::now();
        let testfn_inner = |state: &TestState<User, User>, active: Option<bool>| {
            update(state.user(), state.model().clone(), Some("obvious_day@camp.stupid".into()), None, active, &now2)
        };
        let testfn = |state: &TestState<User, User>| {
            testfn_inner(state, None)
        };

        let mods = testfn(&state).unwrap().into_vec();
        let user2 = mods[0].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(user2.email(), "obvious_day@camp.stupid");
        assert_eq!(user2.name(), "leonard");
        assert_eq!(user2.active(), &true);
        assert_eq!(user2.updated(), &now2);

        let mut state2 = state.clone();
        state2.user = Some(user2.clone());
        state2.model = Some(user2);
        let mods = testfn_inner(&state2, Some(false)).unwrap().into_vec();
        let user3 = mods[0].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(user3.email(), "obvious_day@camp.stupid");
        assert_eq!(user3.name(), "leonard");
        assert_eq!(user3.active(), &false);
        assert_eq!(user3.updated(), &now2);

        let mut state3 = state.clone();
        state3.user = Some(user3.clone());
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_set_roles() {
        let id = UserID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let user = make_user(&id, Some(vec![Role::IdentityAdmin]), &now);
        state.user = Some(user.clone());
        state.model = Some(user);

        let now2 = util::time::now();
        let testfn = |state: &TestState<User, User>| {
            set_roles(state.user(), state.model().clone(), vec![Role::User], &now2)
        };

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);

        let user2 = mods[0].clone().expect_op::<User>(Op::Update).unwrap();
        assert_eq!(user2.id(), &id);
        assert_eq!(user2.roles(), &vec![Role::User]);
        assert_eq!(user2.updated(), &now2);

        // the user changed their roles to not allow setting roles, so when they
        // try to set their roles back to identity admin it shuould fail lol
        // sucker.
        let mut state2 = state.clone();
        state2.user = Some(user2.clone());
        state2.model = Some(user2);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));

        // inactive users should not be able to run mods
        let mut state3 = state.clone();
        state3.user_mut().set_active(false);
        let res = testfn(&state3);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }

    #[test]
    fn can_delete() {
        let id = UserID::create();
        let now = util::time::now();
        let mut state = TestState::standard(vec![], &now);
        let user = make_user(&id, Some(vec![Role::IdentityAdmin]), &now);
        state.user = Some(user.clone());
        state.model = Some(user);

        let testfn = |state: &TestState<User, User>| {
            delete(state.user(), state.model().clone(), &now)
        };
        test::double_deleted_tester(&state, "user", &testfn);

        let mods = testfn(&state).unwrap().into_vec();
        assert_eq!(mods.len(), 1);
        let user2 = mods[0].clone().expect_op::<User>(Op::Delete).unwrap();
        assert_eq!(user2.deleted(), &Some(now.clone()));

        let mut state2 = state.clone();
        state2.user = Some(user2.clone());
        state2.model = Some(user2);
        let res = testfn(&state2);
        assert_eq!(res, Err(Error::InsufficientPrivileges));
    }
}

