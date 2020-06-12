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

use thiserror::Error;

/// An error type for when transaction processing goes awry.
#[derive(Error, Debug, PartialEq)]
pub enum TransactionError {
    /// Event is finalized already
    #[error("this event is already finalized")]
    EventAlreadyFinalized,
    /// A work event needs a cost parameter but it wasn't given
    #[error("work even missing costs parameter")]
    MissingWorkCosts,
}

//pub mod account;
//pub mod agreement;
//pub mod commitment;
pub mod company;
pub mod company_member;
//pub mod currency;
pub mod event;
//pub mod intent;
pub mod occupation;
pub mod process;
pub mod process_spec;
pub mod region;
pub mod resource;
pub mod resource_spec;
pub mod user;

