use std::path::StripPrefixError;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "child process exited with code {}",
        if let Some(c) = .0 { *c } else { 1 }
    )]
    ProcessFailed(Option<i32>),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    StripPrefixError(#[from] StripPrefixError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
