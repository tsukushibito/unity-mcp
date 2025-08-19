# Unity MCP Server - 開発ガイドライン (2025年8月更新)

## 基本開発方針
- **現在の段階**: gRPC統合完了、Unity Bridge実装段階
- **高速フィードバックループ**: 単一リポジトリで Rust + Unity 統合開発
- **型安全性重視**: プロトコルバッファによる通信の型安全性確保
- **段階的実装**: 動作確認しながら機能を段階的に追加

## 開発環境設定

### 必須作業ディレクトリ
**重要: Rustコマンドは必ず `server/` ディレクトリから実行すること**
```bash
cd /workspaces/unity-mcp/server  # Rust開発用
cd /workspaces/unity-mcp         # プロジェクト全体作業用
```

### 主要開発コマンド

**Rust Server開発 (server/内で実行):**
```bash
# ビルドとチェック
cargo build --locked
cargo check
cargo fmt --check  # フォーマット確認
cargo fmt           # フォーマット適用
cargo clippy --all-targets -- -D warnings

# テスト実行
cargo test                                    # 単体テストのみ
cargo test --features server-stubs          # 統合テスト含む
cargo test smoke --features server-stubs    # gRPCテスト
cargo test --features server-stubs -- --nocapture  # 詳細出力

# プロトコルバッファ変更後の必須手順
cargo clean          # 生成コード強制再生成
cargo build --features server-stubs

# サーバー実行
cargo run            # stdio transport
RUST_LOG=info cargo run  # ログレベル指定
```

**Unity Bridge開発:**
```bash
# C# gRPCコード生成 (bridge/Tools内)
./generate-csharp.sh  # プロトコルバッファからC#生成
./copy-grpc-libs.sh   # gRPCライブラリコピー

# Unity テスト (CI用)
Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
```

## コーディング規約

### Rust コード規約
**インポート順序:**
```rust
// 1. std
use std::collections::HashMap;

// 2. 外部クレート
use anyhow::Result;
use tokio::sync::oneshot;
use tonic::transport::Channel;

// 3. ローカルモジュール
use crate::grpc::ChannelManager;
use crate::mcp::tools::Health;
```

**命名規則:**
- 関数・変数: `snake_case` (例: `create_channel`)
- 型・構造体: `PascalCase` (例: `McpService`)  
- 定数: `SCREAMING_SNAKE_CASE` (例: `DEFAULT_TIMEOUT`)

**エラーハンドリング:**
- アプリケーションレベル: `anyhow::Result<T>`
- ドメインエラー: `thiserror` による構造化エラー
- **絶対禁止**: 本番コードでの `unwrap()` / `expect()` 使用
- リクエストハンドラでのパニック禁止

### C# コード規約
**命名規則:**
- 型・メソッド: `PascalCase` (例: `EditorControlService`)
- フィールド・ローカル変数: `camelCase` (例: `channelManager`)
- 定数: `UPPER_CASE` (例: `DEFAULT_PORT`)

**エラーハンドリング:**
```csharp
try
{
    // gRPC呼び出し
}
catch (RpcException ex)
{
    UnityEngine.Debug.LogError($"gRPC error: {ex.Status}");
}
```

## プロジェクト構造パターン

### モジュール構成 (Rust)
- `module_name.rs` を使用（`mod.rs` は避ける - Rust 2018+ 規約）
- 機能別モジュール分割
```
src/
├── main.rs           # エントリーポイント
├── mcp/              # MCP関連
│   ├── service.rs    # McpService実装
│   └── tools/        # MCPツール群
├── grpc/             # gRPC統合
│   ├── config.rs     # 設定管理
│   └── channel.rs    # 接続管理
└── generated/        # 自動生成コード
```

### テスト戦略
**単体テスト:**
- モジュール内に `#[cfg(test)]` で配置
- 決定論的テスト（ネットワーク依存回避）

**統合テスト:**
- `server/tests/` 配下
- `--features server-stubs` 必須
- gRPC接続テスト: `smoke.rs`

## 重要な開発注意点

### プロトコルバッファ関連
- **proto変更後**: 必ず `cargo clean` → `cargo build --features server-stubs`
- **生成コードの場所**: `server/src/generated/` （Git管理外）
- **CI要件**: protoc 3.21.12 必須

### フィーチャーフラグ
- **`server-stubs`**: 統合テスト実行時に必須
- **使用例**: `cargo test --features server-stubs`

### gRPC接続
- **認証**: トークンベース認証実装済み
- **設定**: 環境変数経由（`GrpcConfig`）
- **接続管理**: `ChannelManager` による接続プール

## CI/CD 注意点
- **マトリックステスト**: Ubuntu + macOS
- **protoc バージョン**: 3.21.12 固定
- **テスト実行**: `cargo test --features server-stubs`
- **Unity テスト**: EditMode テストのみ

## デバッグとトラブルシューティング

### よくある問題
1. **gRPC接続エラー**: `smoke.rs` テストで確認
2. **proto生成失敗**: `cargo clean` してから再ビルド
3. **テスト失敗**: `server-stubs` フィーチャー確認

### ログ設定
```bash
# デバッグログ有効化
RUST_LOG=debug cargo run

# 特定モジュールのみ
RUST_LOG=server::grpc=debug cargo run
```

## コミット規約
Conventional Commits形式、英語メッセージ:
```
feat: add gRPC health check integration
fix: resolve channel connection timeout issue
docs: update development setup instructions
```