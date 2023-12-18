use std::env;

use thiserror::Error;

use crate::err::{Error, Result};

#[derive(Error, Debug)]
pub enum ArgError {
    #[error("Unknown argument `{}`", .0)]
    UnknownArgument(String),
    #[error("No action specified, use `ccpp help` to show help")]
    NoAction,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Action {
    None,
    Clean,
    Build,
    Run,
    Help,
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
                "-r" | "--release" => res.release = true,
                "--" => {
                    res.app_args.extend(args.map(|a| a.to_owned()));
                    break;
                },
                _ => {
                    return Err(Error::Arg(ArgError::UnknownArgument(
                        arg.to_owned(),
                    )))
                }
            }
        }

        if res.action == Action::None {
            Err(Error::Arg(ArgError::NoAction))
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
