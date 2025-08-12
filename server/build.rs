use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("..").join("proto").canonicalize()?;
    let files = [
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
    ]
    .into_iter()
    .map(|rel| proto_root.join(rel))
    .collect::<Vec<_>>();

    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&files, std::slice::from_ref(&proto_root))?;

    println!("cargo:rerun-if-changed={}", proto_root.display());
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
    }
    Ok(())
}
