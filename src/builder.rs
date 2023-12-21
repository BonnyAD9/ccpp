use std::{
    collections::{HashMap, HashSet},
    env,
    ffi::OsStr,
    fs::create_dir_all,
    path::Path,
    process::{Child, Command},
    thread,
    time::Duration,
};

use termal::{printc, printcln};

use crate::{
    config::Config,
    dependency::{get_dependencies, Dependency},
    dir_structure::DirStructure,
    err::{Error, Result},
};

pub struct Builder {
    /// Max number of threads running at the same time
    pub thread_count: usize,
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
            thread_count: std::thread::available_parallelism()
                .map_or(1, |t| t.get().checked_sub(2).unwrap_or(1)),
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

        let mut to_build = vec![];

        for file in deps {
            if !file.is_up_to_date()? {
                to_build.push(file);
            }
        }

        if self.thread_count <= 1 {
            for file in to_build {
                self.sync_build_file(
                    file.direct.iter().map(|i| i.as_ref()),
                    file.file.as_ref(),
                )?;
            }
        } else {
            let mut threads = vec![];
            if let Err(e) = self.parallel_build(to_build, &mut threads) {
                for mut thread in threads {
                    if let Err(e) = thread.wait() {
                        printcln!(
                            "      {'r bold}Error{'_}\
                            failed to wait for thread: {}",
                            e
                        );
                        _ = thread.kill();
                    }
                }
                return Err(e);
            }
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
    fn parallel_build(
        &self,
        files: Vec<Dependency>,
        threads: &mut Vec<Child>,
    ) -> Result<()> {
        const TIMEOUT: Duration = Duration::from_millis(10);

        let mut files = files.into_iter();

        while let Some(file) = files.next() {
            threads.push(self.start_build_file(
                file.direct.iter().map(|i| i.as_ref()),
                &file.file,
            )?);
            if threads.len() == self.thread_count {
                break;
            }
        }

        'files: while let Some(file) = files.next() {
            for t in threads.iter_mut() {
                if let Some(res) = t.try_wait()? {
                    if !res.success() {
                        return Err(Error::ProcessFailed(res.code()));
                    }
                    *t = self.start_build_file(
                        file.direct.iter().map(|i| i.as_ref()),
                        &file.file,
                    )?;
                    continue 'files;
                }
            }
            thread::sleep(TIMEOUT);
        }

        for thread in threads.into_iter() {
            let res = thread.wait()?;
            if !res.success() {
                return Err(Error::ProcessFailed(res.code()));
            }
        }

        Ok(())
    }

    fn start_build_file<'a, S, I>(
        &self,
        input: I,
        output: &Path,
    ) -> Result<Child>
    where
        S: AsRef<OsStr>,
        I: Iterator<Item = S> + Clone,
    {
        if let Some(d) = output.parent() {
            create_dir_all(d)?;
        }

        let mut cmd = Command::new(&self.cc);
        cmd.args(self.cflags.iter())
            .arg("-o")
            .arg(output)
            .arg("-c")
            .args(input.clone());

        if self.print_command {
            printc!("{'g bold}  Compiling{'_}");
            for f in input {
                print!(" {}", f.as_ref().to_string_lossy());
            }
            println!();
        }
        Ok(cmd.spawn()?)
    }

    fn sync_build_file<'a, S, I>(&self, input: I, output: &Path) -> Result<()>
    where
        S: AsRef<OsStr>,
        I: Iterator<Item = S> + Clone,
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
            printcln!("{'g bold}    Linking{'_} {}", output.to_string_lossy());
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
