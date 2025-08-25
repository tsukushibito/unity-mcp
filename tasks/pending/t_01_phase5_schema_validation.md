# T01 Phase5: Schema Validation実装

**Status:** Ready for implementation  
**Priority:** High  
**Estimated effort:** 3-4 hours  
**Depends on:** Phase4 (Feature Negotiation)

## 目標と成果物

T01仕様のschema hash validation mechanismを実装し、クライアント・サーバー間でProtocol Bufferスキーマの互換性を保証する。

### 成果物
- [ ] SHA-256ベースのschema hash計算
- [ ] FileDescriptorSet生成とprocessing
- [ ] Build-time schema hash generation
- [ ] Runtime schema validation
- [ ] Schema mismatch error handling

## 前提条件

- Phase4完了（Feature negotiation動作確認済み）
- T01仕様のSection 4 (Schema Hash)理解
- protoc toolchainの理解
- prost build processの理解

## Schema Hash計算仕様

T01 Section 4に基づく実装：

```
schema_hash = SHA-256( FileDescriptorSet )

FileDescriptorSet生成条件：
- inputs: proto/mcp/unity/v1/*.proto (プロジェクトで使用される全ファイル)
- flags: --include_imports=true, --include_source_info=false
- protoc version: ≥ 3.21 (一貫性のため)
```

## 実装手順

### Step 1: Build-time Schema Hash Generation

`server/build.rs`にschema hash生成機能を追加：

```rust
use std::{env, fs, path::PathBuf, process::Command};
use sha2::{Sha256, Digest};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_root = manifest_dir.join("..").join("proto");
    let out_dir = manifest_dir.join("src").join("generated");
    
    // Gather proto files for deterministic generation
    let mut files = [
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto", 
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
        "mcp/unity/v1/ipc.proto",
        "mcp/unity/v1/ipc_control.proto",
    ]
    .into_iter()
    .map(|rel| proto_root.join(rel))
    .collect::<Vec<_>>();
    files.sort(); // ★ 決定性確保のため必須（パス文字列昇順）

    fs::create_dir_all(&out_dir).unwrap();
    println!("cargo:rerun-if-changed={}", proto_root.display());

    // 1. Generate protobuf code (existing)
    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    config.compile_protos(
        &files.iter().map(PathBuf::as_path).collect::<Vec<_>>(),
        &[proto_root.as_path()],
    ).unwrap();

    // 2. Generate FileDescriptorSet for schema hash
    let descriptor_set_path = out_dir.join("descriptor_set.bin");
    generate_descriptor_set(&files, &proto_root, &descriptor_set_path);
    
    // 3. Calculate schema hash
    let schema_hash = calculate_schema_hash(&descriptor_set_path);
    
    // 4. Generate schema hash constant
    let schema_hash_code = format!(
        r#"
// Generated schema hash - DO NOT EDIT
pub const SCHEMA_HASH: &[u8] = &{:?};
pub const SCHEMA_HASH_HEX: &str = "{}";
"#,
        schema_hash.as_slice(),
        hex::encode(&schema_hash)
    );
    
    let schema_hash_file = out_dir.join("schema_hash.rs");
    fs::write(schema_hash_file, schema_hash_code).unwrap();
    
    println!("cargo:rustc-env=SCHEMA_HASH={}", hex::encode(&schema_hash));
}

fn generate_descriptor_set(
    proto_files: &[PathBuf], 
    proto_root: &PathBuf, 
    output_path: &PathBuf
) {
    let mut cmd = Command::new("protoc");
    
    cmd.arg("--include_imports")
       .arg("--include_source_info=false")
       .arg(format!("--descriptor_set_out={}", output_path.display()))
       .arg(format!("--proto_path={}", proto_root.display()));
    
    for file in proto_files {
        cmd.arg(file);
    }
    
    let output = cmd.output().expect("Failed to execute protoc");
    
    if !output.status.success() {
        panic!(
            "protoc failed with stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn calculate_schema_hash(descriptor_set_path: &PathBuf) -> Vec<u8> {
    let descriptor_bytes = fs::read(descriptor_set_path)
        .expect("Failed to read descriptor set");
    
    let mut hasher = Sha256::new();
    hasher.update(&descriptor_bytes);
    hasher.finalize().to_vec()
}
```

### Step 2: Runtime Schema Hash Access

`server/src/ipc/codec.rs`のschema_hash関数を実装：

