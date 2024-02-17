use std::path::{Path, PathBuf};

use crate::err::Result;

use super::config::Config;

pub(super) trait Compiler {
    fn bin(&self) -> &Path;

    fn src_root(&self) -> &Path;

    fn bin_root(&self) -> &Path;

    fn compile_args(&self) -> &Vec<String>;

    fn link_args(&self) -> &Vec<String>;

    fn try_new(
        bin: PathBuf,
        compile_args: Vec<String>,
        link_args: Vec<String>,
        conf: &Config,
    ) -> Result<Self>
    where
        Self: Sized;
}
