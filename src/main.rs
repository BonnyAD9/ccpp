use builder::Builder;
use config::Config;
use dir_structure::DirStructure;
use err::Result;

mod builder;
mod config;
mod dependency;
mod dir_structure;
mod err;

fn main() -> Result<()> {
    let conf = Config::from_toml_file("ccpp.toml")?;

    let mut dir = DirStructure::new("main");
    dir.analyze(false)?;

    let bld = Builder::from_config(&conf, false);

    bld.build(&dir)?;

    Ok(())
}
