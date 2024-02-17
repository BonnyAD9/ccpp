use std::path::{PathBuf, StripPrefixError};

use thiserror::Error;

use crate::{arg_parser::ArgError, dependency::DepFile};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Cannot build the target, two or more intermidiate targets depend on \
        each other in cycle"
    )]
    DependencyCycle,
    #[error(
        "The given file has inconsitent dependencies. Cannot create \
        dependency twice for the same file."
    )]
    DuplicateDependency,
    #[error(
        "Cannot build file {} because it has no files to be build from",
        .0.to_string_lossy()
    )]
    NothingToBuild(PathBuf),
    #[error(
        "Invalid/unknown file type '{:?}' of file '{}'",
        .0.typ,
        .0.path.to_string_lossy()
    )]
    InvalidFileType(DepFile),
    #[error("Invalid value `{value}` for {option} in compiler option.")]
    InvalidCompilerValue { option: String, value: String },
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
