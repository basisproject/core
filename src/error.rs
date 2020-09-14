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
    /// When we try to perform an operation that would erase costs (such as
    /// trying to delete a Process that has non-zero costs).
    #[error("cannot erase costs")]
    CannotEraseCosts,
    /// When trying to erase credits from the system. Generally this happens if
    /// you try to delete an account that has a non-zero balance.
    #[error("cannot erase credits")]
    CannotEraseCredits,
    /// When you try to do something that requires a commitment but the given
    /// commitment doesn't match the action being performed.
    #[error("commitment is invalid")]
    CommitmentInvalid,
    /// When trying to move a set of Costs from a Costs object where all values
    /// of the moving Cost to not have the same proportion to the original cost.
    #[error("costs being moved are not proportional")]
    CostsNotProportional,
    /// An error while processing an event.
    #[error("event error {0:?}")]
    Event(#[from] EventError),
    /// You don't have permission to perform this action
    #[error("insufficient privileges")]
    InsufficientPrivileges,
    /// We get this when trying to pull a measure out of a resource and come up
    /// blank, for instance when using `consume` on a resource that hasn't had
    /// its quantities initialized via `produce`/`raise`/`transfer`/etc.
    #[error("a resource measurement (account/onhand quantity) is missing")]
    ResourceMeasureMissing,
    /// Happens when trying to operate on two `Measure` objects with different
    /// units, such as adding 12 Hours to 16 Kilograms
    #[error("operation on measurement with mismatched units")]
    MeasureUnitsMismatched,
    /// The given `Member` must be a `MemberWorker` class
    #[error("the member given must be a worker (not company, user, etc)")]
    MemberMustBeWorker,
    /// We're missing required fields in a call
    #[error("fields missing {0:?}")]
    MissingFields(Vec<String>),
    /// An account cannot have a negative balance
    #[error("operation creates negative account balance")]
    NegativeAccountBalance,
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
    /// When we try to update an object that has been deleted (or an object
    /// attached to the deleted object).
    #[error("object {0} is deleted")]
    ObjectIsDeleted(String),
    /// When we try to update an object that is inactive (or an object attached
    /// to the inactive object).
    #[error("object {0} is inactive")]
    ObjectIsInactive(String),
    /// When we try to modify an object that is now in a read-only state.
    #[error("object {0} is read-only")]
    ObjectIsReadOnly(String),
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

