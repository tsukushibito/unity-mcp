#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKGS="$ROOT/Tools/packages"
DEST="$ROOT/Packages/com.example.mcp-bridge/Editor/Plugins/Grpc"
mkdir -p "$DEST"

pushd "$ROOT/tools" >/dev/null
dotnet restore grpc-client-runtime.csproj
popd >/dev/null

copy_one () { # <pkgid> <ver> <dllname>
  local base="$PKGS/$1/$2"; local dll="$3.dll"; local src=""
  for p in \
    "$base/lib/netstandard2.1/$dll" \
    "$base/lib/netstandard2.0/$dll" \
    "$base/lib/net462/$dll" \
    "$base/runtimes/unix/lib/netstandard2.0/$dll" \
    "$base/runtimes/any/lib/netstandard2.0/$dll"
  do
    [[ -f "$p" ]] && src="$p" && break
  done
  [[ -z "${src:-}" ]] && echo "Not found: $1 $dll" && exit 1
  cp -f "$src" "$DEST/"
  echo "Copied: $(basename "$src")"
}

copy_one google.protobuf     3.32.0  Google.Protobuf
copy_one grpc.core.api       2.71.0  Grpc.Core.Api
copy_one grpc.net.common     2.71.0  Grpc.Net.Common   # ← タイポ注意: 正しくは Grpc.Net.Common
copy_one grpc.net.client     2.71.0  Grpc.Net.Client
copy_one grpc.net.client.web 2.71.0  Grpc.Net.Client.Web

echo "DLLs copied to $DEST"
