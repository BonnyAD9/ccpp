use crate::err::Result;
use std::{
    borrow::Cow,
    fs::read_dir,
    path::{Path, PathBuf},
};

pub struct DirStructure {
    /// extensions of source files
    src_extensions: Vec<String>,
    /// path to the source directory
    src: PathBuf,
    /// path to the release binary
    rel_bin: PathBuf,
    /// path to the debug binary
    deb_bin: PathBuf,
    /// path to the release objects folder
    rel_obj: PathBuf,
    /// path to the debug objects folder
    deb_obj: PathBuf,
    /// path to object files, each obj file coresponds to src file at the same
    /// position
    obj: Vec<PathBuf>,
    /// path to the binary file
    bin: PathBuf,
    /// all source files, each file coresponds to obj file
    src_files: Vec<PathBuf>,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl DirStructure {
    pub fn new<S>(proj_name: S) -> Self
    where
        S: AsRef<str>,
    {
        #[allow(unused_mut)]
        let mut rel_bin = Path::new("bin/release/").join(proj_name.as_ref());
        #[allow(unused_mut)]
        let mut deb_bin = Path::new("bin/debug/").join(proj_name.as_ref());

        #[cfg(target_os = "windows")]
        {
            rel_bin.set_extension("exe");
            deb_bin.set_extension("exe");
        }

        Self {
            src_extensions: vec!["c".to_owned()],
            src: "src/".into(),
            rel_bin,
            deb_bin,
            rel_obj: "bin/release/obj/".into(),
            deb_obj: "bin/debug/obj/".into(),
            obj: vec![],
            bin: PathBuf::new(),
            src_files: vec![],
        }
    }

    /// Finds all source files and generates corresponding files in
    /// [`Self::obj`]. Also sets [`Self::bin`].
    pub fn analyze(&mut self, release: bool) -> Result<()> {
        self.src_files.clear();
        self.find_src_files()?;

        self.bin = if release {
            self.rel_bin.clone()
        } else {
            self.deb_bin.clone()
        };

        self.obj.clear();
        self.gen_objs(release)
    }

    /// gets the source files
    pub fn srcs(&self) -> &[PathBuf] {
        &self.src_files
    }

    /// gets the object files
    pub fn objs(&self) -> &[PathBuf] {
        &self.obj
    }

    /// gets path to the binary file
    pub fn binary(&self) -> &Path {
        &self.bin
    }

    /// gets the release binary file
    pub fn rel_bin(&self) -> &Path {
        &self.rel_bin
    }

    /// gets the debug binary file
    pub fn deb_bin(&self) -> &Path {
        &self.deb_bin
    }

    /// gets the release objects folder
    pub fn rel_obj(&self) -> &Path {
        &self.rel_obj
    }

    /// gets the debug objects folder
    pub fn deb_obj(&self) -> &Path {
        &self.deb_obj
    }
}

//===========================================================================//
//                                  Private                                  //
//===========================================================================//

impl DirStructure {
    /// finds all files in the directory [`Self::src`] with one of the
    /// extensions from [`Self::src_extensions`]
    fn find_src_files(&mut self) -> Result<()> {
        let mut dirs = vec![self.src.clone()];

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

    /// Generates obj files from src files with debug/release path prefix
    fn gen_objs(&mut self, release: bool) -> Result<()> {
        const EXT: &str = "o";

        let prefix = if release {
            &self.rel_obj
        } else {
            &self.deb_obj
        };

        for s in &self.src_files {
            let mut d = prefix.join(s.strip_prefix(&self.src)?);
            d.set_extension(EXT);
            self.obj.push(d);
        }

        Ok(())
    }
}
