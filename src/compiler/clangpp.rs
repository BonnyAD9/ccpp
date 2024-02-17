use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{dependency::Dependency, err::Result};

use super::{common::Compiler, config::Config, gcc, gpp};

pub struct Clangpp {
    bin: PathBuf,
    src_root: PathBuf,
    bin_root: PathBuf,
    compile_args: Vec<String>,
    link_args: Vec<String>,
}

impl Clangpp {
    pub fn build(
        &self,
        file: Dependency,
    ) -> Result<(Command, Vec<Dependency>)> {
        gcc::build(self, file)
    }

    pub fn new(bin: PathBuf, conf: &Config, is_c: bool) -> Result<Self> {
        gpp::try_new(bin, conf, is_c)
    }
}

impl Compiler for Clangpp {
    fn bin(&self) -> &Path {
        &self.bin
    }

    fn src_root(&self) -> &Path {
        &self.src_root
    }

    fn bin_root(&self) -> &Path {
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
