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

`server/build.rs`を更新してipc_control.protoを含める：

```rust
let files = [
    "mcp/unity/v1/common.proto",
    "mcp/unity/v1/editor_control.proto", 
    "mcp/unity/v1/assets.proto",
    "mcp/unity/v1/build.proto",
    "mcp/unity/v1/operations.proto",
    "mcp/unity/v1/events.proto",
    "mcp/unity/v1/ipc.proto",
    "mcp/unity/v1/ipc_control.proto",  // 新規追加
]
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

## 注意事項

- Breaking changeとなるため、既存のhandshake実装は一時的に動作しなくなる可能性がある
- Phase 1完了後、Phase 2で実際のhandshake logicを実装するまでは統合テストが失敗する可能性がある
- Proto schemaの変更は両側（RustとUnity）で同期が必要