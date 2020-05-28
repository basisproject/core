use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid role")]
    InvalidRole,
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
    #[error("event is missing resource quantity measurement")]
    EventMissingResourceQuantity,
    #[error("event missing resource")]
    EventMissingResource,
    #[error("event missing `output_of` process")]
    EventMissingOutputProcess,
    #[error("event missing `input_of` process")]
    EventMissingInputProcess,
    #[error("event creates negative resource amount")]
    EventCreatesNegativeResourceAmount,
    #[error("operation on measurement with mismatched units")]
    MeasureUnitsMismatched,
    #[error("missing measure object")]
    MissingMeasure,
    #[error("operation creates negative costs")]
    NegativeCosts,
    #[error("operation creates negative measurement")]
    NegativeMeasurement,
    #[error("error operating on NumericUnion")]
    NumericUnionOpError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

