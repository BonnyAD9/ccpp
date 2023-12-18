use builder::Builder;
use dir_structure::DirStructure;
use err::Result;

mod builder;
mod dependency;
mod dir_structure;
mod err;

fn main() -> Result<()> {
    let mut dir = DirStructure::new("main");
    dir.analyze(false)?;

    let bld = Builder {
        cc: "cc".into(),
        ld: "cc".into(),
        cflags: vec!["-g".into(), "-O3".into(), "-fsanitize=address".into()],
        ldflags: vec!["-fsanitize=address".into()],
        print_command: true,
    };

    bld.build(&dir)?;

    Ok(())
}
