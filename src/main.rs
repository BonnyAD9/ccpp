use std::{
    fs, io, path::Path, process::{Command, ExitCode}
};

use arg_parser::{Action, Args};
use builder::Builder;
use config::Config;
use dir_structure::DirStructure;
use err::{Error, Result};
use termal::{formatc, gradient, printcln};

use crate::serde_config::{SerdeConfig, SerdeProject};

mod arg_parser;
mod builder;
mod compiler;
mod config;
mod dependency;
mod dir_structure;
mod err;
mod file_type;
mod include_deps;
mod serde_config;

const CONF_FILE: &str = "ccpp.toml";

fn main() -> ExitCode {
    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", formatc!("{'r}Failure:{'_} {}", e));
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

fn clean(_args: &Args) -> Result<()> {
    let conf = Config::from_toml_file(CONF_FILE)?;
    match fs::remove_dir_all(&conf.release_build.compiler_conf.bin_root) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => Err(e)?
    }
    match fs::remove_dir_all(&conf.debug_build.compiler_conf.bin_root) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => Err(e)?
    }
    Ok(())
}

fn build(args: &Args) -> Result<()> {
    let (conf, dir) = prepare(args)?;
    build_loaded(args, &conf, &dir)
}

fn run(args: &Args) -> Result<()> {
    let (conf, dir) = prepare(args)?;
    // printcln!("{'g bold}  Compiling{'_}");
    // printcln!("{'g bold}    Linking{'_}");
    build_loaded(args, &conf, &dir)?;
    printcln!("{'g bold}    Running{'_} {}", conf.project.name);
    run_loaded(args, &conf)
}

fn prepare(args: &Args) -> Result<(Config, DirStructure)> {
    let conf = Config::from_toml_file(CONF_FILE)?;
    let mut dir = DirStructure::from_config(&conf, args.release);
    dir.analyze()?;
    Ok((conf, dir))
}

fn build_loaded(args: &Args, conf: &Config, dir: &DirStructure) -> Result<()> {
    let mut bld = Builder::from_config(conf, args.release)?;
    let target = if args.release {
        &conf.release_build.target
    } else {
        &conf.debug_build.target
    };

    bld.build_all(target, dir.srcs())
}

fn run_loaded(args: &Args, conf: &Config) -> Result<()> {
    let target = if args.release {
        &conf.release_build.target
    } else {
        &conf.debug_build.target
    };

    Command::new(target)
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

    let conf = SerdeConfig {
        project: SerdeProject {
            name: name.into_owned(),
            src: None,
            bin: None,
        },
        ..SerdeConfig::default()
    };

    let conf_path = dir.join("ccpp.toml");
    let src_path = dir.join("src");
    conf.to_toml_file(conf_path)?;
    if !src_path.exists() {
        fs::create_dir_all(&src_path)?;
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
    printcln!(
        "Welcome to {'g i}ccpp{'_} help by {}{'_}
Version: {}

{'g}Usage:
  {'w}ccpp <action>{'_} {'gr}[flags] [-- [arg] [arg] ...]

{'g}Actions:
  {'y}help  h  -h  -?  --help{'_}
    Shows this help.

  {'y}clean{'_}
    Delete all compiled files (binary and object files).

  {'y}build{'_}
    Build the source code.

  {'y}run{'_}
    Build the source and run the app with the arguments after `--`.

  {'y}new {'w}<project folder>{'_}
    Create a new project in the given folder. The project name will be the
    folder name. If the folder doesn't exist, it is created.

{'g}Flags:
  {'y}-r  --release{'_}
    Build/run in release mode.
",
        gradient("BonnyAD9", (250, 50, 170), (180, 50, 240)),
        v.unwrap_or("unknown")
    );
    Ok(())
}

fn debug_code(_args: &Args) -> Result<()> {
    /*
    let (_conf, dir) = prepare(args)?;
    let deps = get_dependencies(&dir)?;
    for dep in &deps {
        println!("{:?}", dep);
    }*/
    Ok(())
}