```rust
use bytes::Bytes;
use prost::Message;
use thiserror::Error;

use crate::generated::mcp::unity::v1 as pb;

// Include generated schema hash
include!(concat!(env!("OUT_DIR"), "/schema_hash.rs"));

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("decode error: {0}")]
    Decode(#[from] prost::DecodeError),
}

pub fn encode_envelope(env: &pb::IpcEnvelope) -> Result<Bytes, CodecError> {
    let mut buf = bytes::BytesMut::with_capacity(env.encoded_len());
    env.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode_envelope(b: Bytes) -> Result<pb::IpcEnvelope, CodecError> {
    pb::IpcEnvelope::decode(b).map_err(CodecError::Decode)
}

pub fn encode_control(control: &pb::IpcControl) -> Result<Bytes, CodecError> {
    let mut buf = bytes::BytesMut::with_capacity(control.encoded_len());
    control.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode_control(b: Bytes) -> Result<pb::IpcControl, CodecError> {
    pb::IpcControl::decode(b).map_err(CodecError::Decode)
}

pub fn schema_hash() -> Vec<u8> {
    SCHEMA_HASH.to_vec()
}

pub fn schema_hash_hex() -> &'static str {
    SCHEMA_HASH_HEX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_hash_properties() {
        let hash = schema_hash();
        assert_eq!(hash.len(), 32); // SHA-256 is 32 bytes
        
        let hex = schema_hash_hex();
        assert_eq!(hex.len(), 64); // 32 bytes * 2 hex chars
        assert_eq!(hex, hex::encode(&hash));
    }
}
```

### Step 3: Unity側Schema Hash生成

Unity側でも同様のschema hash計算を実装：

```csharp
// Unity Editor script for schema hash generation
using System;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using UnityEngine;
using UnityEditor;

public static class SchemaHashGenerator
{
    private const string DESCRIPTOR_SET_PATH = "Packages/com.example.mcp-bridge/Runtime/Generated/descriptor_set.bin";
    private const string SCHEMA_HASH_FILE = "Packages/com.example.mcp-bridge/Runtime/Generated/SchemaHash.cs";
    
    [MenuItem("MCP/Generate Schema Hash")]
    public static void GenerateSchemaHash()
    {
        try
        {
            // Generate descriptor set using protoc
            GenerateDescriptorSet();
            
            // Calculate hash
            var hash = CalculateSchemaHash();
            
            // Generate C# code
            GenerateSchemaHashCode(hash);
            
            Debug.Log($"Schema hash generated: {BitConverter.ToString(hash).Replace("-", "").ToLower()}");
            AssetDatabase.Refresh();
        }
        catch (Exception ex)
        {
            Debug.LogError($"Failed to generate schema hash: {ex.Message}");
        }
    }
    
    private static void GenerateDescriptorSet()
    {
        var protoRoot = Path.Combine(Application.dataPath, "..", "proto");
        var outputPath = Path.Combine(Application.dataPath, "..", DESCRIPTOR_SET_PATH);
        
        var protoFiles = new[]
        {
            "mcp/unity/v1/common.proto",
            "mcp/unity/v1/editor_control.proto",
            "mcp/unity/v1/assets.proto",
            "mcp/unity/v1/build.proto",
            "mcp/unity/v1/operations.proto",
            "mcp/unity/v1/events.proto",
            "mcp/unity/v1/ipc.proto",
            "mcp/unity/v1/ipc_control.proto",
        };
        
        var args = new StringBuilder();
        args.Append("--include_imports ");
        args.Append("--include_source_info=false ");
        args.Append($"--descriptor_set_out=\"{outputPath}\" ");
        args.Append($"--proto_path=\"{protoRoot}\" ");
        
        foreach (var file in protoFiles)
        {
            args.Append($"\"{Path.Combine(protoRoot, file)}\" ");
        }
        
        var process = new System.Diagnostics.Process
        {
            StartInfo = new System.Diagnostics.ProcessStartInfo
            {
                FileName = "protoc",
                Arguments = args.ToString(),
                UseShellExecute = false,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
            }
        };
        
        process.Start();
        process.WaitForExit();
        
        if (process.ExitCode != 0)
        {
            var error = process.StandardError.ReadToEnd();
            throw new Exception($"protoc failed: {error}");
        }
    }
    
    private static byte[] CalculateSchemaHash()
    {
        var descriptorPath = Path.Combine(Application.dataPath, "..", DESCRIPTOR_SET_PATH);
        var descriptorBytes = File.ReadAllBytes(descriptorPath);
        
        using (var sha256 = SHA256.Create())
        {
            return sha256.ComputeHash(descriptorBytes);
        }
    }
    
    private static void GenerateSchemaHashCode(byte[] hash)
    {
        var hexHash = BitConverter.ToString(hash).Replace("-", "").ToLower();
        var code = $@"
// Generated schema hash - DO NOT EDIT
// This file is automatically generated by SchemaHashGenerator
using System;

namespace MCP.Bridge.Generated
{{
    public static class SchemaHash
    {{
        public static readonly byte[] Hash = new byte[]
        {{
            {string.Join(", ", Array.ConvertAll(hash, b => $"0x{b:x2}"))}
        }};
        
        public const string HexHash = ""{hexHash}"";
        
        public static bool Matches(byte[] other)
        {{
            if (other == null || other.Length != Hash.Length)
                return false;
                
            for (int i = 0; i < Hash.Length; i++)
            {{
                if (Hash[i] != other[i])
                    return false;
            }}
            return true;
        }}
    }}
}}
";
        
        var outputPath = Path.Combine(Application.dataPath, "..", SCHEMA_HASH_FILE);
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath));
        File.WriteAllText(outputPath, code);
    }
}
```

