use std::{env, fs::create_dir_all, path::PathBuf};

use thiserror::Error;

use crate::err::{Error, Result};

macro_rules! next_arg {
    ($args:ident, $err:expr) => {
        if let Some(arg) = $args.next() {
            arg
        } else {
            return Err($err.into());
        }
    };
}

#[derive(Error, Debug)]
pub enum ArgError {
    #[error("Invalid value `{value}` for argument `{arg}`: {expl}")]
    InvalidValue {
        value: String,
        arg: String,
        expl: &'static str,
    },
    #[error("Missing argument after `{}`", .0)]
    MissingArgument(String),
    #[error("Unknown argument `{}`", .0)]
    UnknownArgument(String),
    #[error("No action specified, use `ccpp help` to show help")]
    #[allow(dead_code)]
    NoAction,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Action {
    None,
    Clean,
    Build,
    Run,
    Help,
    New(PathBuf),
}

#[derive(Debug)]
pub struct Args {
    pub action: Action,
    pub release: bool,
    pub app_args: Vec<String>,
}

impl Args {
    pub fn get() -> Result<Args> {
        let args: Vec<_> = env::args().collect();
        let mut args = args.iter().map(|a| a.as_str());
        args.next();
        Self::parse(args)
    }

    pub fn parse<'a, I>(mut args: I) -> Result<Args>
    where
        I: Iterator<Item = &'a str>,
    {
        let mut res = Args::default();

        while let Some(arg) = args.next() {
            match arg {
                "clean" => res.action = Action::Clean,
                "build" => res.action = Action::Build,
                "run" => res.action = Action::Run,
                "help" | "h" | "-h" | "-?" | "--help" => {
                    res.action = Action::Help
                }
                "new" => {
                    let value = next_arg!(
                        args,
                        ArgError::MissingArgument(arg.to_owned())
                    );
                    let folder: PathBuf = value.into();
                    if folder.exists() && !folder.is_dir() {
                        return Err(ArgError::InvalidValue {
                            value: value.into(),
                            arg: arg.into(),
                            expl: "Expected directory",
                        }
                        .into());
                    }
                    if !folder.exists() {
                        create_dir_all(&folder)?;
                        res.action = Action::New(folder);
                    } else {
                        let folder = folder.canonicalize()?;
                        res.action = Action::New(folder);
                    }
                }
                "-r" | "--release" => res.release = true,
                "--" => {
                    res.app_args.extend(args.map(|a| a.to_owned()));
                    break;
                }
                _ => {
                    return Err(Error::Arg(ArgError::UnknownArgument(
                        arg.to_owned(),
                    )))
                }
            }
        }

        if res.action == Action::None {
            #[cfg(not(debug_assertions))]
            {
                Err(Error::Arg(ArgError::NoAction))
            }
            #[cfg(debug_assertions)]
            {
                Ok(res)
            }
        } else {
            Ok(res)
        }
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            action: Action::None,
            release: false,
            app_args: vec![],
        }
    }
}
