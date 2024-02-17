use std::{path::PathBuf, process::Command};

use crate::{compiler::common::Compiler, dependency::Dependency, err::Result};

use super::{config::Config, gcc};

pub struct Clang {
    bin: PathBuf,
    src_root: PathBuf,
    bin_root: PathBuf,
    compile_args: Vec<String>,
    link_args: Vec<String>,
}

impl Clang {
    pub fn build(
        &self,
        file: Dependency,
    ) -> Result<(Command, Vec<Dependency>)> {
        gcc::build(self, file)
    }

    pub fn new(bin: PathBuf, conf: &Config) -> Result<Self> {
        gcc::try_new(bin, conf)
    }
}

impl Compiler for Clang {
    fn bin(&self) -> &std::path::Path {
        &self.bin
    }

    fn src_root(&self) -> &std::path::Path {
        &self.src_root
    }

    fn bin_root(&self) -> &std::path::Path {
        &self.bin_root
    }

    fn compile_args(&self) -> &Vec<String> {
        &self.compile_args
    }

    fn link_args(&self) -> &Vec<String> {
        &self.link_args
    }

    fn try_new(
        bin: PathBuf,
        compile_args: Vec<String>,
        link_args: Vec<String>,
        conf: &Config,
    ) -> Result<Self> {
        Ok(Self {
            bin,
            src_root: conf.src_root.clone(),
            bin_root: conf.bin_root.clone(),
            compile_args,
            link_args,
        })
    }
}
