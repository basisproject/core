use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid role")]
    InvalidRole,
}

pub type Result<T> = std::result::Result<T, Error>;

