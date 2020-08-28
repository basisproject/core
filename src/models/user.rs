//! The user is the point of access to the entire system. Users have
//! permissions, users own accounts, users are linked to companies (via
//! `CompanyMember` objects), and much more.
//!
//! Every person in the system (whether they are a member or not) is represented
//! by a `User` object.

use crate::{
    access::{Permission, Role},
    models::{
        lib::{
            agent::{Agent, AgentID},
            basis_model::ActiveState,
        },
    },
    error::{Error, Result},
};

basis_model! {
    /// The `User` model describes a user of the system.
    pub struct User {
        id: <<UserID>>,
        /// Defines this user's roles, ie what permissions they have access to.
        roles: Vec<Role>,
        /// The user's email. Might be best to use a proxy address, since most
        /// of this data will be fairly public.
        email: String,
        /// The user's full name.
        name: String,
    }
    UserBuilder
}

impl User {
    /// Determines if a user can perform an action (base on their roles).
    pub fn can(&self, permission: &Permission) -> bool {
        if !self.is_active() {
            return false;
        }
        for role in self.roles() {
            if role.can(permission) {
                return true;
            }
        }
        false
    }

    /// Check if this user can perform an action.
    pub fn access_check(&self, permission: Permission) -> Result<()> {
        if !self.can(&permission) {
            Err(Error::InsufficientPrivileges)?;
        }
        Ok(())
    }
}

impl Agent for User {
    fn agent_id(&self) -> AgentID {
        self.id().clone().into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        access::{Permission, Role},
        models::{
            user::UserID,
            testutils::make_user,
        },
        util,
    };

    #[test]
    fn permissions() {
        let now = util::time::now();
        let user = make_user(&UserID::create(), None, &now);
        assert!(user.can(&Permission::UserDelete));
        assert!(user.access_check(Permission::UserDelete).is_ok());
        assert!(user.access_check(Permission::CompanyAdminDelete).is_err());

        let user2 = make_user(&UserID::create(), Some(vec![Role::User, Role::CompanyAdmin]), &now);
        assert!(user2.can(&Permission::UserDelete));
        assert!(user2.access_check(Permission::UserDelete).is_ok());
        assert!(user2.access_check(Permission::CompanyAdminDelete).is_ok());

        let user3 = make_user(&UserID::create(), Some(vec![]), &now);
        assert!(!user3.can(&Permission::UserDelete));
        assert!(user3.access_check(Permission::UserDelete).is_err());
        assert!(user3.access_check(Permission::CompanyAdminDelete).is_err());

        let mut user4 = user2.clone();
        user4.set_deleted(Some(now.clone()));
        assert!(!user4.can(&Permission::UserDelete));
        assert!(user4.access_check(Permission::UserDelete).is_err());
        assert!(user4.access_check(Permission::CompanyAdminDelete).is_err());

        let mut user5 = user2.clone();
        user5.set_active(false);
        assert!(!user5.can(&Permission::UserDelete));
        assert!(user5.access_check(Permission::UserDelete).is_err());
        assert!(user5.access_check(Permission::CompanyAdminDelete).is_err());
    }
}

