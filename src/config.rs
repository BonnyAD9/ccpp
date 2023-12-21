use std::{
    fs::{self, read_to_string},
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::err::Result;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub project: Project,
    pub build: Build,
    pub debug_build: Build,
    pub release_build: Build,
}

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Build {
    pub target: Option<String>,
    pub cc: Option<String>,
    pub ld: Option<String>,
    pub cflags: Option<Vec<String>>,
    pub ldflags: Option<Vec<String>>,
}

impl Config {
    pub fn from_toml_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(toml::from_str::<Self>(&read_to_string(path)?)?)
    }

    pub fn to_toml_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let value = toml::to_string_pretty(self)?;
        fs::write(path, value)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: Project {
                name: "main".to_owned(),
            },
            build: Build {
                target: None,
                cc: "cc".to_owned().into(),
                ld: "cc".to_owned().into(),
                cflags: vec!["-std=c17".into()].into(),
                ldflags: None,
            },
            debug_build: Build {
                target: None,
                cc: None,
                ld: None,
                cflags: vec![
                    "-g".into(),
                    "-O0".into(),
                    "-fsanitize=address".into(),
                    "-Wall".into(),
                ]
                .into(),
                ldflags: vec!["-fsanitize=address".into()].into(),
            },
            release_build: Build {
                target: None,
                cc: None,
                ld: None,
                cflags: vec!["-O3".into()].into(),
                ldflags: None,
            },
        }
    }
}
