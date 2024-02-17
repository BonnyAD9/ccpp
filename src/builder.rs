use std::{
    collections::HashSet,
    fs, mem,
    path::PathBuf,
    process::{Child, Command},
    thread,
    time::Duration,
};

use crate::{
    compiler::Compiler,
    config::Config,
    dependency::{DepCache, DepFile, Dependency},
    err::{Error, Result},
    file_type::{FileState, FileType, Language},
};

pub struct Builder {
    /// Max number of threads running at the same time
    thread_count: usize,
    compiler: Compiler,
    print_command: bool,
    built: HashSet<DepFile>,
    dep_queue: Vec<Dependency>,
    command_queue: Vec<QCommand>,
    cache: DepCache,
    pool: Vec<(Child, QCommand)>,
}

struct QCommand {
    command: Command,
    requires: Vec<DepFile>,
    provides: Vec<DepFile>,
}

//===========================================================================//
//                                   Public                                  //
//===========================================================================//

impl Builder {
    pub fn from_config(conf: &Config, release: bool) -> Result<Self> {
        let build = if release {
            &conf.release_build
        } else {
            &conf.debug_build
        };

        Ok(Self {
            thread_count: std::thread::available_parallelism()
                .map_or(1, |t| t.get().checked_sub(2).unwrap_or(1)),
            compiler: Compiler::new(
                build.cc.clone(),
                build.cpp.clone(),
                &build.compiler_conf,
            )?,
            print_command: true,
            built: HashSet::new(),
            dep_queue: vec![],
            command_queue: vec![],
            cache: DepCache::new(),
            pool: vec![],
        })
    }

    pub fn build_all<P1, P2, I>(
        &mut self,
        target: P1,
        sources: I,
    ) -> Result<()>
    where
        P1: Into<PathBuf>,
        P2: Into<PathBuf>,
        I: IntoIterator<Item = P2>,
    {
        let mut lang = Language::C;
        let direct = sources
            .into_iter()
            .map(|s| {
                let res: DepFile = s.into().into();
                if matches!(
                    res.typ,
                    Some(FileType {
                        lang: Language::Cpp,
                        ..
                    })
                ) {
                    lang = Language::Cpp;
                }
                res
            })
            .collect();

        let file = DepFile {
            path: target.into().into(),
            typ: Some(FileType {
                lang,
                state: FileState::Executable,
            }),
        };

        let mut file = Dependency::new(file, direct, Default::default());

        self.cache.fill_dependency(&mut file)?;
        self.queue_target(file)?;
        self.build()
    }

    pub fn queue_target(&mut self, target: Dependency) -> Result<()> {
        if !target.is_up_to_date()? {
            self.dep_queue.push(target);
        }
        Ok(())
    }

    pub fn build(&mut self) -> Result<()> {
        let mut child_pool: Vec<(Child, QCommand)> = vec![];

        // don't return until all processes have exited

        let res = if let Err(e) = self.build_with_pool(&mut child_pool) {
            e
        } else {
            return Ok(());
        };

        // wait for all proceses to exit
        for (mut c, _) in child_pool {
            if c.wait().is_err() {
                // if kill fails, there is nothing we can do to exit the
                // process
                _ = c.kill();
            }
        }

        Err(res)
    }
}

impl Builder {
    fn build_with_pool(
        &mut self,
        pool: &mut Vec<(Child, QCommand)>,
    ) -> Result<()> {
        loop {
            match self.select_command() {
                Ok(Some(cmd)) => {
                    self.wait_and_run_command(pool, cmd)?;
                }
                Ok(None) => break,
                Err(Error::DependencyCycle) => {
                    if !self.wait_for_any(pool)? {
                        return Err(Error::DependencyCycle);
                    }
                }
                Err(e) => return Err(e),
            }
        }

        self.wait_for_all(pool)
    }

