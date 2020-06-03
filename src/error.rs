//! The main error enum for the project lives here, and documents the various
//! conditions that can arise while interacting with the system.

use crate::{
    models::{
        event::EventError,
    },
};
use thiserror::Error;

/// This is our error enum. It contains an entry for any part of the system in
/// which an expectation is not met or a problem occurs.
#[derive(Error, Debug, PartialEq)]
pub enum Error {
    /// There was an error while using a builder (likely an internal error)
    #[error("error building object {0}")]
    BuilderFailed(String),
    /// An error while processing an event. See `models::event::EventError`
    #[error("event error {0:?}")]
    EventError(#[from] EventError),
    /// You don't have permission to perform this action
    #[error("insufficient privileges")]
    InsufficientPrivileges,
    /// Happens when trying to operate on two `Measure` objects with different
    /// units, such as adding 12 Hours to 16 Kilograms
    #[error("operation on measurement with mismatched units")]
    MeasureUnitsMismatched,
    /// Negative costs cannot be created, as they would represent a surplus
    /// (aka profit). Frowned upon here!
    #[error("operation creates negative costs")]
    NegativeCosts,
    /// Negative measurements cannot be created, as you cannot realistically
    /// have -3 widgets.
    #[error("operation creates negative measurement")]
    NegativeMeasurement,
    /// Represents an error that occurs when dealing with a NumericUnion (such
    /// as a conversion error when adding two that have different types).
    #[error("error operating on NumericUnion: {0}")]
    NumericUnionOpError(String),
    /// When we ask a `Modification` for a model but the `Op` we give it doesn't
    /// match expectation.
    #[error("Op does not match expectation")]
    OpMismatch,
    /// When we try to convert an AgentID to another ID type but it fails (like
    /// `let company_id: CompanyID = AgentID::UserID(user_id).try_from()?;`).
    #[error("AgentID is the wrong type")]
    WrongAgentIDType,
    /// Tried to convert `Model` to an inner model type but failed (for instance
    /// `let company: Company = Model::User(user).try_into()?;`)
    #[error("error converting Model to its inner form")]
    WrongModelType,
}

/// Wraps `std::result::Result` around our `Error` enum
pub type Result<T> = std::result::Result<T, Error>;

