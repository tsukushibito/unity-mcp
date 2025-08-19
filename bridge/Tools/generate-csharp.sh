#!/usr/bin/env bash
set -euo pipefail

# スクリプトのディレクトリを基準にrepo-rootを特定
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PROTO_ROOT="$REPO_ROOT/proto"
OUT="$REPO_ROOT/bridge/Packages/com.example.mcp-bridge/Editor/Generated/Proto"

mkdir -p "$OUT"

protoc-grpctools \
  -I"$PROTO_ROOT" \
  --csharp_out="$OUT" \
  --grpc_out="$OUT" \
  --plugin=protoc-gen-grpc=grpc_csharp_plugin \
  $(find "$PROTO_ROOT/mcp/unity/v1" -name '*.proto')

echo "[generate-csharp.sh] C# gRPC stubs generated into $OUT"
