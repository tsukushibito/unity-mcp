# PR-1: MCP Scaffolding — 基本構造の実装

> **目標**: rmcp MCP serverの基本構造を実装し、`unity.health`ツールをスタブレベルで動作させる

---

## DoD (Definition of Done)

- [ ] `server` が **rmcp MCP server** over **stdio** としてビルド・実行可能
- [ ] MCP `list_tools` で `unity.health` が表示される
- [ ] `call_tool("unity.health")` がスタブ実装で呼び出し可能（gRPC統合は次のPRで）
- [ ] **mod.rs を使用しない** モジュール構造が正しく実装されている
- [ ] ローカルビルドが成功（`cargo build --locked`）
- [ ] 基本的な単体テストが通る

---

## ファイル構造（最終形）

```
server/
├─ Cargo.toml                 # 依存関係確認済み
├─ build.rs                   # 既存（gRPC client codegen）
└─ src/
   ├─ main.rs                 # MCP server起動への変更
   ├─ config.rs               # 統合設定管理
   ├─ observability.rs        # tracing初期化のリファクタリング
   ├─ mcp.rs                  # MCPモジュールルート
   ├─ mcp/
   │  ├─ service.rs           # McpService + serve_stdio()
   │  ├─ tools.rs             # toolsモジュール管理
   │  └─ tools/
   │     └─ health.rs         # unity.health スタブ実装
   └─ mcp_types.rs            # DTOs for tool I/O
```

---

## 実装手順

### Step 1: 依存関係確認

既存の `Cargo.toml` を確認：
```bash
cd server
grep -E "(rmcp|serde|tracing)" Cargo.toml
```

必要に応じて追加：
```bash
cargo add schemars --dev  # JsonSchemaが必要な場合のみ
```

### Step 2: `observability.rs` の作成

既存の `main.rs` から `init_tracing()` を抽出してリファクタリング：

```rust
// server/src/observability.rs
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_target(false).init();
}
```

### Step 3: `config.rs` の統合設定

既存の `GrpcConfig` を拡張してMCP用設定も含める：

```rust
// server/src/config.rs
use std::env;

#[derive(Clone, Debug)]
pub struct BridgeConfig {
    pub host: String,   // 既存のGrpcConfigから
    pub port: u16,      // 既存のGrpcConfigから
    pub health_timeout_ms: u64, // 新規追加
}

impl BridgeConfig {
    pub fn load() -> Self {
        let host = env::var("MCP_BRIDGE_HOST")
            .or_else(|_| env::var("UNITY_BRIDGE_HOST"))
            .unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("MCP_BRIDGE_PORT")
            .or_else(|_| env::var("UNITY_BRIDGE_PORT"))
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(50051);
        let health_timeout_ms = env::var("UNITY_HEALTH_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(2000);
        Self { host, port, health_timeout_ms }
    }
}
```

### Step 4: `mcp_types.rs` のDTO定義

```rust
// server/src/mcp_types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthOut {
    pub ready: bool,
    pub version: String,
}
```

### Step 5: `mcp.rs` モジュールルート

```rust
// server/src/mcp.rs
pub mod service;
pub mod tools;
```

### Step 6: `mcp/tools.rs` ツールモジュール管理

```rust
// server/src/mcp/tools.rs
pub mod health;
```

### Step 7: `mcp/service.rs` の基本実装

```rust
// server/src/mcp/service.rs
use rmcp::{prelude::*, server::Server};

#[derive(Clone)]
pub struct McpService {
    // 一旦シンプルに
}

impl McpService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let transport = rmcp::transport::stdio();
        Server::new(self).serve(transport).await?;
        Ok(())
    }
}

#[rmcp::tool_router]
impl McpService {}
```

### Step 8: `mcp/tools/health.rs` スタブ実装

```rust
// server/src/mcp/tools/health.rs
use rmcp::{prelude::*, types::Json};
use crate::mcp_types::HealthOut;
use crate::mcp::service::McpService;

impl McpService {
    #[rmcp::tool(name = "unity.health", description = "Unity Bridge health check")]
    pub async fn tool_unity_health(&self) -> Result<Json<HealthOut>, ToolError> {
        // スタブ実装：固定値を返す
        Ok(Json(HealthOut {
            ready: true,
            version: "stub-0.1.0".to_string(),
        }))
    }
}
```

### Step 9: `main.rs` の変更

```rust
// server/src/main.rs
mod config;
mod observability;
mod mcp;
mod mcp_types;

use crate::mcp::service::McpService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    observability::init_tracing();
    
    let svc = McpService::new();
    svc.serve_stdio().await
}
```

---

## テスト

### 基本的な単体テスト

```rust
// server/src/mcp/tools/health.rs に追加
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unity_health_stub() {
        let svc = McpService::new();
        let result = svc.tool_unity_health().await;
        assert!(result.is_ok());
        let health = result.unwrap();
        assert!(health.0.ready);
        assert_eq!(health.0.version, "stub-0.1.0");
    }
}
```

---

## 確認コマンド

### ビルド確認
```bash
cd server
cargo build --locked
```

### 基本テスト
```bash
cd server
cargo test
```

### MCP server動作確認（手動）
```bash
cd server
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}' | cargo run
```

---

## トラブルシューティング

### よくある問題
1. **rmcp関連のコンパイルエラー**: feature flagの確認
2. **モジュール解決エラー**: `mod.rs`を使っていないか確認
3. **依存関係エラー**: `Cargo.lock`削除後再ビルド

### デバッグ方法
```bash
RUST_LOG=debug cargo run
```

---

## 次のPRへの準備

- [ ] ChannelManagerの実際のAPIを確認
- [ ] gRPCクライアント統合ポイントの特定
- [ ] 環境変数名の統一方針決定
- [ ] エラーハンドリング戦略の詳細化

---

## 注意事項

- **mod.rsは絶対に使用しない**
- この段階では実際のgRPC呼び出しは実装しない
- 単純なスタブ実装で動作確認を優先
- 既存のコードへの影響を最小化