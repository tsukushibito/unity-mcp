#!/usr/bin/env bash
set -euo pipefail

# Protocol Buffer generation for Rust MCP Server
# This script replaces the build.rs functionality for generating Rust structs from protobuf messages.
# Run this manually when proto files change, then commit the generated files.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SERVER_ROOT="$REPO_ROOT/server"
PROTO_ROOT="$REPO_ROOT/proto"
OUT_DIR="$SERVER_ROOT/src/generated"

echo "[generate-rust-proto.sh] Starting Protocol Buffer generation..."
echo "  Proto root: $PROTO_ROOT"
echo "  Output dir: $OUT_DIR"

# Ensure output directory exists
mkdir -p "$OUT_DIR"

# Check required tools
if ! command -v protoc &> /dev/null; then
    echo "Error: protoc (Protocol Buffer Compiler) not found. Please install it:"
    echo "  Ubuntu/Debian: sudo apt-get install protobuf-compiler"
    echo "  macOS: brew install protobuf"
    echo "  Or download from: https://grpc.io/docs/protoc-installation/"
    exit 1
fi

# Gather proto files in deterministic order (same as build.rs)
PROTO_FILES=(
    "mcp/unity/v1/common.proto"
    "mcp/unity/v1/editor_control.proto"
    "mcp/unity/v1/assets.proto"
    "mcp/unity/v1/prefab.proto"
    "mcp/unity/v1/build.proto"
    "mcp/unity/v1/operations.proto"
    "mcp/unity/v1/events.proto"
    "mcp/unity/v1/ipc.proto"
    "mcp/unity/v1/ipc_control.proto"
    "mcp/unity/v1/tests.proto"
)

# Verify all proto files exist
echo "[generate-rust-proto.sh] Verifying proto files..."
for proto_file in "${PROTO_FILES[@]}"; do
    full_path="$PROTO_ROOT/$proto_file"
    if [ ! -f "$full_path" ]; then
        echo "Error: Proto file not found: $full_path"
        exit 1
    fi
    echo "  ✓ $proto_file"
done

# Generate FileDescriptorSet for schema hash calculation
DESCRIPTOR_PATH="$OUT_DIR/schema.pb"
echo "[generate-rust-proto.sh] Generating FileDescriptorSet..."
protoc \
    -I="$PROTO_ROOT" \
    --descriptor_set_out="$DESCRIPTOR_PATH" \
    --include_imports \
    "${PROTO_FILES[@]/#/$PROTO_ROOT/}"

# Generate Rust structs using protoc directly
echo "[generate-rust-proto.sh] Generating Rust code with protoc..."

# Use a temporary directory for prost-build based generation
TEMP_DIR=$(mktemp -d)
TEMP_CARGO_TOML="$TEMP_DIR/Cargo.toml"
TEMP_SRC_DIR="$TEMP_DIR/src"
TEMP_MAIN_RS="$TEMP_SRC_DIR/main.rs"

echo "Creating temporary prost-build project in $TEMP_DIR..."
mkdir -p "$TEMP_SRC_DIR"

# Create temporary Cargo.toml
cat > "$TEMP_CARGO_TOML" << 'EOF'
[package]
name = "generate_proto_temp"
version = "0.1.0"
edition = "2021"

[dependencies]
prost-build = "0.14.1"
EOF

# Create temporary main.rs for generation
cat > "$TEMP_MAIN_RS" << EOF
use prost_build::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = "$PROTO_ROOT";
    let out_dir = "$OUT_DIR";
    
    let files = [
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/prefab.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
        "mcp/unity/v1/ipc.proto",
        "mcp/unity/v1/ipc_control.proto",
        "mcp/unity/v1/tests.proto",
    ];
    
    let full_paths: Vec<_> = files.iter()
        .map(|f| format!("{}/{}", proto_root, f))
        .collect();
        
    let mut cfg = Config::new();
    cfg.out_dir(&out_dir);
    cfg.compile_protos(&full_paths, &[&proto_root])?;
    
    println!("Generated Rust protobuf code in {}", out_dir);
    Ok(())
}
EOF

# Run the generation
echo "Running prost-build generation..."
cd "$TEMP_DIR"
cargo run --quiet

# Clean up temporary directory
cd "$SERVER_ROOT"
rm -rf "$TEMP_DIR"

# Verify generated files exist (check for any .rs files)
echo "Checking generated Rust files..."
if ls "$OUT_DIR"/*.rs 1> /dev/null 2>&1; then
    echo "Generated Rust files:"
    ls -la "$OUT_DIR"/*.rs
else
    echo "Error: No Rust files were generated"
    echo "This might indicate a compatibility issue with protoc output format"
    echo "Please verify protoc installation and plugin availability"
    exit 1
fi

# Calculate schema hash
echo "[generate-rust-proto.sh] Calculating schema hash..."
if command -v sha256sum &> /dev/null; then
    HASH_HEX=$(sha256sum "$DESCRIPTOR_PATH" | cut -d' ' -f1)
elif command -v shasum &> /dev/null; then
    HASH_HEX=$(shasum -a 256 "$DESCRIPTOR_PATH" | cut -d' ' -f1)
else
    echo "Error: Neither sha256sum nor shasum found. Please install coreutils."
    exit 1
fi

# Convert hex hash to byte array format
HASH_BYTES="["
for i in $(seq 0 2 62); do
    if [ $i -gt 0 ]; then
        HASH_BYTES="$HASH_BYTES, "
    fi
    byte_hex="${HASH_HEX:$i:2}"
    byte_dec=$((16#$byte_hex))
    HASH_BYTES="$HASH_BYTES$byte_dec"
done
HASH_BYTES="$HASH_BYTES]"

# Generate schema_hash.rs
cat > "$OUT_DIR/schema_hash.rs" << EOF
pub const SCHEMA_HASH: [u8; 32] = $HASH_BYTES;
EOF


echo "[generate-rust-proto.sh] Protocol Buffer generation completed successfully!"
echo "Generated files:"
echo "  ✓ $OUT_DIR/mcp.unity.v1.rs"
echo "  ✓ $OUT_DIR/schema.pb"
echo "  ✓ $OUT_DIR/schema_hash.rs"
echo ""
echo "Next steps:"
echo "  1. Review the generated files"
echo "  2. Test with: cd $SERVER_ROOT && cargo build"
echo "  3. Commit changes: git add src/generated/ && git commit -m 'regenerate proto files'"