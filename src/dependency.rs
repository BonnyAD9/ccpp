use std::{
    borrow::Cow,
    collections::HashMap,
    io,
    path::Path,
};

use crate::{
    dir_structure::DirStructure, err::Result, include_deps::get_included_files,
};

#[derive(Debug, Clone)]
pub struct Dependency<'a> {
    /// File that has dependencies
    pub file: Cow<'a, Path>,
    /// Direct dependencies to build [`Self::file`]
    pub direct: Vec<Cow<'a, Path>>,
    /// Indirect dependencies of [`Self::file`]
    pub indirect: Vec<Cow<'a, Path>>,
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
            Err(e) if e.kind() == io::ErrorKind::Unsupported => {
                return Ok(false);
            }
            e => e?,
        };

        // need to update if dependency is newer than file
        for dep in self.direct.iter().chain(self.indirect.iter()) {
            let dep_mod = dep.metadata()?.modified()?;
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
    dep_dep: &'a mut HashMap<Cow<'a, Path>, Dependency<'a>>,
) -> Result<Vec<Dependency<'a>>> {
    let mut res = vec![];

    for (obj, src) in dir.objs().iter().zip(dir.srcs()) {
        res.push(Dependency::from_src(obj, src, dep_dep)?);
    }

    Ok(res)
}

//===========================================================================//
//                                  Private                                  //
//===========================================================================//

enum DepDirection<'a> {
    Same(Cow<'a, Path>),
    LastDeeper(Cow<'a, Path>),
}

/// Finds all dependencies of `file` from source file `src`
impl<'a> Dependency<'a> {
    fn new(file: Cow<'a, Path>) -> Self {
        Self {
            file,
            direct: vec![],
            indirect: vec![],
        }
    }

    fn from_src<'b>(
        file: &'a Path,
        src: &'a Path,
        dep_dep: &'b mut HashMap<Cow<'b, Path>, Dependency<'b>>,
    ) -> Result<Self> {
        let direct = vec![src.into()];
        let mut indirect = vec![];

        if let Some(parent) = src.parent() {
            indirect.extend(
                get_included_files(src)?
                    .into_iter()
                    .filter(|d| d.relative)
                    .map(|d| parent.join(d.path).into()),
            );
        }

        let mut to_exam: Vec<_> =
            indirect.iter().map(|f| DepDirection::Same(*f)).collect();
        let mut dep_stack = vec![Self {
            file: src.into(),
            direct,
            indirect,
        }];
        while let Some(file) = to_exam.pop() {
            let mut pop = false;
            let file = match file {
                DepDirection::Same(path) => path,
                DepDirection::LastDeeper(path) => {
                    pop = true;
                    path
                }
            };

            if let Some(dep) = dep_dep.get(&file) {
                if let Some(top) = dep_stack.last_mut() {
                    top.indirect.extend(dep.indirect.iter().map(|d| *d));
                }
            } else if !dep_stack.iter().any(|d| d.file == file) {
                if let Some(parent) = file.parent() {
                    let indirect = get_included_files(&file)?
                        .into_iter()
                        .filter(|d| d.relative)
                        .map(|d| parent.join(d.path).into())
                        .collect();

                    let dep = Self {
                        file,
                        direct: vec![],
                        indirect,
                    };

                    if dep.indirect.is_empty() {
                        dep_dep.insert(dep.file, dep);
                    } else {
                        to_exam.push(DepDirection::LastDeeper(dep.indirect[0]));
                        to_exam.extend(dep.indirect.iter().skip(1).map(|d| DepDirection::Same(*d)));
                        dep_stack.push(dep);
                    }
                }
            }

            if pop {
                if let Some(dep) = dep_stack.pop() {
                    if let Some(top_dep) = dep_stack.last_mut() {
                        top_dep.indirect.extend(dep.indirect.iter().map(|d| *d));
                    }
                    dep_dep.insert(dep.file, dep);
                }
            }
        }

        if dep_stack.len() != 1 {
            eprintln!("Error that should never happen");
        }

        Ok(dep_stack[0])
    }
}
