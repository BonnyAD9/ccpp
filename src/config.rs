use std::path::PathBuf;

use crate::compiler;

pub struct Config {
    pub project: Project,
    pub debug_build: Build,
    pub release_build: Build,
}

pub struct Project {
    pub name: String,
}

pub struct Build {
    pub target: PathBuf,
    pub cc: Option<PathBuf>,
    pub cpp: Option<PathBuf>,
    pub compiler_conf: CompilerConfig,
}

pub type CompilerConfig = compiler::config::Config;
