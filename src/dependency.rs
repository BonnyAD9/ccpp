use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    io,
    ops::Deref,
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    err::{Error, Result},
    file_type::FileType,
    include_deps::get_included_files,
};

#[derive(Debug, Clone)]
pub struct Dependency {
    /// File that has dependencies
    pub file: DepFile,
    /// Direct dependencies to build [`Self::file`]
    pub direct: Vec<DepFile>,
    /// Indirect dependencies of [`Self::file`]
    pub indirect: HashSet<DepFile>,
}

#[derive(Clone, Eq, Debug)]
pub struct DepFile {
    pub path: Rc<Path>,
    pub typ: Option<FileType>,
}

impl PartialEq for DepFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Hash for DepFile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}

impl Deref for DepFile {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for DepFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl From<PathBuf> for DepFile {
    fn from(value: PathBuf) -> Self {
        let lang = value.extension().and_then(FileType::from_ext);
        Self {
            path: value.into(),
            typ: lang,
        }
    }
}

pub struct DepCache {
    cache: HashMap<DepFile, Dependency>,
}

enum DepDirection {
    Same(DepFile),
    LastDeeper(DepFile),
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl Dependency {
    pub fn new(
        file: DepFile,
        direct: Vec<DepFile>,
        indirect: HashSet<DepFile>,
    ) -> Self {
        Self {
            file,
            direct,
            indirect,
        }
    }

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

impl DepCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Finds the indirect dependencies for the given dependency file.
    pub fn fill_dependency(&mut self, dep: &mut Dependency) -> Result<()> {
        if self.cache.contains_key(&dep.file) {
            return Err(Error::DuplicateDependency);
        }

        for file in &dep.direct {
            let deps = self.get_dependencies(file.clone())?;
            dep.indirect.extend(deps.indirect.iter().cloned());
        }

        Ok(())
    }

    pub fn get_dependencies(&mut self, file: DepFile) -> Result<&Dependency> {
        let mut indirect: HashSet<DepFile> = HashSet::new();

        if let Some(parent) = file.parent() {
            indirect.extend(
                get_included_files(file.clone())?
                    .into_iter()
                    .filter(|d| d.relative)
                    .map(|d| parent.join(d.path).canonicalize())
                    .filter_map(|d| d.ok())
                    .map(|d| d.into()),
            );
        }

        let mut to_exam: Vec<_> = indirect
            .iter()
            .map(|f| DepDirection::Same(f.clone()))
            .collect();
        let mut dep_stack =
            vec![Dependency::new(file.clone(), vec![], indirect)];
        while let Some(file) = to_exam.pop() {
            let mut pop = false;
            let file = match file {
                DepDirection::Same(path) => path,
                DepDirection::LastDeeper(path) => {
                    pop = true;
                    path
                }
            };

            if let Some(dep) = self.cache.get(&file) {
                if let Some(top) = dep_stack.last_mut() {
                    top.indirect.extend(dep.indirect.iter().cloned());
                }
            } else if let Some(parent) = file.parent() {
                let indirect = get_included_files(file.clone())?
                    .into_iter()
                    .filter(|d| d.relative)
                    .map(|d| parent.join(d.path).canonicalize())
                    .filter(|d| d.is_ok())
                    .map(|d| d.unwrap().into())
                    .filter(|d| {
                        *d != file && !dep_stack.iter().any(|d2| d2.file == *d)
                    })
                    .collect();

                let dep = Dependency::new(file, vec![], indirect);

                let mut indirect = dep.indirect.iter();

                if let Some(d) = indirect.next() {
                    to_exam.push(DepDirection::LastDeeper(d.clone()));
                    to_exam.extend(
                        indirect.map(|d| DepDirection::Same(d.clone())),
                    );
                    dep_stack.push(dep);
                } else {
                    self.cache.insert(dep.file.clone(), dep);
                }
            }

            if pop {
                if let Some(dep) = dep_stack.pop() {
                    if let Some(top_dep) = dep_stack.last_mut() {
                        top_dep.indirect.extend(dep.indirect.iter().cloned());
                    }
                    self.cache.insert(dep.file.clone(), dep);
                }
            }
        }

        if dep_stack.len() > 1 {
            Err(Error::DoesNotHappen("Dependency stack has too many items."))
        } else if let Some(res) = dep_stack.into_iter().next() {
            self.cache.insert(file.clone(), res);
            self.cache.get(&file).ok_or(Error::DoesNotHappen(
                "Item just iserted into hashmap is not in the hashmap?",
            ))
        } else {
            Err(Error::DoesNotHappen("Dependency stack has no items"))
        }
    }
}
