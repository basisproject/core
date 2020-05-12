use crate::access::{Permission, Role};

basis_model! {
    pub struct User {
        #[builder(default)]
        roles: Vec<Role>,
        email: String,
        name: String,
    }
    UserBuilder
}

impl User {
    pub fn can(&self, permission: &Permission) -> bool {
        if !self.is_active() {
            return false;
        }
        for role in &self.roles {
            if role.can(permission) {
                return true;
            }
        }
        false
    }
}

