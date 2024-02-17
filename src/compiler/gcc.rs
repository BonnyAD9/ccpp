use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    dependency::{DepFile, Dependency},
    err::{Error, Result},
    file_type::{FileState, FileType},
};

use super::{
    common::Compiler,
    config::{Config, Optimization, Std},
};

pub struct Gcc {
    bin: PathBuf,
    src_root: PathBuf,
    bin_root: PathBuf,
    compile_args: Vec<String>,
    link_args: Vec<String>,
}

impl Gcc {
    pub fn build(
        &self,
        file: Dependency,
    ) -> Result<(Command, Vec<Dependency>)> {
        build(self, file)
    }

    pub fn new(bin: PathBuf, conf: &Config) -> Result<Self> {
        try_new(bin, conf)
    }
}

impl Compiler for Gcc {
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

// the implementation of the compilation is implemented on the common compiler
// trait so that other compilers may reuse the code

pub(super) fn try_new<C>(bin: PathBuf, conf: &Config) -> Result<C>
where
    C: Compiler,
{
    let mut compile_args = vec![];
    let mut link_args = vec![];

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

    match &conf.c_std {
        Std::Number(n) => {
            if !conf.c_std.is_c_num() {
                return Err(Error::InvalidCompilerValue {
                    option: "c_std".to_owned(),
                    value: n.to_string(),
                });
            }
            compile_args.push(format!("-std=c{n}"))
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

pub(super) fn build<C>(
    cc: &C,
    file: Dependency,
) -> Result<(Command, Vec<Dependency>)>
where
    C: Compiler,
{
    let typ = if let Some(typ) = file.file.typ {
        typ
    } else {
        return Err(Error::InvalidFileType(file.file));
    };

    match typ.state {
        FileState::Object => build_object(cc, file),
        FileState::Executable => build_executable(cc, file),
        _ => Err(Error::InvalidFileType(file.file)),
    }
}

pub(super) fn build_object<C>(
    cc: &C,
    file: Dependency,
) -> Result<(Command, Vec<Dependency>)>
where
    C: Compiler,
{
    if file.direct.is_empty() {
        return Err(Error::NothingToBuild(file.file.path.to_path_buf()));
    }

    let mut cmd = Command::new(cc.bin());
    cmd.args(["-c", "-o"]).arg(file.file.path.as_ref());

    for file in file.direct {
        if !matches!(
            file.typ,
            Some(FileType {
                state: FileState::Source,
                ..
            })
        ) {
            return Err(Error::InvalidFileType(file));
        }
        cmd.arg(file.path.as_ref());
    }

    cmd.args(cc.compile_args());

    Ok((cmd, vec![]))
}

pub(super) fn build_executable<C>(
    cc: &C,
    file: Dependency,
) -> Result<(Command, Vec<Dependency>)>
where
    C: Compiler,
{
    if file.direct.is_empty() {
        return Err(Error::NothingToBuild(file.file.path.to_path_buf()));
    }

    let mut cmd = Command::new(cc.bin());
    cmd.arg("-o").arg(file.file.as_ref());

    let mut deps = vec![];

    for file in file.direct {
        let typ = if let Some(typ) = file.typ {
            typ
        } else {
            return Err(Error::InvalidFileType(file));
        };

        match typ.state {
            FileState::Object => _ = cmd.arg(file.as_ref()),
            FileState::Source => {
                let dep = obj_source_dep(cc, file)?;
                cmd.arg(dep.file.as_ref());
                deps.push(dep);
            }
            _ => return Err(Error::InvalidFileType(file)),
        }
    }

    cmd.args(cc.link_args());

    Ok((cmd, deps))
}

pub(super) fn obj_source_dep<C>(cc: &C, file: DepFile) -> Result<Dependency>
where
    C: Compiler,
{
    let mut res = cc.bin_root().join("project");
    res.push(file.strip_prefix(cc.src_root())?);
    res.as_mut_os_string().push(".o");

    let res = DepFile {
        path: res.into(),
        typ: file.typ.map(|t| FileType {
            state: FileState::Object,
            ..t
        }),
    };
    let direct = vec![file];

    Ok(Dependency::new(res, direct, Default::default()))
}
