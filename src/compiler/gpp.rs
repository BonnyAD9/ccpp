use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    dependency::Dependency,
    err::{Error, Result},
};

use super::{
    common::Compiler,
    config::{Config, Optimization, Std},
    gcc,
};

pub struct Gpp {
    bin: PathBuf,
    src_root: PathBuf,
    bin_root: PathBuf,
    compile_args: Vec<String>,
    link_args: Vec<String>,
}

impl Gpp {
    pub fn build(
        &self,
        file: Dependency,
    ) -> Result<(Command, Vec<Dependency>)> {
        gcc::build(self, file)
    }

    pub fn new(bin: PathBuf, conf: &Config, is_c: bool) -> Result<Self> {
        try_new(bin, conf, is_c)
    }
}

impl Compiler for Gpp {
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

pub(super) fn try_new<C>(bin: PathBuf, conf: &Config, is_c: bool) -> Result<C>
where
    C: Compiler,
{
    let mut compile_args = vec![];
    let mut link_args = vec![];

    if is_c {
        link_args.push("-lstdc++".to_owned());
    }

    if !conf.optimization.in_range(0..=3) {
        return Err(Error::InvalidCompilerValue {
            option: "optimization".to_owned(),
            value: conf.optimization.to_string(),
        });
    }

    match conf.optimization {
        Optimization::None => compile_args.push("-O0".to_owned()),
        Optimization::All => compile_args.push("-O3".to_owned()),
        Optimization::Level(n) => compile_args.push(format!("-O{n}")),
    }

    if conf.asan {
        compile_args.push("-fsanitize=address".to_owned());
        link_args.push("-fsanitize=address".to_owned());
    }

    if conf.dbg_symbols {
        compile_args.push("-g".to_owned())
    }

    match &conf.cpp_std {
        Std::Number(n) => {
            if !conf.cpp_std.is_cpp_num() {
                return Err(Error::InvalidCompilerValue {
                    option: "cpp_std".to_owned(),
                    value: n.to_string(),
                });
            }
            compile_args.push(format!("-std=c++{n}"))
        }
        Std::Name(std) => compile_args.push(format!("-std={std}")),
    }

    compile_args.extend(conf.defines.iter().map(|(name, value)| {
        if let Some(value) = value {
            format!("-D{name}={value}")
        } else {
            format!("-D{name}")
        }
    }));

    compile_args.extend(conf.warn.iter().map(|w| format!("-W{w}")));
    compile_args.extend(conf.no_warn.iter().map(|w| format!("-Wno-{w}")));
    compile_args.extend(conf.args.iter().cloned());
    link_args.extend(conf.args.iter().cloned());

    C::try_new(bin, compile_args, link_args, conf)
}
