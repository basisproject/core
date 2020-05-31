use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        Op,
        Modifications,
        region::{self, RegionID},
        user::User,
    }
};

/// Create a region model
pub fn create<T: Into<String>>(user: &User, id: RegionID, name: T, now: &DateTime<Utc>) -> Result<Modifications> {
    if !user.can(&Permission::RegionCreate) {
        Err(Error::PermissionDenied)?;
    }
    let model = region::builder()
        .id(id)
        .name(name.into())
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(Modifications::new_single(Op::Create, model))
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

    #[test]
    fn can_create() {
        let id = RegionID::create();
        let now = util::time::now();
        let user = user::builder()
            .id("52221")
            .roles(vec![Role::SuperAdmin])
            .email("surely@hotmail.com")   // don't call me shirley
            .name("buzzin' frog")
            .active(true)
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap();
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
}

