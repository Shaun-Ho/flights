use glob::glob;
use std::env;
use std::io::Result;
use std::path::PathBuf;

/// Workspace root, relative to this crate. Used as the protoc include path so that
/// proto imports (e.g. `import "proto/parser/aircraft.proto"`) resolve from the repo root.
const WORKSPACE_ROOT: &str = "../..";

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed={WORKSPACE_ROOT}/proto");

    let mut proto_files = Vec::new();

    let paths =
        glob(&format!("{WORKSPACE_ROOT}/proto/**/*.proto")).expect("Failed to read glob pattern");

    for entry in paths {
        match entry {
            Ok(path) => proto_files.push(path),
            Err(e) => println!("cargo:warning=Glob error: {e:?}"),
        }
    }
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let descriptor_path = out_dir.join("file_descriptor_set.bin");

    let mut config = prost_build::Config::new();
    config.enable_type_names();
    config.bytes(["."]);
    config.file_descriptor_set_path(&descriptor_path);
    config.compile_protos(&proto_files, &[WORKSPACE_ROOT])?;

    Ok(())
}
