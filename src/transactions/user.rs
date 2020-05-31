use chrono::{DateTime, Utc};
use crate::{
    access::{Permission, Role},
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        user::{self, User, UserID},
    },
};

/// Create a user
pub fn create<T: Into<String>>(caller: &User, id: UserID, roles: Vec<Role>, email: T, name: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::UserCreate);
    let model = user::builder()
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

/// Update a user's roles
pub fn set_roles(caller: &User, mut subject: User, roles: Vec<Role>, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::UserSetRoles);
    user::set::roles(&mut subject, roles);
    user::set::updated(&mut subject, now.clone());
    Ok(Modifications::new_single(Op::Update, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
            Model,

            user,
        },
        util,
    };
    use std::convert::TryFrom;

    fn make_user(now: &DateTime<Utc>) -> User {
        user::builder()
            .id("52221")
            .roles(vec![Role::SuperAdmin])
            .email("surely@hotmail.com")   // don't call me shirley
            .name("buzzin' frog")
            .active(true)
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap()
    }

    #[test]
    fn can_create() {
        let id = UserID::create();
        let now = util::time::now();
        let user = make_user(&now);
        let mods = create(&user, id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);
        match mods[0].clone().into_pair() {
            (Op::Create, Model::User(model)) => {
                assert_eq!(model.id(), &id);
                assert_eq!(model.email(), "zing@lyonbros.com");
                assert_eq!(model.name(), "leonard");
                assert_eq!(model.active(), &true);
            }
            _ => panic!("unexpected result"),
        }

        let id = UserID::create();
        let now = util::time::now();
        let mut user = make_user(&now);
        user::set::roles(&mut user, vec![Role::User]);
        let mods = create(&user, id.clone(), vec![Role::User], "zing@lyonbros.com", "leonard", true, &now);
        match mods {
            Err(Error::PermissionDenied) => {}
            _ => panic!("should have failed"),
        }
    }
}

