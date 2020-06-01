//! The main error enum for the project lives here. The goal is to be somewhat
//! verbose with our errors.

use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("AgentID is the wrong type")]
    WrongAgentIDType,
    #[error("event `output_of` process does not match ID")]
    EventMismatchedOutputProcessID,
    #[error("event `input_of` process does not match ID")]
    EventMismatchedInputProcessID,
    #[error("event `resource` object does not match ID")]
    EventMismatchedResourceID,
    #[error("event `provider` object does not match ID")]
    EventMismatchedProviderID,
    #[error("event is missing costs")]
    EventMissingCosts,
    #[error("event is missing `labor_type`")]
    EventMissingLaborType,
    #[error("event is missing resource quantity measurement")]
    EventMissingResourceQuantity,
    #[error("event missing resource")]
    EventMissingResource,
    #[error("event missing `output_of` process")]
    EventMissingOutputProcess,
    #[error("event missing `input_of` process")]
    EventMissingInputProcess,
    #[error("event missing `provider` member")]
    EventMissingProvider,
    #[error("event missing `transfer_type`")]
    EventMissingTransferType,
    #[error("event creates negative resource amount")]
    EventCreatesNegativeResourceAmount,
    #[error("event labor effort must be recorded in hours")]
    EventLaborMustBeHours,
    #[error("operation on measurement with mismatched units")]
    MeasureUnitsMismatched,
    #[error("missing measure object")]
    MissingMeasure,
    #[error("operation creates negative costs")]
    NegativeCosts,
    #[error("operation creates negative measurement")]
    NegativeMeasurement,
    #[error("error operating on NumericUnion: {0}")]
    NumericUnionOpError(String),
    #[error("insufficient privileges")]
    InsufficientPrivileges,
    #[error("error building object {0}")]
    BuilderFailed(String),
    #[error("error converting Model to its inner form")]
    ModelConvertError,
    #[error("Op does not match expectation")]
    OpMismatch,
    #[error("modification missing at given index")]
    ModificationIndex,
}

pub type Result<T> = std::result::Result<T, Error>;

