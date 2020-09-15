//! The access module defines the various top-level permissions within the
//! system and the roles that contain those permissions.
//!
//! Roles can have multiple Permission objects. Permissions are additive,
//! meaning everyone starts with *no* permissions (returning
//! [Error::InsufficientPrivileges][err_priv]) and permissions are added
//! (allowed) from there.
//!
//! Generally, the access system just applies to [Users].
//!
//! [err_priv]: ../error/enum.Error.html#variant.InsufficientPrivileges
//! [Users]: ../models/user/struct.User.html

use crate::{
    error::{Error, Result},
};
use serde::{Serialize, Deserialize};

/// Define the system-wide permissions.
///
/// Note there may be per-model permissions that are handled separately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    All,
    AllBut(Vec<Permission>),

    AccountCreate,
    AccountDelete,
    AccountSetOwners,
    AccountTransfer,
    AccountUpdate,

    CompanyAdminDelete,
    CompanyAdminUpdate,
    CompanyCreate,
    CompanyDelete,
    CompanyPayroll,
    CompanyUpdate,
    CompanyUpdateAgreements,
    CompanyUpdateCommitments,
    CompanyUpdateIntents,
    CompanyUpdateMembers,
    CompanyUpdateResources,
    CompanyUpdateResourceSpecs,
    CompanyUpdateProcesses,
    CompanyUpdateProcessSpecs,

    CurrencyCreate,
    CurrencyDelete,
    CurrencyUpdate,

    EventCreate,
    EventUpdate,

    UserAdminCreate,
    UserAdminUpdate,
    UserCreate,
    UserDelete,
    UserSetRoles,
    UserUpdate,

    ResourceSpecCreate,
    ResourceSpecDelete,
    ResourceSpecUpdate,

    OccupationCreate,
    OccupationDelete,
    OccupationUpdate,
}

/// Define the system-wide roles users can have.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Role {
    SuperAdmin,
    IdentityAdmin,
    CompanyAdmin,
    Bank,
    User,
    Guest,
}

impl Role {
    /// For a given role, return the permissions that role has access to.
    pub fn permissions(&self) -> Vec<Permission> {
        match *self {
            Role::SuperAdmin => {
                vec![Permission::All]
            },
            Role::IdentityAdmin => {
                vec![
                    Permission::UserUpdate,
                    Permission::UserSetRoles,
                    Permission::UserAdminCreate,
                    Permission::UserAdminUpdate,
                    Permission::UserDelete,
                ]
            },
            Role::CompanyAdmin => {
                vec![
                    Permission::CompanyAdminUpdate,
                    Permission::CompanyAdminDelete,
                ]
            }
            Role::Bank => {
                vec![
                    Permission::CurrencyCreate,
                    Permission::CurrencyUpdate,
                    Permission::CurrencyDelete,
                ]
            },
            Role::User => {
                vec![
                    Permission::UserUpdate,
                    Permission::UserDelete,
                    Permission::CompanyCreate,
                    Permission::CompanyDelete,
                    Permission::CompanyPayroll,     // hey, milton. what's happening.
                    Permission::CompanyUpdate,
                    Permission::CompanyUpdateAgreements,
                    Permission::CompanyUpdateCommitments,
                    Permission::CompanyUpdateIntents,
                    Permission::CompanyUpdateMembers,
                    Permission::CompanyUpdateResourceSpecs,
                    Permission::CompanyUpdateResources,
                    Permission::CompanyUpdateProcessSpecs,
                    Permission::CompanyUpdateProcesses,
                    Permission::ResourceSpecCreate,
                    Permission::ResourceSpecUpdate,
                    Permission::ResourceSpecDelete,
                    Permission::AccountCreate,
                    Permission::AccountUpdate,
                    Permission::AccountSetOwners,
                    Permission::AccountTransfer,
                    Permission::AccountDelete,
                    Permission::EventCreate,
                    Permission::EventUpdate,
                ]
            }
            Role::Guest => {
                vec![
                    Permission::UserCreate,
                ]
            }
        }
    }

    /// Determine if a role has a specific permission.
    pub fn can(&self, perm: &Permission) -> bool {
        for p in &self.permissions() {
            match p {
                Permission::All => {
                    return true;
                }
                Permission::AllBut(x) => {
                    if x.contains(perm) {
                        return false;
                    }
                    return true;
                }
                _ => {
                    if p == perm {
                        return true
                    }
                }
            }
        }
        false
    }
}

/// Check if a guest can perform an action.
pub fn guest_check(perm: Permission) -> Result<()> {
    if (Role::Guest).can(&perm) {
        Ok(())
    } else {
        Err(Error::InsufficientPrivileges)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn permissions_work() {
        let super_admin = Role::SuperAdmin;
        assert!(super_admin.can(&Permission::All));
        assert!(super_admin.can(&Permission::UserCreate));
        assert!(super_admin.can(&Permission::UserUpdate));
        assert!(super_admin.can(&Permission::UserAdminUpdate));
        assert!(super_admin.can(&Permission::UserDelete));
        assert!(super_admin.can(&Permission::CompanyCreate));
        assert!(super_admin.can(&Permission::CompanyAdminUpdate));
        assert!(super_admin.can(&Permission::CompanyAdminDelete));

        let comp_admin = Role::CompanyAdmin;
        assert!(!comp_admin.can(&Permission::UserCreate));
        assert!(!comp_admin.can(&Permission::UserUpdate));
        assert!(!comp_admin.can(&Permission::UserAdminUpdate));
        assert!(!comp_admin.can(&Permission::UserDelete));
        assert!(!comp_admin.can(&Permission::CompanyCreate));
        assert!(comp_admin.can(&Permission::CompanyAdminUpdate));
        assert!(comp_admin.can(&Permission::CompanyAdminDelete));
    }
}

