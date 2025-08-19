# Step 1 IPC Implementation - Current Status

## Overview
Direct IPC migration step 1の実装進捗状況と残りの作業をドキュメント化。

## Completed Work ✅

### 1. Protocol Buffers Definition
- ✅ `proto/mcp/unity/v1/ipc.proto` を新規作成
- ✅ IPCエンベロープメッセージ定義完了
  - `IpcEnvelope` (correlation_id + oneof kind)
  - `IpcRequest`, `IpcResponse`, `IpcEvent`
  - `IpcHello`, `IpcWelcome`
- ✅ `server/build.rs` に `ipc.proto` を追加
- ✅ Protobuf コード生成成功 (`src/generated/mcp.unity.v1.rs`)

### 2. Dependencies Management
- ✅ Cargo.toml に必要な依存関係追加済み
  - `thiserror`, `rand`, `cfg-if`
  - `tokio` に `net` 機能追加

### 3. gRPC Code Removal
- ✅ `server/src/grpc/` ディレクトリ削除
- ✅ gRPCフィーチャーフラグ削除 (`transport-grpc`, `server-stubs`)
- ✅ lib.rs からgRPCモジュール削除、IPCモジュール追加

### 4. IPC Module Structure
- ✅ `server/src/ipc/` ディレクトリ構造作成
- ✅ `mod.rs` - モジュール宣言
- ✅ `path.rs` - エンドポイント解決とOS固有デフォルト実装
- ✅ `framing.rs` - LengthDelimitedCodec ラッパー実装
- ✅ `codec.rs` - Protobuf エンコード/デコード実装（テスト付き）
- ✅ `client.rs` - IpcClient基本構造実装

### 5. Service Integration
- ✅ `McpService` をIpcClientベースに移行
- ✅ `health.rs` ツールをIPC使用に変更

## Current Issues ❌

### 1. Compilation Errors in client.rs
以下のコンパイルエラーが残存：

```
error[E0599]: the method `send` exists for struct `SplitStream<...>`, but its trait bounds were not satisfied
error[E0599]: the method `next` exists for struct `SplitSink<...>`, but its trait bounds were not satisfied
```

**Root Cause:** `futures::stream::split()` の戻り値の使い方が間違っている
- `split()` は `(SplitStream, SplitSink)` を返す
- `SplitStream` は読み取り用（`StreamExt::next()`）
- `SplitSink` は書き込み用（`SinkExt::send()`）
- 現在のコードでは逆に使用している

### 2. Warning: transport-grpc Feature
`src/config.rs:117` で削除されたはずの `transport-grpc` フィーチャーが参照されている

## Next Steps (Priority Order)

### Immediate Fixes Required
1. **Fix split() usage in client.rs:**
   ```rust
   let (reader, writer) = framed.split();
   // reader は SplitStream (読み取り用) - reader.next()
   // writer は SplitSink (書き込み用) - writer.send()
   ```

2. **Remove transport-grpc references:**
   - `src/config.rs` の該当行を修正または削除

### Testing & Validation
3. **Unit Tests:**
   - codec.rs テストは既存 ✅
   - path.rs テストは既存 ✅
   - client.rs の基本機能テスト追加

4. **Integration Testing:**
   - 簡単なTCP echo serverでのhandshake テスト
   - HealthRequest/Response roundtrip テスト

### Documentation
5. **Update CLAUDE.md:**
   - gRPC関連の記述をIPC関連に更新
   - 新しいコマンド例の追加

## Current Architecture

```
server/
  src/
    ipc/
      mod.rs          ✅ モジュール宣言
      path.rs         ✅ OS固有エンドポイント処理
      framing.rs      ✅ 長さ区切りフレーミング
      codec.rs        ✅ Protobuf エンコード/デコード
      client.rs       ❌ コンパイルエラー（要修正）
    mcp/
      service.rs      ✅ IpcClient統合済み
      tools/
        health.rs     ✅ IPC使用に変更済み
    generated/
      mcp.unity.v1.rs ✅ IPC proto生成済み
```

## Key Implementation Details

### IpcClient Design
- 非同期接続とハンドシェイク
- コリレーションID管理による request/response マッピング
- バックグラウンドread/writeタスク分離
- イベントブロードキャスト対応
- OS固有エンドポイント対応（Unix domain socket, Named pipe, TCP fallback）

### Configuration
- Environment variables:
  - `MCP_IPC_ENDPOINT` (endpoint string)
  - `MCP_IPC_TOKEN` (auth token)
  - `MCP_IPC_CONNECT_TIMEOUT_MS` (default: 2000)
  - `MCP_IPC_CALL_TIMEOUT_MS` (default: 4000)

## Files Modified
- `proto/mcp/unity/v1/ipc.proto` (新規)
- `server/build.rs` (ipc.proto追加)
- `server/Cargo.toml` (依存関係追加、フィーチャー整理)
- `server/src/lib.rs` (grpc削除、ipc追加)
- `server/src/ipc/*` (新規モジュール群)
- `server/src/mcp/service.rs` (IpcClient統合)
- `server/src/mcp/tools/health.rs` (IPC使用)

## Definition of Done for Step 1
- [ ] コンパイルエラー解消
- [ ] 基本的なunit testが通る
- [ ] `IpcClient::connect()` が完了できる
- [ ] `IpcClient::health()` がHealthResponseを返す
- [ ] イベントチャネルが動作する

## Next Step (Step 2) Preview
- Unity側 `EditorIpcServer` 実装
- IpcClient にreconnect loop追加
- 他のAPI（Assets, Build, Operations）の具象化