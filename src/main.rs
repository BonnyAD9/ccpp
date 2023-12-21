use std::{
    collections::HashMap,
    fs::{self, create_dir_all, remove_dir_all, remove_file},
    path::Path,
    process::{Command, ExitCode},
};

use arg_parser::{Action, Args};
use builder::Builder;
use config::{Config, Project};
use dependency::get_dependencies;
use dir_structure::DirStructure;
use err::{Error, Result};

mod arg_parser;
mod builder;
mod config;
mod dependency;
mod dir_structure;
mod err;
mod include_deps;

const CONF_FILE: &str = "ccpp.toml";

fn main() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            println!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<()> {
    let args = Args::get()?;
    match &args.action {
        Action::None => debug_code(&args),
        Action::Clean => clean(&args),
        Action::Build => build(&args),
        Action::Run => run(&args),
        Action::Help => help(&args),
        Action::New(dir) => new(&args, &dir),
    }
}

fn clean(args: &Args) -> Result<()> {
    let conf = Config::from_toml_file(CONF_FILE)?;
    let dir = DirStructure::from_config(&conf, args.release);

    if dir.rel_obj().exists() {
        remove_dir_all(dir.rel_obj())?;
    }
    if dir.deb_obj().exists() {
        remove_dir_all(dir.deb_obj())?;
    }
    if dir.rel_bin().exists() {
        remove_file(dir.rel_bin())?;
    }
    if dir.deb_bin().exists() {
        remove_file(dir.deb_bin())?;
    }

    Ok(())
}

fn build(args: &Args) -> Result<()> {
    let (conf, dir) = prepare(args)?;
    build_loaded(args, &conf, &dir)
}

fn run(args: &Args) -> Result<()> {
    let (conf, dir) = prepare(args)?;
    build_loaded(args, &conf, &dir)?;
    run_loaded(args, &conf, &dir)
}

fn prepare(args: &Args) -> Result<(Config, DirStructure)> {
    let conf = Config::from_toml_file(CONF_FILE)?;
    let mut dir = DirStructure::from_config(&conf, args.release);
    dir.analyze(args.release)?;
    Ok((conf, dir))
}

fn build_loaded(args: &Args, conf: &Config, dir: &DirStructure) -> Result<()> {
    let bld = Builder::from_config(conf, args.release);
    bld.build(dir)
}

fn run_loaded(args: &Args, _conf: &Config, dir: &DirStructure) -> Result<()> {
    Command::new(dir.binary())
        .args(args.app_args.iter())
        .spawn()?
        .wait()?;
    Ok(())
}

fn new(_args: &Args, dir: &Path) -> Result<()> {
    let name = if let Some(name) = dir.file_name() {
        name.to_string_lossy()
    } else {
        return Err(Error::Generic(format!(
            "Couldn't get the directory name of {dir:?}"
        )));
    };

    let conf = Config {
        project: Project {
            name: name.into_owned(),
        },
        ..Config::default()
    };

    let conf_path = dir.join("ccpp.toml");
    let src_path = dir.join("src");
    conf.to_toml_file(conf_path)?;
    if !src_path.exists() {
        create_dir_all(&src_path)?;
        fs::write(
            src_path.join("main.c"),
            "#include <stdio.h>

int main(void) {
    printf(\"Hello World!\\n\");
}
",
        )?;
        fs::write(dir.join(".gitignore"), "bin\n")?;
    }

    Ok(())
}

fn help(_args: &Args) -> Result<()> {
    let v: Option<&str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "Welcome to ccpp help by BonnyAD9
Version: {}

Usage:
  ccpp <action> [flags] [-- [arg] [arg] ...]

Actions:
  help  h  -h  -?  --help
    Shows this help.

  clean
    Delete all compiled files (binary and object files).

  build
    Build the source code.

  run
    Build the source and run the app with the arguments after `--`.

  new [project folder]
    Create a new project in the given folder. The project name will be the
    folder name. If the folder doesn't exist, it is created.

Flags:
  -r  --release
    Build in release mode.
",
        v.unwrap_or("unknown")
    );
    Ok(())
}

fn debug_code(args: &Args) -> Result<()> {
    let (_conf, dir) = prepare(args)?;
    let mut dep_dep = HashMap::new();
    let deps = get_dependencies(&dir, &mut dep_dep)?;
    for dep in &deps {
        println!("{:?}", dep);
    }
    Ok(())
}
