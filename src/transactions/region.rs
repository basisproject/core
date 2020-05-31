use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        region::{self, RegionID, Region},
        user::User,
    }
};

/// Create a region
pub fn create<T: Into<String>>(user: &User, id: RegionID, name: T, now: &DateTime<Utc>) -> Result<Modifications> {
    access_check!(user, Permission::RegionCreate);
    let model = region::builder()
        .id(id)
        .name(name.into())
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
}

/// Delete a region
pub fn delete(user: &User, region: Region) -> Result<Modifications> {
    access_check!(user, Permission::RegionDelete);
    Ok(Modifications::new_single(Op::Delete, region))
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
        let id = RegionID::create();
        let now = util::time::now();
        let user = make_user(&now);
        let mods = create(&user, id.clone(), "xina", &now).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);
        match mods[0].clone().into_pair() {
            (Op::Create, Model::Region(region)) => {
                assert_eq!(region.id(), &id);
                assert_eq!(region.name(), "xina");
            }
            _ => panic!("unexpected result"),
        }
    }

    #[test]
    fn can_delete() {
        let id = RegionID::create();
        let now = util::time::now();
        let user = make_user(&now);
        let mods = create(&user, id.clone(), "fine", &now).unwrap().into_modifications();
        let region = Region::try_from(mods[0].clone().into_pair().1).unwrap();
        let mods = delete(&user, region).unwrap().into_modifications();
        assert_eq!(mods.len(), 1);
        match mods[0].clone().into_pair() {
            (Op::Delete, Model::Region(region)) => {
                assert_eq!(region.id(), &id);
            }
            _ => panic!("unexpected result"),
        }
    }
}

