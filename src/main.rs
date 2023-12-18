use dir_structure::DirStructure;
use err::Result;

mod dir_structure;
mod err;

fn main() -> Result<()> {
    let mut dir = DirStructure::new("main");
    dir.analyze(false)?;

    for s in dir.srcs() {
        println!("{s:?}");
    }

    println!();

    for o in dir.objs() {
        println!("{o:?}");
    }

    Ok(())
}
