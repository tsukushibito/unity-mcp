# PR-3: Testing & CI — 包括的テストとCI統合

> **目標**: 全てのエラーケースを網羅する統合テストを実装し、CI/CDパイプラインを完成させる

---

## DoD (Definition of Done)

- [ ] 成功・タイムアウト・利用不可のシナリオをカバーする統合テストが動作
- [ ] CI（`ci.yml`）でLinux環境でのビルド+テストが通る
- [ ] MCP server のエンドツーエンドテストが自動化されている
- [ ] テストにネットワーク不安定性がない（ローカルバインド使用）
- [ ] 既存のprotoc 3.21.12との互換性が確保されている
- [ ] カバレッジが適切なレベルに達している

---

## 前提条件

- PR-1, PR-2が完了し、実際のgRPC統合が動作している
- 既存の `tests/smoke.rs` の理解と活用

---

## テスト戦略

### 1. 統合テストレベル
- **ファイル**: `tests/health_mcp.rs`
- **スコープ**: MCP server ↔ ダミーgRPCサーバー間のエンドツーエンド
- **目的**: 実際のMCP プロトコル動作確認

### 2. 単体テストレベル  
- **スコープ**: 個別コンポーネント（設定、エラーマッピング等）
- **目的**: ロジックの正確性確認

### 3. CIテストレベル
- **環境**: Ubuntu Linux (GitHub Actions)
- **スコープ**: 完全なビルド・テストパイプライン

---

## 実装手順

### Step 1: 統合テスト基盤の実装

```rust
// tests/health_mcp.rs
#[cfg(feature = "server-stubs")]
use std::{net::SocketAddr, time::Duration};
#[cfg(feature = "server-stubs")]
use tokio::{net::TcpListener, sync::oneshot, time::{timeout, sleep}};
#[cfg(feature = "server-stubs")]
use tokio_stream::wrappers::TcpListenerStream;
#[cfg(feature = "server-stubs")]
use tonic::{Request, Response, Status, transport::Server};

#[cfg(feature = "server-stubs")]
use server::generated::mcp::unity::v1::{
    HealthRequest, HealthResponse,
    editor_control_server::{EditorControl, EditorControlServer},
};

#[cfg(feature = "server-stubs")]
use server::config::UnifiedConfig;
#[cfg(feature = "server-stubs")]
use server::mcp::service::McpService;

#[cfg(feature = "server-stubs")]
#[derive(Debug)]
struct TestEditorControlService {
    behavior: TestBehavior,
}

#[cfg(feature = "server-stubs")]
#[derive(Debug, Clone)]
enum TestBehavior {
    Success { ready: bool, version: String },
    Delay(Duration),
    Unavailable,
}

#[cfg(feature = "server-stubs")]
#[tonic::async_trait]
impl EditorControl for TestEditorControlService {
    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        match &self.behavior {
            TestBehavior::Success { ready, version } => {
                Ok(Response::new(HealthResponse {
                    ready: *ready,
                    version: version.clone(),
                    status: if *ready { "OK".to_string() } else { "NOT_READY".to_string() },
                }))
            }
            TestBehavior::Delay(d) => {
                sleep(*d).await;
                Ok(Response::new(HealthResponse {
                    ready: true,
                    version: "delayed".to_string(),
                    status: "OK".to_string(),
                }))
            }
            TestBehavior::Unavailable => {
                Err(Status::unavailable("Service temporarily unavailable"))
            }
        }
    }

    // 他のメソッドはスタブ実装
    async fn get_play_mode(
        &self,
        _req: Request<server::generated::mcp::unity::v1::Empty>,
    ) -> Result<Response<server::generated::mcp::unity::v1::GetPlayModeResponse>, Status> {
        Err(Status::unimplemented("Not implemented in test"))
    }

    async fn set_play_mode(
        &self,
        _req: Request<server::generated::mcp::unity::v1::SetPlayModeRequest>,
    ) -> Result<Response<server::generated::mcp::unity::v1::SetPlayModeResponse>, Status> {
        Err(Status::unimplemented("Not implemented in test"))
    }
}

#[cfg(feature = "server-stubs")]
async fn start_test_server(behavior: TestBehavior) -> anyhow::Result<(SocketAddr, oneshot::Sender<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);

    let (tx, rx) = oneshot::channel::<()>();
    let svc = TestEditorControlService { behavior };
    
    tokio::spawn(async move {
        let _ = Server::builder()
            .add_service(EditorControlServer::new(svc))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = rx.await;
            })
            .await;
    });

    // サーバー起動を少し待つ
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    Ok((addr, tx))
}
```

