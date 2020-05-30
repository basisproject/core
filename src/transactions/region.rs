use chrono::{DateTime, Utc};
use crate::{
    access::Permission,
    error::{Error, Result},
    models::{
        region::{Region, RegionID},
        user::User,
    }
};

// TODO:
// TODO:
// trying to figure out how to make transactions the exposed api for "getting
// things done" while at the same time allowing transactions to pass back models
// while ALSO forcing the models to NOT have set_*/get_mut helper functions.
// TODO:
// TODO:
pub fn create<T: Into<String>>(user: &User, id: RegionID, name: T, now: &DateTime<Utc>) -> Result<Region> {
    if !user.can(&Permission::RegionCreate) {
        Err(Error::PermissionDenied)?;
    }
    let res = Region::builder()
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
        util,
    };

    #[test]
    fn can_create() {
        let id = RegionID::create();
        let now = util::time::now();
        let user = User::builder()
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

