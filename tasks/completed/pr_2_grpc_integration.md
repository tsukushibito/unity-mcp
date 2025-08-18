# PR-2: gRPC Integration — 実際のUnity Bridge連携

> **目標**: PR-1のスタブ実装を実際のgRPC呼び出しに置き換え、ChannelManagerを統合する

---

## DoD (Definition of Done)

- [ ] `call_tool("unity.health")` が実際のgRPC `EditorControl.Health` を呼び出す
- [ ] レスポンスが構造化JSON `{ "ready": true, "version": "X.Y.Z" }` で返される
- [ ] タイムアウト処理が適切に動作する（設定可能な期限）
- [ ] gRPCエラーが適切なMCP ToolErrorにマッピングされる
- [ ] ダミーサーバーに対する手動テストが成功する
- [ ] 設定統合が完了している（既存GrpcConfigとの整合）

---

## 前提条件

- PR-1が完了し、MCPスキャフォールディングが動作している
- 既存の `ChannelManager` と `GrpcConfig` の動作確認済み

---

## 実装手順

### Step 1: 設定統合の完成

既存の `GrpcConfig` とMCP用設定を統合：

```rust
// server/src/config.rs を更新
use std::env;
use crate::grpc::config::GrpcConfig;

#[derive(Clone, Debug)]
pub struct UnifiedConfig {
    pub grpc: GrpcConfig,
    pub health_timeout_ms: u64,
}

impl UnifiedConfig {
    pub fn load() -> Self {
        // 既存のGrpcConfigを活用
        let grpc = GrpcConfig::from_env();
        
        // MCP固有の設定を追加
        let health_timeout_ms = env::var("UNITY_HEALTH_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(2000);
            
        Self { grpc, health_timeout_ms }
    }
    
    pub fn health_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.health_timeout_ms)
    }
}
```

### Step 2: McpService にChannelManager統合

```rust
// server/src/mcp/service.rs を更新
use rmcp::{prelude::*, server::Server};
use crate::grpc::channel::ChannelManager;
use crate::config::UnifiedConfig;

#[derive(Clone)]
pub struct McpService {
    cm: ChannelManager,
    config: UnifiedConfig,
}

impl McpService {
    pub async fn new(config: UnifiedConfig) -> anyhow::Result<Self> {
        let cm = ChannelManager::connect(&config.grpc).await?;
        Ok(Self { cm, config })
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let transport = rmcp::transport::stdio();
        Server::new(self).serve(transport).await?;
        Ok(())
    }
    
    // 内部アクセサー
    pub(crate) fn channel_manager(&self) -> &ChannelManager { &self.cm }
    pub(crate) fn config(&self) -> &UnifiedConfig { &self.config }
}

#[rmcp::tool_router]
impl McpService {}
```

### Step 3: 実際のgRPC呼び出し実装

```rust
// server/src/mcp/tools/health.rs を更新
use rmcp::{prelude::*, types::Json};
use tonic::Code;
use crate::mcp_types::HealthOut;
use crate::mcp::service::McpService;
use crate::generated::mcp::unity::v1::HealthRequest;

impl McpService {
    #[rmcp::tool(name = "unity.health", description = "Unity Bridge health check")]
    pub async fn tool_unity_health(&self) -> Result<Json<HealthOut>, ToolError> {
        // gRPCクライアント取得
        let mut client = self.channel_manager().editor_control_client();
        
        // リクエスト作成
        let request = HealthRequest {};
        
        // タイムアウト設定
        let timeout = self.config().health_timeout();
        
        // gRPC呼び出し（タイムアウト付き）
        let response = tokio::time::timeout(timeout, client.health(request))
            .await
            .map_err(|_| ToolError::from_message("Unity Bridge deadline exceeded"))?
            .map_err(to_tool_error)?;
            
        let health_response = response.into_inner();
        
        Ok(Json(HealthOut {
            ready: health_response.ready,
            version: health_response.version,
        }))
    }
}

// gRPC Status -> MCP ToolError マッピング
fn to_tool_error(status: tonic::Status) -> ToolError {
    match status.code() {
        Code::Unavailable => 
            ToolError::from_message("Unity Bridge unavailable"),
        Code::DeadlineExceeded => 
            ToolError::from_message("Unity Bridge deadline exceeded"),
        Code::Unauthenticated => 
            ToolError::from_message("Unauthenticated to Unity Bridge"),
        Code::PermissionDenied => 
            ToolError::from_message("Permission denied by Unity Bridge"),
        _ => 
            ToolError::from_message(format!("Unity Bridge error: {}", status.message())),
    }
}
```

