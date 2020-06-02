//! Transactions are the primary interface for interacting with the Basis
//! system. They are responsible for taking the needed information (which must
//! be passed in) and returning a list of modifications that the caller is
//! responsible for applying to whatever storage medium they are using.
//!
//! The high-level picture here is that we're creating a functional API for the
//! models within the system and the interactions between them. The logic all
//! lives in the transactions (and in some cases the models) but storage happens
//! somewhere else and we don't touch it here.
//!
//! This means that any storage system that *can* support the Basis data models
//! could (in theory) be used without needing to couple any of the logic to the
//! storage mechanism.

pub mod region;
pub mod user;
pub mod occupation;
//pub mod currency;
pub mod company;
//pub mod agent;
//pub mod process_spec;
//pub mod process;
//pub mod event;
//pub mod company_member;
//pub mod agreement;
//pub mod account;
//pub mod resource_spec;
//pub mod resource;
//pub mod commitment;

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use crate::{
        access::Role,
        models::{
            user::{User, UserID},
        },
    };

    pub fn make_user(user_id: &UserID, now: &DateTime<Utc>, roles: Option<Vec<Role>>) -> User {
        User::builder()
            .id(user_id.clone())
            .roles(roles.unwrap_or(vec![Role::User]))
            .email("surely@hotmail.com")   // don't call me shirley
            .name("buzzin' frog")
            .active(true)
            .created(now.clone())
            .updated(now.clone())
            .build().unwrap()
    }
}

