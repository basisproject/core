use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid role")]
    InvalidRole,

    #[error("missing product in costing data")]
    CostMissingProduct,

    #[error("missing tag in costing data")]
    CostMissingTag,
}

pub type Result<T> = std::result::Result<T, Error>;

