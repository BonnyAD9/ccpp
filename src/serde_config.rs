use std::{
    fs::{self, read_to_string},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    compiler::config::{Optimization, Std},
    config::{Build, CompilerConfig, Config, Project},
    err::Result,
};

#[derive(Serialize, Deserialize, Default)]
pub struct SerdeConfig {
    pub project: SerdeProject,
    #[serde(default)]
    pub build: Option<SerdeBuild>,
    #[serde(default)]
    pub debug_build: Option<SerdeBuild>,
    #[serde(default)]
    pub release_build: Option<SerdeBuild>,
}

#[derive(Serialize, Deserialize)]
pub struct SerdeProject {
    pub name: String,
    pub src: Option<String>,
    pub bin: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SerdeBuild {
    #[serde(default)]
    pub cc: Option<String>,
    #[serde(default)]
    pub cpp: Option<String>,
    #[serde(default)]
    pub compiler_configuration: Option<SerdeCompilerConfig>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SerdeCompilerConfig {
    pub optimization: Option<Optimization>,
    pub asan: Option<bool>,
    pub dbg_symbols: Option<bool>,
    pub c_std: Option<Std>,
    pub cpp_std: Option<Std>,
    pub defines: Option<Vec<(String, Option<String>)>>,
    pub warn: Option<Vec<String>>,
    pub no_warn: Option<Vec<String>>,
    pub args: Option<Vec<String>>,
}

impl Config {
    pub fn from_toml_file<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(toml::from_str::<SerdeConfig>(&read_to_string(path)?)?.resolve())
    }
}

impl Default for SerdeProject {
    fn default() -> Self {
        Self {
            name: "main".into(),
            src: None,
            bin: None,
        }
    }
}

impl SerdeConfig {
    pub fn to_toml_file<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let value = toml::to_string_pretty(self)?;
        fs::write(path, value)?;
        Ok(())
    }

    fn resolve(self) -> Config {
        let bin =
            Path::new(self.project.bin.as_ref().map_or("bin", |s| s.as_str()));
        let src_root: PathBuf = self
            .project
            .src
            .as_ref()
            .map_or("src", |s| s.as_str())
            .into();
        let bin_debug_root = bin.join("debug");
        let bin_release_root = bin.join("release");

        #[allow(unused_mut)]
        let mut debug_target = bin_debug_root.join(&self.project.name);
        #[allow(unused_mut)]
        let mut release_target = bin_release_root.join(&self.project.name);

        #[cfg(target_os = "windows")]
        {
            debug_target.set_extension("exe");
            release_target.set_extension("exe");
        }

        let common = self.build.unwrap_or_default();
        let debug_build = self.debug_build.unwrap_or_default();
        let release_build = self.release_build.unwrap_or_default();

        Config {
            project: self.project.resolve(),
            debug_build: debug_build.resolve_debug(
                common.clone(),
                debug_target,
                src_root.clone(),
                bin_debug_root,
            ),
            release_build: release_build.resolve_release(
                common,
                release_target,
                src_root,
                bin_release_root,
            ),
        }
    }
}

impl SerdeProject {
    fn resolve(self) -> Project {
        Project { name: self.name }
    }
}

impl SerdeBuild {
    fn resolve_debug(
        self,
        common: SerdeBuild,
        target: PathBuf,
        src_root: PathBuf,
        bin_root: PathBuf,
    ) -> Build {
        let compiler_configuration =
            match (self.compiler_configuration, common.compiler_configuration)
            {
                (Some(s), Some(c)) => s.resolve_debug(c, src_root, bin_root),
                (None, Some(s)) | (Some(s), None) => {
                    s.resolve_debug(Default::default(), src_root, bin_root)
                }
                (None, None) => SerdeCompilerConfig::default().resolve_debug(
                    Default::default(),
                    src_root,
                    bin_root,
                ),
            };

        Build {
            target,
            cc: self.cc.or(common.cc).map(Into::into),
            cpp: self.cpp.or(common.cpp).map(Into::into),
            compiler_conf: compiler_configuration,
        }
    }

    fn resolve_release(
        self,
        common: SerdeBuild,
        target: PathBuf,
        src_root: PathBuf,
        bin_root: PathBuf,
    ) -> Build {
        let compiler_conf =
            match (self.compiler_configuration, common.compiler_configuration)
            {
                (Some(s), Some(c)) => s.resolve_release(c, src_root, bin_root),
                (None, Some(s)) | (Some(s), None) => {
                    s.resolve_release(Default::default(), src_root, bin_root)
                }
                (None, None) => SerdeCompilerConfig::default()
                    .resolve_release(Default::default(), src_root, bin_root),
            };

        Build {
            target,
            cc: self.cc.or(common.cc).map(Into::into),
            cpp: self.cpp.or(common.cpp).map(Into::into),
            compiler_conf,
        }
    }
}

macro_rules! vec_join_or {
    ($default:expr, $a:expr, $b:expr) => {
        match ($a, $b) {
            (Some(mut a), Some(mut b)) => {
                a.append(&mut b);
                a
            }
            (Some(s), None) | (None, Some(s)) => s,
            (None, None) => $default,
        }
    };
}

impl SerdeCompilerConfig {
    fn resolve_debug(
        self,
        common: SerdeCompilerConfig,
        src_root: PathBuf,
        bin_root: PathBuf,
    ) -> CompilerConfig {
        CompilerConfig {
            bin_root,
            src_root,
            optimization: self
                .optimization
                .or(common.optimization)
                .unwrap_or(Optimization::None),
            asan: self.asan.or(common.asan).unwrap_or(true),
            dbg_symbols: self
                .dbg_symbols
                .or(common.dbg_symbols)
                .unwrap_or(true),
            c_std: self.c_std.or(common.c_std).unwrap_or(17.into()),
            cpp_std: self.cpp_std.or(common.cpp_std).unwrap_or(20.into()),
            defines: vec_join_or!(vec![], common.defines, self.defines),
            warn: vec_join_or!(vec!["all".into()], common.warn, self.warn),
            no_warn: vec_join_or!(vec![], common.no_warn, self.no_warn),
            args: vec_join_or!(vec![], common.args, self.args),
        }
    }

    fn resolve_release(
        self,
        common: SerdeCompilerConfig,
        src_root: PathBuf,
        bin_root: PathBuf,
    ) -> CompilerConfig {
        CompilerConfig {
            bin_root,
            src_root,
            optimization: self
                .optimization
                .or(common.optimization)
                .unwrap_or(Optimization::All),
            asan: self.asan.or(common.asan).unwrap_or_default(),
            dbg_symbols: self
                .dbg_symbols
                .or(common.dbg_symbols)
                .unwrap_or_default(),
            c_std: self.c_std.or(common.c_std).unwrap_or(17.into()),
            cpp_std: self.cpp_std.or(common.cpp_std).unwrap_or(20.into()),
            defines: vec_join_or!(
                vec![("NDEBUG".into(), None)],
                common.defines,
                self.defines
            ),
            warn: vec_join_or!(vec!["all".to_owned()], common.warn, self.warn),
            no_warn: vec_join_or!(vec![], common.no_warn, self.no_warn),
            args: vec_join_or!(vec![], common.args, self.args),
        }
    }
}
