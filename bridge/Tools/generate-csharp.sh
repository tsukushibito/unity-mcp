#!/usr/bin/env bash
set -euo pipefail

# スクリプトのディレクトリを基準にrepo-rootを特定
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PROTO_ROOT="$REPO_ROOT/proto"
OUT="$REPO_ROOT/bridge/Packages/com.example.mcp-bridge/Editor/Generated"

# Clean up old generated files to avoid conflicts
if [ -d "$OUT" ]; then
  echo "[generate-csharp.sh] Cleaning up old generated files..."
  rm -f "$OUT"/*.cs
  rm -rf "$OUT"/Proto
fi

mkdir -p "$OUT"

# Generate C# protobuf messages only (no gRPC stubs) for Direct IPC
protoc \
  -I="$PROTO_ROOT" \
  --csharp_out="$OUT" \
  --csharp_opt=base_namespace=Mcp.Unity.V1 \
  "$PROTO_ROOT"/mcp/unity/v1/common.proto \
  "$PROTO_ROOT"/mcp/unity/v1/editor_control.proto \
  "$PROTO_ROOT"/mcp/unity/v1/assets.proto \
  "$PROTO_ROOT"/mcp/unity/v1/build.proto \
  "$PROTO_ROOT"/mcp/unity/v1/operations.proto \
  "$PROTO_ROOT"/mcp/unity/v1/events.proto \
  "$PROTO_ROOT"/mcp/unity/v1/ipc.proto \
  "$PROTO_ROOT"/mcp/unity/v1/ipc_control.proto

echo "[generate-csharp.sh] C# Protobuf messages generated into $OUT"
