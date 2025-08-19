// prost-build compilation for message-only Protocol Buffer generation
// This build script generates Rust structs for protobuf messages without gRPC services.
// Output is placed in src/generated/ for stable imports.

use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_root = manifest_dir.join("..").join("proto");

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

    let out_dir = manifest_dir.join("src").join("generated");
    fs::create_dir_all(&out_dir).unwrap();

    println!("cargo:rerun-if-changed={}", proto_root.display());

    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);

    // IMPORTANT: We are NOT generating any gRPC services here.
    // This generates only the message types.
    config.compile_protos(
        &files.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        &[proto_root.as_path()],
    ).unwrap();
}