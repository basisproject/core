use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid role")]
    InvalidRole,
    #[error("AgentID is the wrong type")]
    WrongAgentIDType,
    #[error("event is missing resource quantity measurement")]
    EventMissingResourceQuantity,
    #[error("event is missing resource quantity measurement")]
    EventMismatchedMeasureUnits,
    #[error("event missing resource")]
    EventMissingResource,
    #[error("event missing `input_of` process")]
    EventMissingInputProcess,
    #[error("event missing `output_of` process")]
    EventMissingOutputProcess,
    #[error("error operating on NumericUnion")]
    NumericUnionOpError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

