//! The user is the point of access to the entire system. User's have
//! permissions, users own accounts, users are linked to companies (via
//! `CompanyMember` objects), and much more.
//!
//! Every person in the system (whether they are a member or not) is represented
//! by a `User` object.

use crate::{
    access::{Permission, Role},
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

