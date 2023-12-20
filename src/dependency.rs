use std::{
    collections::{HashMap, HashSet},
    io,
    path::Path,
    rc::Rc,
};

use crate::{
    dir_structure::DirStructure,
    err::{Error, Result},
    include_deps::get_included_files,
};

#[derive(Debug, Clone)]
pub struct Dependency {
    /// File that has dependencies
    pub file: Rc<Path>,
    /// Direct dependencies to build [`Self::file`]
    pub direct: Vec<Rc<Path>>,
    /// Indirect dependencies of [`Self::file`]
    pub indirect: HashSet<Rc<Path>>,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl Dependency {
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
pub fn get_dependencies(
    dir: &DirStructure,
    dep_dep: &mut HashMap<Rc<Path>, Dependency>,
) -> Result<Vec<Dependency>> {
    let mut res = vec![];

    for (obj, src) in dir.objs().iter().zip(dir.srcs()) {
        res.push(Dependency::from_src(obj, src, dep_dep)?);
    }

    Ok(res)
}

//===========================================================================//
//                                  Private                                  //
//===========================================================================//

enum DepDirection {
    Same(Rc<Path>),
    LastDeeper(Rc<Path>),
}

/// Finds all dependencies of `file` from source file `src`
impl Dependency {
    fn _new(file: Rc<Path>) -> Self {
        Self {
            file,
            direct: vec![],
            indirect: HashSet::new(),
        }
    }

    fn from_src(
        file: &Path,
        src: &Path,
        dep_dep: &mut HashMap<Rc<Path>, Dependency>,
    ) -> Result<Self> {
        let direct = vec![src.into()];
        let mut indirect = HashSet::new();

        if let Some(parent) = src.parent() {
            indirect.extend(
                get_included_files(src)?
                    .into_iter()
                    .filter(|d| d.relative)
                    .map(|d| parent.join(d.path).into()),
            );
        }

        let mut to_exam: Vec<_> = indirect
            .iter()
            .map(|f: &Rc<Path>| DepDirection::Same(f.clone()))
            .collect();
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
                    top.indirect
                        .extend(dep.indirect.iter().map(|d| d.clone()));
                }
            } else {
                if let Some(parent) = file.parent() {
                    let indirect = get_included_files(&file)?
                        .into_iter()
                        .filter(|d| d.relative)
                        .map(|d| parent.join(d.path).into())
                        .filter(|d| {
                            *d != file
                                && !dep_stack.iter().any(|d2| d2.file == *d)
                        })
                        .collect();

                    //println!("{file:?}\n{dep_stack:?}\n{indirect:?}");

                    let dep = Self {
                        file,
                        direct: vec![],
                        indirect,
                    };

                    let mut indirect = dep.indirect.iter();

                    if let Some(d) = indirect.next() {
                        to_exam.push(DepDirection::LastDeeper(d.clone()));
                        to_exam.extend(
                            indirect.map(|d| DepDirection::Same(d.clone())),
                        );
                        dep_stack.push(dep);
                    } else {
                        dep_dep.insert(dep.file.clone(), dep);
                    }
                }
            }

            if pop {
                if let Some(dep) = dep_stack.pop() {
                    if let Some(top_dep) = dep_stack.last_mut() {
                        top_dep
                            .indirect
                            .extend(dep.indirect.iter().map(|d| d.clone()));
                    }
                    dep_dep.insert(dep.file.clone(), dep);
                }
            }
        }

        if dep_stack.len() > 1 {
            Err(Error::DoesNotHappen("Dependency stack has too many items."))
        } else if let Some(res) = dep_stack.into_iter().next() {
            Ok(Self {
                file: file.into(),
                direct: res.direct,
                indirect: res.indirect,
            })
        } else {
            Err(Error::DoesNotHappen("Dependency stack has no items"))
        }
    }
}
