// prost-build compilation for message-only Protocol Buffer generation
// This build script generates Rust structs for protobuf messages for direct IPC communication.
// No gRPC services are generated - we use direct IPC transport only.
// Output is placed in src/generated/ for stable imports.

use prost_build::Config;
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest.join("..").join("proto");
    let out_dir = manifest.join("src").join("generated");
    fs::create_dir_all(&out_dir)?;

    // 1) Gather & sort proto files for deterministic generation
    let mut files = vec![
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
        "mcp/unity/v1/ipc.proto",
        "mcp/unity/v1/ipc_control.proto", // New addition
    ]
    .into_iter()
    .map(|p| proto_root.join(p))
    .collect::<Vec<_>>();
    files.sort(); // Deterministic ordering

    // 2) Generate Rust code + FileDescriptorSet
    let descriptor_path = out_dir.join("schema.pb");
    let mut cfg = Config::new();
    cfg.out_dir(&out_dir);
    cfg.file_descriptor_set_path(&descriptor_path);
    cfg.compile_protos(&files, std::slice::from_ref(&proto_root))?;

    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
    }

    // 3) Generate schema hash as byte array (not hex string)
    let bytes = fs::read(&descriptor_path)?;
    let hash = Sha256::digest(&bytes);
    let hash_array: [u8; 32] = hash.into();

    fs::write(
        out_dir.join("schema_hash.rs"),
        format!("pub const SCHEMA_HASH: [u8; 32] = {:?};\n", hash_array),
    )?;

    Ok(())
}
