use glob::glob;
use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=src/");

    let mut proto_files = Vec::new();

    let paths = glob("src/**/protobuf/**/*.proto").expect("Failed to read glob pattern");

    for entry in paths {
        match entry {
            Ok(path) => proto_files.push(path),
            Err(e) => println!("cargo:warning=Glob error: {e:?}"),
        }
    }

    prost_build::compile_protos(&proto_files, &["src/"])?;

    Ok(())
}
