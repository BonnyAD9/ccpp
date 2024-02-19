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
    file_type::{FileState, FileType, Language},
    include_deps::{get_included_files, IncFile},
};

#[derive(Debug, Clone)]
pub struct Dependency {
    /// File that has dependencies
    pub file: DepFile,
    /// Direct dependencies to build [`Self::file`]
    pub direct: Vec<DepFile>,
    /// Indirect dependencies of [`Self::file`]
    pub transitive: HashSet<DepFile>,
    pub non_transitive: HashSet<DepFile>,
    pub modules: Modules,
}

#[derive(Debug, Clone, Default)]
pub struct Modules {
    provides: Option<DepModule>,
    imports: Vec<DepModule>,
    exports: Vec<DepModule>,
    user: Vec<DepFile>,
    system: Vec<DepFile>,
    user_exports: Vec<DepFile>,
    system_exports: Vec<DepFile>,
}

#[derive(Clone, Eq, Debug)]
pub struct DepFile {
    pub path: Rc<Path>,
    pub typ: Option<FileType>,
}

pub type DepModule = Rc<str>;

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

impl DepFile {
    pub fn header(path: PathBuf) -> Self {
        let mut res: Self = path.into();
        res.typ = if let Some(typ) = res.typ {
            Some(FileType { lang: typ.lang, state: FileState::Header })
        } else {
            Some(FileType { lang: Language::Cpp, state: FileState::Header })
        };
        res
    }
}

pub struct DepCache {
    file_cache: HashMap<DepFile, Dependency>,
    module_map: HashMap<DepModule, DepFile>,
    module_cache: HashMap<DepFile, Dependency>,
}

