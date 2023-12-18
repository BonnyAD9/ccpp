use std::{borrow::Cow, path::Path};

use crate::{dir_structure::DirStructure, err::Result};

pub struct Dependency<'a> {
    /// File that has dependencies
    pub file: &'a Path,
    /// Dependencies of [`Self::file`]
    pub deps: Vec<Cow<'a, Path>>,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

/// Finds all dependencies for the project in the directory structure
pub fn get_dependencies<'a>(
    dir: &'a DirStructure,
) -> Result<Vec<Dependency<'a>>> {
    dir.objs()
        .iter()
        .zip(dir.srcs())
        .map(|(obj, src)| Dependency::from_src(obj, src))
        .collect()
}

//===========================================================================//
//                                  Private                                  //
//===========================================================================//

/// Finds all dependencies of `file` from source file `src`
impl<'a> Dependency<'a> {
    fn from_src(file: &'a Path, src: &'a Path) -> Result<Self> {
        let deps = vec![src.into()];

        // TODO: find included files

        Ok(Self { file, deps })
    }
}
