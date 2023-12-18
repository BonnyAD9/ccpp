use dependency::get_dependencies;
use dir_structure::DirStructure;
use err::Result;

mod dependency;
mod dir_structure;
mod err;

fn main() -> Result<()> {
    let mut dir = DirStructure::new("main");
    dir.analyze(false)?;

    let file_deps = get_dependencies(&dir)?;

    for file_dep in file_deps {
        print!("{:?}:", file_dep.file);
        for dep in &file_dep.deps {
            print!(" {dep:?}");
        }
        println!();
    }

    Ok(())
}