### Step 2: 成功ケースのテスト

```rust
#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_success() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: true,
        version: "test-1.0.0".to_string(),
    };
    
    let (addr, _shutdown) = start_test_server(behavior).await?;
    
    // 環境変数設定（テスト用）
    std::env::set_var("MCP_BRIDGE_ADDR", format!("http://{}", addr));
    std::env::set_var("UNITY_HEALTH_TIMEOUT_MS", "5000");
    
    let config = UnifiedConfig::load();
    let svc = McpService::new(config).await?;
    
    // 直接ツールメソッドを呼び出し
    let result = svc.tool_unity_health().await?;
    
    assert!(result.0.ready);
    assert_eq!(result.0.version, "test-1.0.0");
    
    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_not_ready() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: false,
        version: "test-not-ready".to_string(),
    };
    
    let (addr, _shutdown) = start_test_server(behavior).await?;
    std::env::set_var("MCP_BRIDGE_ADDR", format!("http://{}", addr));
    
    let config = UnifiedConfig::load();
    let svc = McpService::new(config).await?;
    
    let result = svc.tool_unity_health().await?;
    
    assert!(!result.0.ready); // Bridge が not ready を報告
    assert_eq!(result.0.version, "test-not-ready");
    
    Ok(())
}
```

### Step 3: エラーケースのテスト

```rust
#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_timeout() -> anyhow::Result<()> {
    let behavior = TestBehavior::Delay(Duration::from_millis(2000)); // 2秒遅延
    let (addr, _shutdown) = start_test_server(behavior).await?;
    
    std::env::set_var("MCP_BRIDGE_ADDR", format!("http://{}", addr));
    std::env::set_var("UNITY_HEALTH_TIMEOUT_MS", "500"); // 0.5秒タイムアウト
    
    let config = UnifiedConfig::load();
    let svc = McpService::new(config).await?;
    
    let result = svc.tool_unity_health().await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("deadline exceeded"));
    
    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_unavailable() -> anyhow::Result<()> {
    let behavior = TestBehavior::Unavailable;
    let (addr, _shutdown) = start_test_server(behavior).await?;
    
    std::env::set_var("MCP_BRIDGE_ADDR", format!("http://{}", addr));
    
    let config = UnifiedConfig::load();
    let svc = McpService::new(config).await?;
    
    let result = svc.tool_unity_health().await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("unavailable"));
    
    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_connection_refused() -> anyhow::Result<()> {
    // 接続先ポートを存在しないものに設定
    std::env::set_var("MCP_BRIDGE_ADDR", "http://127.0.0.1:9999");
    std::env::set_var("UNITY_HEALTH_TIMEOUT_MS", "1000");
    
    let config = UnifiedConfig::load();
    
    // ChannelManager の接続時点でエラーになることを確認
    let result = McpService::new(config).await;
    assert!(result.is_err());
    
    Ok(())
}
```

### Step 4: MCP プロトコルレベルのテスト

```rust
#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_mcp_tools_list() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: true,
        version: "test".to_string(),
    };
    
    let (addr, _shutdown) = start_test_server(behavior).await?;
    std::env::set_var("MCP_BRIDGE_ADDR", format!("http://{}", addr));
    
    let config = UnifiedConfig::load();
    let svc = McpService::new(config).await?;
    
    // ツールルーターからツールリストを取得
    let tools = svc.list_tools().await?;
    
    // unity.health ツールが含まれていることを確認
    let health_tool = tools.iter().find(|t| t.name == "unity.health");
    assert!(health_tool.is_some());
    
    let health_tool = health_tool.unwrap();
    assert_eq!(health_tool.description, "Unity Bridge health check");
    
    Ok(())
}
```

### Step 5: CI設定の更新

