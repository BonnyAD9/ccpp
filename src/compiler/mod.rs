use std::{
    borrow::Cow,
    env,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    dependency::Dependency,
    err::{Error, Result},
    file_type::Language,
};

use self::{
    clang::Clang, clangpp::Clangpp, config::Config, gcc::Gcc, gpp::Gpp,
};

mod clang;
mod clangpp;
mod common;
pub mod config;
mod gcc;
mod gpp;

macro_rules! operate {
    ($typ:ident, $compiler:expr, $name:ident, $op:expr) => {
        match $compiler {
            $typ::Gcc($name) => $op,
            $typ::Clang($name) => $op,
        }
    };
}

macro_rules! c_op {
    ($compiler:expr, $name:ident, $op:expr) => {
        operate!(CCompiler, $compiler, $name, $op)
    };
}

macro_rules! cpp_op {
    ($compiler:expr, $name:ident, $op:expr) => {
        operate!(CppCompiler, $compiler, $name, $op)
    };
}

const MAX_SCORE: i32 = 3;

enum CCompiler {
    Gcc(Gcc),
    Clang(Clang),
}

impl CCompiler {
    pub fn new(path: Option<PathBuf>, conf: &Config) -> Result<Self> {
        let (path, typ) = find_compiler(path, Language::C);
        match typ {
            CompilerType::Gcc | CompilerType::Gpp | CompilerType::Other => {
                Ok(Self::Gcc(Gcc::new(path, conf)?))
            }
            CompilerType::Clang | CompilerType::Clangpp => {
                Ok(Self::Clang(Clang::new(path, conf)?))
            }
        }
    }
}

enum CppCompiler {
    Gcc(Gpp),
    Clang(Clangpp),
}

impl CppCompiler {
    pub fn new(path: Option<PathBuf>, conf: &Config) -> Result<Self> {
        let (path, typ) = find_compiler(path, Language::Cpp);
        match typ {
            CompilerType::Gcc | CompilerType::Other => {
                Ok(Self::Gcc(Gpp::new(path, conf, true)?))
            }
            CompilerType::Gpp => Ok(Self::Gcc(Gpp::new(path, conf, false)?)),
            CompilerType::Clang => {
                Ok(Self::Clang(Clangpp::new(path, conf, true)?))
            }
            CompilerType::Clangpp => {
                Ok(Self::Clang(Clangpp::new(path, conf, false)?))
            }
        }
    }
}

#[derive(Copy, Clone)]
enum CompilerType {
    Gcc,
    Gpp,
    Clang,
    Clangpp,
    Other,
}

pub struct Compiler {
    c: CCompiler,
    cpp: CppCompiler,
}

impl Compiler {
    pub fn new(
        c: Option<PathBuf>,
        cpp: Option<PathBuf>,
        conf: &Config,
    ) -> Result<Self> {
        Ok(Self {
            c: CCompiler::new(c, conf)?,
            cpp: CppCompiler::new(cpp, conf)?,
        })
    }

    pub fn build(
        &self,
        file: Dependency,
    ) -> Result<(Command, Vec<Dependency>)> {
        if let Some(typ) = file.file.typ {
            match typ.lang {
                Language::C => c_op!(&self.c, cc, cc.build(file)),
                Language::Cpp => cpp_op!(&self.cpp, cpp, cpp.build(file)),
            }
        } else {
            Err(Error::InvalidFileType(file.file))
        }
    }
}

fn find_compiler(
    path: Option<PathBuf>,
    lng: Language,
) -> (PathBuf, CompilerType) {
    let (mut path, mut typ, mut score) = if let Some(p) = path {
        if let Some(c) = test_compiler(&p) {
            return (p, c);
        } else {
            (Cow::Owned(p), CompilerType::Other, 0)
        }
    } else {
        (Path::new("gcc").into(), CompilerType::Gcc, -2)
    };

    let str2path = |s| Cow::Borrowed(Path::new(s));
    let string2path = |s| Cow::Owned(PathBuf::from(s));

    let c = env::var("CC")
        .into_iter()
        .map(string2path)
        .chain(["cc", "gcc", "clang"].into_iter().map(str2path));
    let cpp = env::var("CXX")
        .into_iter()
        .map(string2path)
        .chain(["c++", "g++", "clang++"].into_iter().map(str2path));
    let mix = ["cl"].into_iter().map(str2path);

    let comps = match lng {
        Language::C => c.chain(mix).chain(cpp),
        Language::Cpp => cpp.chain(mix).chain(c),
    };

    for c in comps {
        let t = test_compiler(&c);
        let s = score_compiler(t, lng);
        if s > score {
            path = c;
            typ = t.unwrap_or(CompilerType::Other);
            score = s;
            if s == MAX_SCORE {
                return (path.into_owned(), typ);
            }
        }
    }

    (path.into_owned(), typ)
}

fn score_compiler(comp: Option<CompilerType>, lng: Language) -> i32 {
    let comp = if let Some(c) = comp {
        c
    } else {
        return -1;
    };

    match comp {
        CompilerType::Other => 1,
        CompilerType::Clang | CompilerType::Gcc => {
            if lng == Language::C {
                MAX_SCORE
            } else {
                2
            }
        }
        CompilerType::Clangpp | CompilerType::Gpp => {
            if lng == Language::Cpp {
                MAX_SCORE
            } else {
                2
            }
        }
    }
}

fn test_compiler(path: &Path) -> Option<CompilerType> {
    let out = Command::new(path).arg("--version").output().ok()?;
    if !out.status.success() {
        return Some(CompilerType::Other);
    }

    const SPACE: u8 = ' ' as u8;
    let name = out
        .stdout
        .iter()
        .position(|c| *c == SPACE)
        .map_or(out.stdout.as_slice(), |c| &out.stdout[..c]);

    let name = String::from_utf8_lossy(name);
    match name.as_ref() {
        "gcc" => Some(CompilerType::Gcc),
        "g++" => Some(CompilerType::Gpp),
        "clang" => {
            let path = path.to_string_lossy();
            if path.ends_with("++") || path.ends_with("pp") {
                Some(CompilerType::Clangpp)
            } else {
                Some(CompilerType::Clang)
            }
        }
        _ => Some(CompilerType::Other),
    }
}
