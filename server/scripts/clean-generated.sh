#!/usr/bin/env bash
set -euo pipefail

# Clean up generated Protocol Buffer files
# Use this when you want to force a complete regeneration of proto files

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SERVER_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUT_DIR="$SERVER_ROOT/src/generated"

echo "[clean-generated.sh] Cleaning generated Protocol Buffer files..."
echo "  Target directory: $OUT_DIR"

if [ ! -d "$OUT_DIR" ]; then
    echo "  Nothing to clean - directory doesn't exist"
    exit 0
fi

# Remove generated files
FILES_TO_REMOVE=(
    "mcp.unity.v1.rs"
    "schema.pb"
    "schema_hash.rs"
)

for file in "${FILES_TO_REMOVE[@]}"; do
    full_path="$OUT_DIR/$file"
    if [ -f "$full_path" ]; then
        echo "  Removing: $file"
        rm -f "$full_path"
    else
        echo "  Skip (not found): $file"
    fi
done

echo "[clean-generated.sh] Cleanup completed!"
echo ""
echo "Next steps:"
echo "  1. Run: ./scripts/generate-rust-proto.sh"
echo "  2. Test: cargo build"
echo "  3. Commit: git add src/generated/ && git commit -m 'regenerate proto files'"