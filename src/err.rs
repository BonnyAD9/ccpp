use std::path::StripPrefixError;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    StripPrefixError(#[from] StripPrefixError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
