use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        region::{self, Region, RegionID},
        user::User,
    }
};

pub fn create<T: Into<String>>(user: &User, id: RegionID, name: T, now: &DateTime<Utc>) -> Result<Region> {
    if !user.can(&Permission::RegionCreate) {
        Err(Error::PermissionDenied)?;
    }
    let res = region::builder()
        .id(id)
        .name(name.into())
        .created(now.clone())
        .updated(now.clone())
        .build()
        .map_err(|e| Error::BuilderFailed(e))?;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        access::Role,
        models::{
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
        let region = create(&user, id.clone(), "xina", &now).unwrap();
        assert_eq!(region.id(), &id);
        assert_eq!(region.name(), "xina");
    }
}