### Step 4: main.rs の更新

```rust
// server/src/main.rs を更新
mod config;
mod observability;
mod mcp;
mod mcp_types;
mod grpc;  // 既存のgrpcモジュールを追加

use crate::config::UnifiedConfig;
use crate::mcp::service::McpService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    observability::init_tracing();
    
    let config = UnifiedConfig::load();
    tracing::info!(
        grpc_addr = %config.grpc.addr,
        timeout_ms = config.health_timeout_ms,
        "Starting MCP server with Unity Bridge integration"
    );
    
    let svc = McpService::new(config).await?;
    svc.serve_stdio().await
}
```

---

## エラーマッピング詳細

| gRPC `tonic::Code`    | MCP ToolError message             | 想定シナリオ |
|-----------------------|-----------------------------------|-------------|
| `Unavailable`         | `Unity Bridge unavailable`        | Bridge未起動 |
| `DeadlineExceeded`    | `Unity Bridge deadline exceeded`  | 応答遅延 |
| `Unauthenticated`     | `Unauthenticated to Unity Bridge` | 認証失敗 |
| `PermissionDenied`    | `Permission denied by Unity Bridge` | 権限不足 |
| (その他)              | `Unity Bridge error: {message}`   | その他のエラー |

---

## 手動テスト

### 1. ダミーサーバーテスト

既存の `tests/smoke.rs` のダミーサーバーを利用：

```bash
# Terminal 1: ダミーサーバー起動
cd server
cargo test --features server-stubs channel_manager_roundtrip_health -- --nocapture

# Terminal 2: MCP server手動テスト  
cd server
MCP_BRIDGE_ADDR="http://127.0.0.1:50051" cargo run
```

### 2. MCP呼び出しテスト

```bash
# tools/list
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}' | cargo run

# tools/call - unity.health
echo '{"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "unity.health"}}' | cargo run
```

### 3. エラーケーステスト

```bash
# Bridge未起動での実行
MCP_BRIDGE_ADDR="http://127.0.0.1:9999" cargo run
# -> "Unity Bridge unavailable" エラーを確認

# タイムアウトテスト
UNITY_HEALTH_TIMEOUT_MS=100 cargo run  
# -> 極短タイムアウトでの動作確認
```

---

## 統合単体テスト

```rust
// server/src/mcp/tools/health.rs に追加
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::UnifiedConfig;
    
    #[tokio::test]
    async fn test_unity_health_with_mock_config() {
        // モック設定でテスト
        let config = UnifiedConfig {
            grpc: crate::grpc::config::GrpcConfig::from_map([
                ("MCP_BRIDGE_ADDR".to_string(), "http://127.0.0.1:50051".to_string()),
            ]),
            health_timeout_ms: 1000,
        };
        
        // 実際のサーバーが必要なので、結合テストで実装
        // ここでは設定ロードのテストに留める
        assert_eq!(config.health_timeout_ms, 1000);
    }
}
```

---

## デバッグとトラブルシューティング

### デバッグ実行
```bash
RUST_LOG=debug MCP_BRIDGE_ADDR="http://127.0.0.1:50051" cargo run
```

### よくある問題

1. **gRPCコネクション失敗**
   - Bridge サーバーの起動確認
   - ポート番号の確認
   - ファイアウォール設定

2. **タイムアウトエラー**
   - `UNITY_HEALTH_TIMEOUT_MS` の調整
   - ネットワーク遅延の確認

3. **認証エラー**
   - `MCP_BRIDGE_TOKEN` の設定確認
   - Bridge側の認証設定

---

## 環境変数設定例

```bash
# 基本設定
export MCP_BRIDGE_ADDR="http://127.0.0.1:50051"
export UNITY_HEALTH_TIMEOUT_MS=2000

# 認証が必要な場合
export MCP_BRIDGE_TOKEN="your-auth-token"

# デバッグログ
export RUST_LOG="debug"
```

---

## 次のPR（テスト・CI）への準備

- [ ] テスト用のダミーサーバー設定方法の文書化
- [ ] CI環境でのgRPCテスト戦略
- [ ] パフォーマンステスト要件の定義
- [ ] エラーケースの網羅的テスト計画

---

## 確認コマンド

```bash
# ビルド確認
cargo build --locked

# 基本テスト
cargo test

# gRPC統合テスト（ダミーサーバー必要）
cargo test --features server-stubs

# 手動MCP動作確認
echo '{"jsonrpc": "2.0", "id": 1, "method": "tools/list"}' | MCP_BRIDGE_ADDR="http://127.0.0.1:50051" cargo run
```