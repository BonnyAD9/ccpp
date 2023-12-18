use std::{borrow::Cow, path::Path, io};

use crate::{dir_structure::DirStructure, err::{Result, Error}};

pub struct Dependency<'a> {
    /// File that has dependencies
    pub file: &'a Path,
    /// Dependencies of [`Self::file`]
    pub deps: Vec<Cow<'a, Path>>,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl<'a> Dependency<'a> {
    pub fn is_up_to_date(&self) -> Result<bool> {
        if !self.file.exists() {
            return Ok(false);
        }

        // get the last modified date, this may not be supported, in that case
        // always return false
        let last_mod = match self.file.metadata()?.modified() {
            Ok(dt) => dt,
            Err(e) => {
                return if e.kind() == io::ErrorKind::Unsupported {
                    Ok(false)
                } else {
                    Err(Error::Io(e))
                };
            }
        };

        // need to update if dependency is newer than file
        for dep in &self.deps {
            let dep_mod = self.file.metadata()?.modified()?;
            if dep_mod > last_mod {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

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