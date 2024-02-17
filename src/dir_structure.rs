use crate::{config::Config, err::Result};
use std::{borrow::Cow, fs::read_dir, path::PathBuf};

pub struct DirStructure {
    /// extensions of source files
    src_extensions: Vec<String>,
    /// all source files, each file coresponds to obj file
    src_files: Vec<PathBuf>,
    src_root: PathBuf,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl DirStructure {
    pub fn from_config(conf: &Config, release: bool) -> Self {
        if release {
            DirStructure::new(
                conf.release_build.compiler_conf.src_root.clone(),
            )
        } else {
            DirStructure::new(conf.debug_build.compiler_conf.src_root.clone())
        }
    }

    pub fn new(src_root: PathBuf) -> Self {
        Self {
            src_extensions: vec![
                "c".into(),
                "C".into(),
                "cc".into(),
                "cpp".into(),
                "CPP".into(),
                "c++".into(),
                "cp".into(),
                "cxx".into(),
            ],
            src_files: vec![],
            src_root,
        }
    }

    /// Finds all source files and generates corresponding files in
    /// [`Self::obj`]. Also sets [`Self::bin`].
    pub fn analyze(&mut self) -> Result<()> {
        self.src_files.clear();
        self.find_src_files()
    }

    /// gets the source files
    pub fn srcs(&self) -> &[PathBuf] {
        &self.src_files
    }
}

//===========================================================================//
//                                  Private                                  //
//===========================================================================//

impl DirStructure {
    /// finds all files in the directory [`Self::src`] with one of the
    /// extensions from [`Self::src_extensions`]
    fn find_src_files(&mut self) -> Result<()> {
        let mut dirs = vec![self.src_root.clone()];

        // Recursively search the directory for files with one of the
        // extensions. The recursion is achieved with the dirs stack.
        while let Some(dir) = dirs.pop() {
            for item in read_dir(dir)? {
                let item = item?;
                let typ = item.file_type()?;

                // recursively search in directories
                if typ.is_dir() {
                    dirs.push(item.path());
                    continue;
                }

                // add only files
                if !typ.is_file() {
                    continue;
                }

                // get the file path and extension
                let item = item.path();
                let ext = item
                    .extension()
                    .map_or_else(|| "".into(), |e| e.to_string_lossy());

                // check if the extension matches
                if !self
                    .src_extensions
                    .iter()
                    .any(|i| Cow::Borrowed(i.as_str()) == ext)
                {
                    continue;
                }

                self.src_files.push(item);
            }
        }

        Ok(())
    }
}
