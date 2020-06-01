use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        region::{self, RegionID, Region},
        user::User,
    },
};

/// Create a region
pub fn create<T: Into<String>>(caller: &User, id: RegionID, name: T, active: bool, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::RegionCreate);
    let model = region::builder()
        .id(id)
        .name(name.into())
        .active(active)
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Delete a region
pub fn delete(caller: &User, mut subject: Region, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(caller, Permission::RegionDelete);
    region::set::deleted(&mut subject, Some(now.clone()));
    Ok(Modifications::new_single(Op::Delete, subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
            Op,

            region::{Region},
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
        let id = RegionID::create();
        let now = util::time::now();
        let user = make_user(&now);
        let mods = create(&user, id.clone(), "xina", true, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let model = mods[0].clone().expect_op::<Region>(Op::Create).unwrap();
        assert_eq!(model.id(), &id);
        assert_eq!(model.name(), "xina");

        let id = RegionID::create();
        let now = util::time::now();
        let mut user = make_user(&now);
        user::set::roles(&mut user, vec![Role::User]);

        let res = create(&user, id.clone(), "xina", true, &now);
        assert_eq!(res, Err(Error::PermissionDenied));
    }

    #[test]
    fn can_delete() {
        let id = RegionID::create();
        let now = util::time::now();
        let mut user = make_user(&now);
        let mods = create(&user, id.clone(), "fine", true, &now).unwrap().into_modifications();
        let region = mods[0].clone().expect_op::<Region>(Op::Create).unwrap();
        let mods = delete(&user, region, &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);

        let model = mods[0].clone().expect_op::<Region>(Op::Delete).unwrap();
        assert_eq!(model.id(), &id);

        let mods = create(&user, id.clone(), "fine", true, &now).unwrap().into_modifications();
        let region = Region::try_from(mods[0].clone().into_pair().1).unwrap();
        user::set::active(&mut user, false);
        let res = delete(&user, region, &now);
        assert_eq!(res, Err(Error::PermissionDenied));
    }
}