    fn select_command(&mut self) -> Result<Option<QCommand>> {
        let mut idx = None;

        for (i, c) in self.command_queue.iter_mut().enumerate().rev() {
            c.requires.retain(|i| !self.built.contains(i));
            if c.requires.is_empty() {
                idx = Some(i);
                break;
            }
        }

        if let Some(i) = idx {
            return Ok(Some(self.command_queue.remove(i)));
        }

        let mut cmd = None;

        while let Some(c) = self.fetch_command()? {
            if c.requires.is_empty() {
                cmd = Some(c);
                break;
            }
            self.command_queue.push(c);
        }

        if let Some(cmd) = cmd {
            return Ok(Some(cmd));
        }

        if self.command_queue.is_empty() {
            Ok(None)
        } else {
            Err(Error::DependencyCycle)
        }
    }

    fn fetch_command(&mut self) -> Result<Option<QCommand>> {
        let file = if let Some(file) = self.dep_queue.pop() {
            file
        } else {
            return Ok(None);
        };

        let resolved = file.file.clone();
        let (command, mut deps) = self.compiler.build(file)?;
        deps.retain(|d| {
            !self.built.contains(&d.file)
                && !self.pool.iter().any(|p| p.1.provides.contains(&d.file))
        });

        let mut i = 0;
        while i < deps.len() {
            self.cache.fill_dependency(&mut deps[i])?;
            if deps[i].is_up_to_date()? {
                deps.remove(i);
                continue;
            }
            i += 1;
        }

        let res = QCommand {
            command,
            requires: deps.iter().map(|d| d.file.clone()).collect(),
            provides: vec![resolved],
        };

        for d in deps.iter_mut() {
            self.cache.fill_dependency(d)?;
        }

        self.dep_queue.extend(deps.into_iter().rev());

        Ok(Some(res))
    }

    fn wait_and_run_command(
        &mut self,
        pool: &mut Vec<(Child, QCommand)>,
        mut cmd: QCommand,
    ) -> Result<()> {
        if pool.len() < self.thread_count {
            let child = cmd.run(self.print_command)?;
            pool.push((child, cmd));
            return Ok(());
        }

        'wait: loop {
            for run in pool.iter_mut() {
                if let Some(r) = run.0.try_wait()? {
                    if !r.success() {
                        return Err(Error::ProcessFailed(r.code()));
                    }
                    let child = cmd.run(self.print_command)?;
                    let run = mem::replace(run, (child, cmd));
                    self.built.extend(run.1.provides);
                    break 'wait;
                }
            }
            // Arbitrary sleep time so that the thread isn't using all its
            // power to just check in cycle that no processes exited.
            thread::sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    fn wait_for_any(
        &mut self,
        pool: &mut Vec<(Child, QCommand)>,
    ) -> Result<bool> {
        if pool.is_empty() {
            return Ok(false);
        }

        let idx = 'wait: loop {
            for (i, run) in pool.iter_mut().enumerate() {
                if let Some(r) = run.0.try_wait()? {
                    if !r.success() {
                        return Err(Error::ProcessFailed(r.code()));
                    }
                    break 'wait i;
                }
            }
            // Arbitrary sleep time so that the thread isn't using all its
            // power to just check in cycle that no processes exited.
            thread::sleep(Duration::from_millis(10));
        };

        let run = pool.swap_remove(idx);
        self.built.extend(run.1.provides);
        Ok(true)
    }

    fn wait_for_all(
        &mut self,
        pool: &mut Vec<(Child, QCommand)>,
    ) -> Result<()> {
        while let Some(mut cmd) = pool.pop() {
            let r = cmd.0.wait()?;
            if !r.success() {
                pool.push(cmd);
                return Err(Error::ProcessFailed(r.code()));
            }
        }

        Ok(())
    }
}

impl QCommand {
    fn run(&mut self, print: bool) -> Result<Child> {
        for r in &self.provides {
            if let Some(p) = r.parent() {
                fs::create_dir_all(p)?;
            }
        }
        if print {
            print!("{}", self.command.get_program().to_string_lossy());
            for a in self.command.get_args() {
                print!(" '{}'", a.to_string_lossy());
            }
            println!();
        }
        Ok(self.command.spawn()?)
    }
}
