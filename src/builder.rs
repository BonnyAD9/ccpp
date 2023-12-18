use std::{
    ffi::OsStr,
    fs::create_dir_all,
    path::Path,
    process::{Child, Command},
};

use crate::{
    dependency::{get_dependencies, Dependency},
    dir_structure::DirStructure,
    err::{Error, Result},
};

pub struct Builder {
    /// C compiler
    pub cc: String,
    /// Linker
    pub ld: String,
    /// Aditional flags for compiler
    pub cflags: Vec<String>,
    /// Aditional flags for linker
    pub ldflags: Vec<String>,
    pub print_command: bool,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl Builder {
    pub fn build(&self, dir: &DirStructure) -> Result<()> {
        let deps = get_dependencies(dir)?;

        for file in &deps {
            if file.is_up_to_date()? {
                continue;
            }

            self.sync_build_file(
                file.direct.iter().map(|i| i.as_ref()),
                file.file,
            )?;
        }

        let bin_dep = Dependency {
            file: dir.binary(),
            direct: dir.objs().iter().map(|o| o.into()).collect(),
            indirect: vec![],
        };

        if !bin_dep.is_up_to_date()? {
            self.sync_link_file(dir.objs().iter(), dir.binary())?;
        }

        Ok(())
    }
}

//===========================================================================//
//                                  Private                                  //
//===========================================================================//

impl Builder {
    fn start_build_file<'a, S, I>(
        &self,
        input: I,
        output: &Path,
    ) -> Result<Child>
    where
        S: AsRef<OsStr>,
        I: Iterator<Item = S>,
    {
        if let Some(d) = output.parent() {
            create_dir_all(d)?;
        }

        let mut cmd = Command::new(&self.cc);
        cmd.args(self.cflags.iter())
            .arg("-o")
            .arg(output)
            .arg("-c")
            .args(input);

        if self.print_command {
            println!("{cmd:?}");
        }
        Ok(cmd.spawn()?)
    }

    fn sync_build_file<'a, S, I>(&self, input: I, output: &Path) -> Result<()>
    where
        S: AsRef<OsStr>,
        I: Iterator<Item = S>,
    {
        let res = self.start_build_file(input, output)?.wait()?;
        if !res.success() {
            Err(Error::ProcessFailed(res.code()))
        } else {
            Ok(())
        }
    }

    fn start_link_file<'a, S, I>(
        &self,
        input: I,
        output: &Path,
    ) -> Result<Child>
    where
        S: AsRef<OsStr>,
        I: Iterator<Item = S>,
    {
        if let Some(d) = output.parent() {
            create_dir_all(d)?;
        }

        let mut cmd = Command::new(&self.ld);
        cmd.args(self.ldflags.iter())
            .arg("-o")
            .arg(output)
            .args(input);

        if self.print_command {
            println!("{cmd:?}");
        }

        Ok(cmd.spawn()?)
    }

    fn sync_link_file<'a, S, I>(&self, input: I, output: &Path) -> Result<()>
    where
        S: AsRef<OsStr>,
        I: Iterator<Item = S>,
    {
        let res = self.start_link_file(input, output)?.wait()?;
        if !res.success() {
            Err(Error::ProcessFailed(res.code()))
        } else {
            Ok(())
        }
    }
}
