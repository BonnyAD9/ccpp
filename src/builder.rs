use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs::create_dir_all,
    path::Path,
    process::{Child, Command},
};

use crate::{
    config::Config,
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
    pub fn from_config(conf: &Config, release: bool) -> Self {
        let mut cc = conf.build.cc.as_ref();
        let mut ld = conf.build.ld.as_ref();
        let mut cflags = conf
            .build
            .cflags
            .as_ref()
            .map_or_else(|| vec![], |f| f.clone());
        let mut ldflags = conf
            .build
            .ldflags
            .as_ref()
            .map_or_else(|| vec![], |f| f.clone());

        let build = if release {
            &conf.release_build
        } else {
            &conf.debug_build
        };

        if let Some(c) = &build.cc {
            cc = Some(c)
        }
        if let Some(l) = &build.ld {
            ld = Some(l)
        }
        if let Some(cf) = &build.cflags {
            cflags.extend(cf.iter().map(|f| f.clone()));
        }
        if let Some(lf) = &build.ldflags {
            ldflags.extend(lf.iter().map(|f| f.clone()));
        }

        Self {
            cc: cc
                .map(|c| c.to_owned())
                .or(env::var("CC").ok())
                .unwrap_or_else(|| "cc".to_owned()),
            ld: ld
                .map(|c| c.to_owned())
                .or(env::var("LD").ok())
                .unwrap_or_else(|| "ld".to_owned()),
            cflags,
            ldflags,
            print_command: true,
        }
    }

    pub fn build(&self, dir: &DirStructure) -> Result<()> {
        let mut dep_deps = HashMap::new();
        let deps = get_dependencies(dir, &mut dep_deps)?;

        for file in &deps {
            if file.is_up_to_date()? {
                continue;
            }

            self.sync_build_file(
                file.direct.iter().map(|i| i.as_ref()),
                file.file.as_ref(),
            )?;
        }

        let bin_dep = Dependency {
            file: dir.binary().into(),
            direct: dir.objs().iter().map(|o| o.as_path().into()).collect(),
            indirect: HashSet::new(),
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