```yaml
# .github/workflows/ci.yml の該当部分を更新
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  rust-server:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install Protocol Buffers
      run: |
        if [[ "${{ runner.os }}" == "Linux" ]]; then
          sudo apt-get update
          sudo apt-get install -y protobuf-compiler=3.21.12-3
        elif [[ "${{ runner.os }}" == "macOS" ]]; then
          brew install protobuf@21
          echo "/opt/homebrew/opt/protobuf@21/bin" >> $GITHUB_PATH
        fi
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
    
    - name: Cache Cargo dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          server/target
        key: ${{ runner.os }}-cargo-${{ hashFiles('server/Cargo.lock') }}
    
    - name: Build server
      working-directory: server
      run: cargo build --locked --verbose
    
    - name: Run unit tests
      working-directory: server
      run: cargo test --locked --verbose
    
    - name: Run integration tests with server-stubs
      working-directory: server
      run: cargo test --locked --verbose --features server-stubs
    
    - name: Check code formatting
      working-directory: server
      run: cargo fmt --all -- --check
    
    - name: Run clippy
      working-directory: server
      run: cargo clippy --all-targets -- -D warnings
```

### Step 6: パフォーマンステスト

```rust
#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_health_calls() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: true,
        version: "concurrent-test".to_string(),
    };
    
    let (addr, _shutdown) = start_test_server(behavior).await?;
    std::env::set_var("MCP_BRIDGE_ADDR", format!("http://{}", addr));
    
    let config = UnifiedConfig::load();
    let svc = McpService::new(config).await?;
    
    // 10並行でhealth callを実行
    let tasks: Vec<_> = (0..10).map(|_| {
        let svc = svc.clone();
        tokio::spawn(async move {
            svc.tool_unity_health().await
        })
    }).collect();
    
    let results = futures::future::try_join_all(tasks).await?;
    
    // 全て成功することを確認
    for result in results {
        let health = result?;
        assert!(health.0.ready);
        assert_eq!(health.0.version, "concurrent-test");
    }
    
    Ok(())
}
```

---

## テスト実行コマンド

### ローカルテスト
```bash
cd server

# 単体テスト
cargo test

# 統合テスト（server-stubsフィーチャー必要）
cargo test --features server-stubs

# 特定のテスト
cargo test --features server-stubs test_unity_health_success

# デバッグ付きテスト
RUST_LOG=debug cargo test --features server-stubs -- --nocapture
```

### CI模擬実行
```bash
# フォーマットチェック
cargo fmt --all -- --check

# Clippy
cargo clippy --all-targets -- -D warnings

# 全テストスイート
cargo test --features server-stubs --locked --verbose
```

---

## テストカバレッジ

### 機能カバレッジ
- [x] 成功ケース（ready=true/false）
- [x] タイムアウトエラー
- [x] 接続エラー（unavailable）
- [x] 接続拒否エラー
- [x] MCPプロトコルレベル
- [x] 並行性テスト

### エラーマッピングカバレッジ
- [x] `Code::Unavailable`
- [x] `Code::DeadlineExceeded`
- [x] `Code::Unauthenticated` (将来対応)
- [x] その他の gRPC エラー

---

## トラブルシューティング

### CI環境での問題

1. **protoc バージョン不整合**
```bash
protoc --version  # 3.21.12 であることを確認
```

2. **ポート競合**
```rust
// テストで必ず `bind(("127.0.0.1", 0))` を使用してランダムポート
let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
```

3. **タイムアウトテストの不安定性**
```rust
// 十分にマージンを取った設定
std::env::set_var("UNITY_HEALTH_TIMEOUT_MS", "100"); // 短いタイムアウト
let behavior = TestBehavior::Delay(Duration::from_millis(500)); // 長い遅延
```

### ローカル開発での問題

1. **Feature flag忘れ**
```bash
cargo test --features server-stubs  # 必須
```

2. **環境変数汚染**
```rust
// テスト毎に環境変数をクリーンアップ
#[tokio::test]
async fn test_something() {
    std::env::remove_var("MCP_BRIDGE_ADDR");
    std::env::remove_var("UNITY_HEALTH_TIMEOUT_MS");
    // テストロジック
}
```

---

## 完成確認チェックリスト

- [ ] `cargo test --features server-stubs` が全て通る
- [ ] `cargo build --locked` が成功する
- [ ] `cargo fmt --check` が通る  
- [ ] `cargo clippy` が警告なしで通る
- [ ] CI/CD パイプラインが Green
- [ ] 手動でのMCPプロトコル動作確認完了
- [ ] 全エラーケースの動作確認完了

---

## 次のフェーズへの準備

- [ ] パフォーマンスベンチマーク結果
- [ ] メモリ使用量プロファイル
- [ ] 実際のUnity Bridgeとの統合テスト計画
- [ ] ドキュメント更新（README, 運用手順）