### Step 4: Handshake中のSchema Validation

Rust client側でschema hashをhandshakeに含める：

```rust
// client.rs内のhandshake処理を更新
async fn spawn_io(/* ... */) -> Result<(), IpcError> {
    // ... existing connection code ...
    
    let hello = pb::IpcHello {
        token: inner.cfg.token.clone().unwrap_or_default(),
        ipc_version: "1.0".to_string(),
        features: desired_features.to_strings(),
        schema_hash: codec::schema_hash(), // 実際のschema hash
        project_root: inner.cfg.project_root.clone().unwrap_or_default(),
        client_name: "unity-mcp-rs".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        meta: create_default_meta(),
    };
    
    // ... send hello and receive welcome ...
    
    // Validate server's schema hash
    if welcome.schema_hash != codec::schema_hash() {
        return Err(IpcError::SchemaMismatch(format!(
            "schema hash mismatch: client={}, server={}",
            hex::encode(codec::schema_hash()),
            hex::encode(&welcome.schema_hash)
        )));
    }
    
    // ... continue processing ...
}
```

Unity server側でのschema validation：

```csharp
private ValidationResult ValidateSchemaHash(byte[] clientSchemaHash)
{
    if (clientSchemaHash == null || clientSchemaHash.Length == 0)
    {
        return ValidationResult.Error(
            IpcReject.Types.Code.FailedPrecondition, 
            "missing schema_hash"
        );
    }
    
    if (!SchemaHash.Matches(clientSchemaHash))
    {
        var clientHex = BitConverter.ToString(clientSchemaHash).Replace("-", "").ToLower();
        return ValidationResult.Error(
            IpcReject.Types.Code.FailedPrecondition,
            $"schema_hash mismatch; client={clientHex} server={SchemaHash.HexHash}"
        );
    }
    
    return ValidationResult.Success();
}

## Schema Hash表現ルール

**データ型と処理方針:**
- 比較は `bytes(32)` の **生バイトで実施**
- ログやエラーメッセージでは **先頭8桁のHex** を表示する
- protobuf送受信は生バイト、デバッグ表示のみhex変換

**実装例:**
```rust
// Rust: 比較は生バイト、ログは短縮hex
if welcome.schema_hash.as_slice() != codec::schema_hash() {
    return Err(IpcError::SchemaMismatch(format!(
        "schema hash mismatch: client={}, server={}",
        hex::encode(codec::schema_hash())[..8].to_string(), // 8桁のみ
        hex::encode(&welcome.schema_hash)[..8].to_string()
    )));
}
```

```csharp
// Unity: 比較は生バイト、ログは短縮hex
if (!SchemaHash.Matches(clientSchemaHash)) {
    return ValidationResult.Error(
        IpcReject.Types.Code.FailedPrecondition,
        $"schema_hash mismatch; client={SchemaHash.ToShortHex(clientSchemaHash)} server={SchemaHash.HexHash[..8]}"
    );
}
```

private IpcWelcome CreateWelcome(IpcHello hello)
{
    // ... existing feature negotiation ...
    
    return new IpcWelcome
    {
        IpcVersion = hello.IpcVersion,
        AcceptedFeatures = { acceptedFeatures },
        SchemaHash = ByteString.CopyFrom(SchemaHash.Hash), // Server's schema hash
        ServerName = "unity-editor-bridge",
        ServerVersion = GetPackageVersion(),
        EditorVersion = Application.unityVersion,
        SessionId = Guid.NewGuid().ToString(),
        Meta = { { "platform", Application.platform.ToString() } }
    };
}
```

