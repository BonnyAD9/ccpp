use std::path::StripPrefixError;

use thiserror::Error;

use crate::arg_parser::ArgError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{}", .0)]
    Generic(String),
    #[error("This is a bug, please report it: {}", .0)]
    DoesNotHappen(&'static str),
    #[error(transparent)]
    Arg(#[from] ArgError),
    #[error(
        "child process exited with code {}",
        if let Some(c) = .0 { *c } else { 1 }
    )]
    ProcessFailed(Option<i32>),
    #[error(transparent)]
    TomlSerError(#[from] toml::ser::Error),
    #[error(transparent)]
    TomlDeError(#[from] toml::de::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    StripPrefixError(#[from] StripPrefixError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