struct DepInfo {
    file: DepFile,
    pop: bool,
    transitive: bool,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl Dependency {
    pub fn new(
        file: DepFile,
        direct: Vec<DepFile>,
        transitive: HashSet<DepFile>,
    ) -> Self {
        Self {
            file,
            direct,
            transitive,
            non_transitive: HashSet::new(),
            modules: Modules::default(),
        }
    }

    pub fn empty(file: DepFile) -> Self {
        Self {
            file,
            direct: vec![],
            transitive: HashSet::new(),
            non_transitive: HashSet::new(),
            modules: Modules::default(),
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
        for dep in self.direct.iter().chain(self.transitive.iter()) {
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
            file_cache: HashMap::new(),
            module_map: HashMap::new(),
            module_cache: HashMap::new(),
        }
    }

    /// Finds the indirect dependencies for the given dependency file.
    pub fn fill_dependency(&mut self, dep: &mut Dependency) -> Result<()> {
        if self.file_cache.contains_key(&dep.file) {
            return Err(Error::DuplicateDependency);
        }

        for file in &dep.direct {
            if matches!(file.typ, Some(FileType { state: FileState::Header | FileState::Source | FileState::SourceModule, .. })) {
                let deps = self.get_dependencies(file.clone())?;
                dep.transitive.extend(deps.transitive.iter().cloned());
            }
        }

        self.resolve_modules();

        Ok(())
    }

    pub fn get_dependencies(&mut self, file: DepFile) -> Result<&Dependency> {
        self.resolve_dependency(file.clone())?;
        self.resolve_modules();
        Ok(self.file_cache.get(&file).unwrap())
    }

    fn resolve_dependency(&mut self, mut file: DepFile) -> Result<&Dependency> {
        // This cannot be converted to if let, because borrow checker is not
        // yet smart enough
        if self.file_cache.contains_key(&file) {
            return Ok(self.file_cache.get(&file).unwrap());
        }

        let mut dep = Dependency::empty(file.clone());

        parse_dependencies(&mut dep)?;
        if dep.modules.provides.is_some() {
            file.typ = Some(FileType {
                lang: Language::Cpp,
                state: FileState::SourceModule
            });
            dep.file.typ = file.typ;
        }

        let mut to_exam: Vec<_> = dep.transitive
            .iter()
            .map(|f| DepInfo {
                file: f.clone(),
                pop: false,
                transitive: true,
            })
            .chain(dep.non_transitive.iter().map(|f| DepInfo {
                file: f.clone(),
                pop: false,
                transitive: false,
            }))
            .collect();
        let mut dep_stack = vec![dep];

        while let Some(DepInfo {file, pop, transitive}) = to_exam.pop() {
            if let Some(dep) = self.file_cache.get(&file) {
                if let Some(top) = dep_stack.last_mut() {
                    top.transitive.extend(dep.transitive.iter().cloned());
                }
            }

            let mut dep = Dependency::empty(file);

            parse_dependencies(&mut dep)?;

            let mut transitive_i = dep.transitive.iter();
            let mut non_transitive_i = dep.non_transitive.iter();

            if let Some(d) = transitive_i.next() {
                to_exam.push(DepInfo { file: d.clone(), pop: true, transitive: true });
                to_exam.extend(transitive_i.map(|d| DepInfo {
                    file: d.clone(),
                    pop: false,
                    transitive: true,
                }));
                to_exam.extend(non_transitive_i.map(|d| DepInfo {
                    file: d.clone(),
                    pop: false,
                    transitive: false,
                }));
                dep_stack.push(dep);
            } else if let Some(d) = non_transitive_i.next() {
                to_exam.push(DepInfo { file: d.clone(), pop: true, transitive: false });
                to_exam.extend(non_transitive_i.map(|d| DepInfo {
                    file: d.clone(),
                    pop: false,
                    transitive: false,
                }));
                dep_stack.push(dep);
            } else {
                self.file_cache.insert(dep.file.clone(), dep);
            }

            if pop {
                if let Some(dep) = dep_stack.pop() {
                    if let Some(top_dep) = dep_stack.last_mut() {
                        if transitive {
                            top_dep.transitive.extend(dep.transitive.iter().cloned());
                        } else {
                            top_dep.non_transitive.extend(dep.transitive.iter().cloned());
                        }
                    }
                    if let Some(m) = &dep.modules.provides {
                        self.module_map.insert(m.clone(), dep.file.clone());
                    }
                    self.file_cache.insert(dep.file.clone(), dep);
                }
            }
        }

        if dep_stack.len() > 1 {
            Err(Error::DoesNotHappen("Dependency stack has too many items."))
        } else if let Some(res) = dep_stack.into_iter().next() {
            if let Some(m) = &res.modules.provides {
                self.module_map.insert(m.clone(), res.file.clone());
            }
            self.file_cache.insert(file.clone(), res);
            self.file_cache.get(&file).ok_or(Error::DoesNotHappen(
                "Item just iserted into hashmap is not in the hashmap?",
            ))
        } else {
            Err(Error::DoesNotHappen("Dependency stack has no items"))
        }
    }

    fn resolve_modules(&mut self) {
        // TODO
    }
}

fn parse_dependencies(dep: &mut Dependency) -> Result<()> {
    if let Some(parent) = dep.file.parent() {
        for inc in get_included_files(&dep.file)? {
            match inc {
                IncFile::User(m) => _ = dep.transitive.insert(DepFile::header(parent.join(m))),
                IncFile::System(_) => {},
                IncFile::ExpModule(m) => dep.modules.provides = Some(m.into()),
                IncFile::ImpModule(mut m) => {
                    if m.starts_with(':') {
                        if let Some(base) = &dep.modules.provides {
                            m.insert_str(0, base);
                        }
                    }
                    dep.modules.imports.push(m.into());
                },
                IncFile::ExpImpModule(mut m) => {
                    if m.starts_with(':') {
                        if let Some(base) = &dep.modules.provides {
                            m.insert_str(0, base);
                        }
                    }
                    dep.modules.exports.push(m.into())
                },
                IncFile::UserModule(m) => {
                    let f = DepFile::header(parent.join(m));
                    dep.modules.user.push(f.clone());
                    dep.non_transitive.insert(f);
                },
                IncFile::SystemModule(m) => dep.modules.system.push(DepFile::header(m)),
                IncFile::ExpUserModule(m) => {
                    let f = DepFile::header(parent.join(m));
                    dep.modules.user_exports.push(f.clone());
                    dep.transitive.insert(f);
                },
                IncFile::ExpSystemModule(m) => dep.modules.system_exports.push(DepFile::header(m)),
            }
        }
    }

    Ok(())
}