### Step 5: Development/CI Integration

Proto変更時の自動schema hash更新：

```bash
# scripts/update-schema-hash.sh
#!/bin/bash
set -e

echo "Updating schema hash after proto changes..."

# Rust side
cd server
cargo clean
cargo build

# Unity side (requires Unity in PATH)
Unity -quit -batchmode -projectPath ../bridge -executeMethod SchemaHashGenerator.GenerateSchemaHash

echo "Schema hash updated successfully"
```

CI/CDでのschema hash validation：

```yaml
# .github/workflows/schema-validation.yml
name: Schema Validation
on: [push, pull_request]

jobs:
  schema-validation:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Setup protoc
      run: |
        sudo apt-get update
        sudo apt-get install -y protobuf-compiler
        protoc --version
    
    - name: Generate Rust schema hash
      run: |
        cd server
        cargo build
        RUST_HASH=$(cargo run --bin schema-hash-tool)
        echo "Rust schema hash: $RUST_HASH"
        echo "RUST_HASH=$RUST_HASH" >> $GITHUB_ENV
    
    - name: Generate Unity schema hash  
      run: |
        # Unity schema hash generation
        UNITY_HASH=$(./scripts/generate-unity-schema-hash.sh)
        echo "Unity schema hash: $UNITY_HASH"
        echo "UNITY_HASH=$UNITY_HASH" >> $GITHUB_ENV
    
    - name: Validate schema hash consistency
      run: |
        if [ "$RUST_HASH" != "$UNITY_HASH" ]; then
          echo "ERROR: Schema hash mismatch!"
          echo "Rust:  $RUST_HASH"
          echo "Unity: $UNITY_HASH"
          exit 1
        fi
        echo "Schema hash validation passed: $RUST_HASH"
```

## テスト要件

### Schema Hash Tests

```rust
#[test]
fn test_schema_hash_deterministic() {
    let hash1 = codec::schema_hash();
    let hash2 = codec::schema_hash();
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 32); // SHA-256
}

#[test]
fn test_schema_hash_hex_format() {
    let hash = codec::schema_hash();
    let hex = codec::schema_hash_hex();
    assert_eq!(hex, hex::encode(&hash));
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_schema_mismatch_rejection() {
    let server = MockUnityServer::new()
        .with_schema_hash("different-hash");
    
    let result = IpcClient::connect(test_config(&server)).await;
    assert!(matches!(result, Err(IpcError::SchemaMismatch(_))));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_schema_validation_success() {
    let expected_hash = codec::schema_hash();
    let server = MockUnityServer::new()
        .with_schema_hash(hex::encode(&expected_hash));
    
    let client = IpcClient::connect(test_config(&server)).await.unwrap();
    assert!(client.health(Duration::from_secs(1)).await.is_ok());
}
```

## 期待される変更ファイル

- `server/build.rs` (schema hash generation)
- `server/src/ipc/codec.rs` (schema hash access)
- `bridge/Assets/Editor/SchemaHashGenerator.cs` (新規)
- `bridge/Packages/com.example.mcp-bridge/Runtime/Generated/SchemaHash.cs` (生成)
- `scripts/update-schema-hash.sh` (新規)
- `.github/workflows/schema-validation.yml` (新規)

## Definition of Done

- [ ] Build時にdeterministicなschema hash生成
- [ ] RustとUnityで同一protocから同一hashを生成
- [ ] Handshake時のschema hash交換と検証
- [ ] Schema mismatch時の適切なerror handling
- [ ] CI/CDでのschema hash consistency validation
- [ ] Schema hash関連のunit/integration tests全てpass
- [ ] Proto変更時のhash更新workflow確立

## トラブルシューティング

よくある問題：
- protoc version差異によるFileDescriptorSet不一致
- include pathや順序による差異
- Unity build processとRust build processの同期
- Binary reproducibility issues

## セキュリティ考慮事項

- Schema hash自体は機密情報ではないが、詳細なerror messageでinternal structureを漏らさない
- Hash collision攻撃への対策（SHA-256使用）
- MVPでは schema_hash 不一致は FAILED_PRECONDITION で必ず拒否し、接続をクローズする

## Performance考慮事項

- Schema hash計算はbuild timeのみ（runtime costなし）
- Hash比較は高速（32 byte comparison）
- Descriptor set generationの最適化