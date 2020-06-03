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

use serde::{Serialize, Deserialize};

/// Define the system-wide permissions.
///
/// Note there may be per-model permissions that are handled separately.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    All,
    AllBut(Vec<Permission>),

    /// Allows a person to alter the fabric of time. Useful for setting
    /// arbitrary dates on transactions, mainly for testing.
    TimeTravel,

    RegionCreate,
    RegionUpdate,
    RegionDelete,

    UserCreate,
    UserUpdate,
    UserSetRoles,
    UserAdminUpdate,
    UserDelete,

    CompanyCreatePrivate,
    CompanyAdminUpdate,
    CompanyAdminDelete,
    CompanySetType,
    CompanyUpdateMembers,
    CompanyClockIn,
    CompanyClockOut,
    CompanySetLaborWage,
    CompanyAdminClock,

    ResourceSpecCreate,
    ResourceSpecUpdate,
    ResourceSpecDelete,
    ResourceSpecAdminUpdate,
    ResourceSpecAdminDelete,

    OccupationCreate,
    OccupationUpdate,
    OccupationDelete,

    AccountCreate,
    AccountUpdate,
    AccountDelete,
}

/// Define the system-wide roles users can have.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Role {
    SuperAdmin,
    TimeTraveler,
    IdentityAdmin,
    CompanyAdmin,
    ResourceSpecAdmin,
    Bank,
    User,
}

impl Role {
    /// For a given role, return the permissions that role has access to.
    pub fn permissions(&self) -> Vec<Permission> {
        match *self {
            Role::SuperAdmin => {
                vec![
                    Permission::AllBut(vec![Permission::TimeTravel]),
                ]
            },
            Role::TimeTraveler => {
                vec![Permission::TimeTravel]
            },
            Role::IdentityAdmin => {
                vec![
                    Permission::UserCreate,
                    Permission::UserUpdate,
                    Permission::UserSetRoles,
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
            Role::ResourceSpecAdmin => {
                vec![
                    Permission::ResourceSpecAdminUpdate,
                    Permission::ResourceSpecAdminDelete,
                ]
            }
            Role::Bank => {
                vec![
                    Permission::CompanySetType,
                ]
            },
            Role::User => {
                vec![
                    Permission::UserUpdate,
                    Permission::UserDelete,
                    Permission::CompanyCreatePrivate,
                    Permission::CompanyUpdateMembers,
                    Permission::CompanyClockIn,
                    Permission::CompanyClockOut,
                    Permission::CompanySetLaborWage,
                    Permission::ResourceSpecCreate,
                    Permission::ResourceSpecUpdate,
                    Permission::ResourceSpecDelete,
                    Permission::AccountCreate,
                    Permission::AccountUpdate,
                    Permission::AccountDelete,
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

/// This macro is so I don't have to create an Access trait with a `can` fn that
/// User and CompanyMember implement. Just being lazy.
macro_rules! access_check {
    ($model:expr, $perm:expr) => {
        if $model.can(&$perm) {
            Ok(())
        } else {
            Err(Error::InsufficientPrivileges)
        }
    };
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn permissions_work() {
        let super_admin = Role::SuperAdmin;
        assert!(super_admin.can(&Permission::All));
        assert!(!super_admin.can(&Permission::TimeTravel));
        assert!(super_admin.can(&Permission::UserCreate));
        assert!(super_admin.can(&Permission::UserUpdate));
        assert!(super_admin.can(&Permission::UserAdminUpdate));
        assert!(super_admin.can(&Permission::UserDelete));
        assert!(super_admin.can(&Permission::CompanyCreatePrivate));
        assert!(super_admin.can(&Permission::CompanyAdminUpdate));
        assert!(super_admin.can(&Permission::CompanyAdminDelete));
        assert!(super_admin.can(&Permission::CompanySetType));

        let traveller = Role::TimeTraveler;
        assert!(traveller.can(&Permission::TimeTravel));
        assert!(!traveller.can(&Permission::UserCreate));
        assert!(!traveller.can(&Permission::UserUpdate));
        assert!(!traveller.can(&Permission::UserAdminUpdate));
        assert!(!traveller.can(&Permission::UserDelete));
        assert!(!traveller.can(&Permission::CompanyCreatePrivate));
        assert!(!traveller.can(&Permission::CompanyAdminUpdate));
        assert!(!traveller.can(&Permission::CompanyAdminDelete));
        assert!(!traveller.can(&Permission::CompanySetType));

        let comp_admin = Role::CompanyAdmin;
        assert!(!comp_admin.can(&Permission::TimeTravel));
        assert!(!comp_admin.can(&Permission::UserCreate));
        assert!(!comp_admin.can(&Permission::UserUpdate));
        assert!(!comp_admin.can(&Permission::UserAdminUpdate));
        assert!(!comp_admin.can(&Permission::UserDelete));
        assert!(!comp_admin.can(&Permission::CompanyCreatePrivate));
        assert!(comp_admin.can(&Permission::CompanyAdminUpdate));
        assert!(comp_admin.can(&Permission::CompanyAdminDelete));
        assert!(!comp_admin.can(&Permission::CompanySetType));

        // TODO: ResourceSpecAdmin
    }
}

