// tonic-build compilation API usage reference:
// https://docs.rs/tonic-build/latest/tonic_build/
//
// This build script uses tonic-prost-build which provides:
// - configure() -> Builder pattern for setting options
// - build_server(bool) -> Controls server stub generation
// - build_client(bool) -> Controls client stub generation
// - compile_protos(&[PathBuf], &[PathBuf]) -> Compiles proto files with include dirs
//
// Alternative APIs available:
// - tonic_build::compile_protos() for simple cases
// - Manual service builders for advanced scenarios

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("..").join("proto");

    let protos = &[
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
    ];

    let proto_files: Vec<_> = protos.iter().map(|p| proto_root.join(p)).collect();

    // Use the configure() builder pattern with all includes
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&proto_files, &[proto_root.clone()])?;

    println!("cargo:rerun-if-changed={}", proto_root.display());
    Ok(())
}
