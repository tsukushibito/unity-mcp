# T01 Phase1: Proto定義とCode Generation

**Status:** Ready for implementation  
**Priority:** High (blocking for all other phases)  
**Estimated effort:** 2-3 hours

## 目標と成果物

T01 IPC Handshake仕様に必要な新しいProtocol Buffer定義を作成し、RustとC#でのcode generationを設定する。

### 成果物
- [ ] `proto/mcp/unity/v1/ipc_control.proto` の作成
- [ ] Rust build scriptの更新
- [ ] 生成されたRustコードの確認
- [ ] Unity C# code generationの設定

## 前提条件

- T01仕様書 (`t_01_ipc_handshake_spec_draft.md`) の理解
- 既存のproto構造 (`proto/mcp/unity/v1/`) の把握
- `server/build.rs` の動作理解

## 実装手順

### Step 1: ipc_control.proto作成

T01仕様のSection 2に基づき、`proto/mcp/unity/v1/ipc_control.proto`を作成：

```proto
syntax = "proto3";
package mcp.unity.v1;

message IpcControl {
  oneof kind {
    IpcHello hello = 1;
    IpcWelcome welcome = 2;
    IpcReject reject = 3;
  }
}

message IpcHello {
  // Security
  string token = 1;                    // required

  // Protocol compatibility
  string ipc_version = 2;              // e.g. "1.0"; major must match
  repeated string features = 3;        // requested feature flags (see §6)

  // Schema & environment
  bytes schema_hash = 4;               // SHA-256 of FileDescriptorSet (see §4)
  string project_root = 5;             // absolute path; normalized
  string client_name = 6;              // e.g. "unity-mcp-rs"
  string client_version = 7;           // semver of Rust server

  map<string,string> meta = 8;         // optional free-form (OS, arch, etc.)
}

message IpcWelcome {
  // Echoed/negotiated
  string ipc_version = 1;              // server-supported for this session
  repeated string accepted_features = 2;
  bytes schema_hash = 3;               // server view of schema

  // Server info
  string server_name = 4;              // e.g. "unity-editor-bridge"
  string server_version = 5;           // plugin/package version
  string editor_version = 6;           // e.g. "Unity 6000.0.x"
  string session_id = 7;               // UUID for logs and tracing

  map<string,string> meta = 8;         // optional (platform, license, etc.)
}

message IpcReject {
  enum Code {
    UNAUTHENTICATED = 0;
    FAILED_PRECONDITION = 1;  // schema mismatch, editor state invalid
    PERMISSION_DENIED = 2;    // token valid but insufficient rights
    OUT_OF_RANGE = 3;         // unsupported major version
    INTERNAL = 4;             // unexpected error
    UNAVAILABLE = 5;          // editor busy starting up, try later
  }
  Code code = 1;
  string message = 2;         // single-sentence reason
}
```

### Step 2: Rust build script更新

`server/build.rs`を更新してipc_control.protoを含める。Schema hash生成も同時に実装：

```rust
use std::{env, fs, path::PathBuf};
use prost_build::Config;
use sha2::{Sha256, Digest};

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
        "mcp/unity/v1/ipc_control.proto",  // 新規追加
    ].into_iter().map(|p| proto_root.join(p)).collect::<Vec<_>>();
    files.sort(); // Deterministic ordering

    // 2) Generate Rust code + FileDescriptorSet
    let descriptor_path = out_dir.join("schema.pb");
    let mut cfg = Config::new();
    cfg.out_dir(&out_dir);
    cfg.file_descriptor_set_path(&descriptor_path);
    cfg.protoc_arg("--include_imports");
    cfg.protoc_arg("--include_source_info=false");
    cfg.compile_protos(&files, &[proto_root.clone()])?;

    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
    }

    // 3) Generate schema hash as byte array (not hex string)
    let bytes = fs::read(&descriptor_path)?;
    let hash = Sha256::digest(&bytes);
    let hash_array: [u8; 32] = hash.into();
    
    fs::write(out_dir.join("schema_hash.rs"),
        format!("pub const SCHEMA_HASH: [u8; 32] = {:?};\n", hash_array)
    )?;
    
    Ok(())
}
```

### Step 3: 既存ipc.protoの調整

現在の`ipc.proto`から`IpcHello`と`IpcWelcome`を削除し、新しい`ipc_control.proto`と統合するための調整を行う。

### Step 4: Rust code generation確認

```bash
cd server
cargo clean
cargo build
```

生成されたコードが`server/src/generated/`に正しく配置されることを確認。

### Step 5: Unity C# code generation設定

Unity側でProtocol Bufferコードを生成するための設定を確認・更新。

## テスト要件

### Unit Tests
- [ ] `ipc_control.proto`のコンパイル成功
- [ ] 生成されたRust structsのbasic操作（作成、serialize、deserialize）
- [ ] enum値の正しいmapping

### Integration Tests
- [ ] 新しいmessage typesを使ったcodec roundtrip test
- [ ] 既存のIPC mechanismとの互換性確認

## 期待される変更ファイル

- `proto/mcp/unity/v1/ipc_control.proto` (新規)
- `server/build.rs` (更新)
- `server/src/generated/` (自動生成)
- `bridge/Packages/com.example.mcp-bridge/Editor/Generated/` (自動生成予定)

## Definition of Done

- [ ] T01仕様のSection 2に完全準拠したproto定義
- [ ] Rustでのコンパイルとcode generation成功
- [ ] 基本的なunit testが全て通る
- [ ] 既存のbuild processが壊れていない
- [ ] 生成されたコードがlintエラーなく正常

## 次のフェーズへの引き継ぎ

Phase 2で必要となる要素：
- 新しい`IpcControl`メッセージの使用例
- handshakeフローでの`IpcHello`/`IpcWelcome`/`IpcReject`の使い分け
- 既存の`client.rs`での統合ポイント

## Schema Hash生成の重要事項

**決定性の確保：**
- Proto filesは必ずソート済みで処理
- `protoc`バージョンは3.21.12以上で固定（CI/local共通）
- `--include_imports=true`、`--include_source_info=false`で統一
- 生成されるschema hashは32バイトの生バイト配列

**データ型統一：**
- Proto定義：`bytes schema_hash`（32バイト生データ）
- Rust：`[u8; 32]`として保持、送信時は`.to_vec()`
- Unity：`byte[]`として保持、ログ表示時のみhex変換
- 比較は常に生バイト同士で実行

## 注意事項

- Breaking changeとなるため、既存のhandshake実装は一時的に動作しなくなる可能性がある
- Phase 1完了後、Phase 2で実際のhandshake logicを実装するまでは統合テストが失敗する可能性がある
- Schema hash生成の決定性確保が必須（OS/CI差分によるhash揺れ防止）