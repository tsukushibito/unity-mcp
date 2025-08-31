# Unity MCP Server Streamable HTTP 移行作業計画

## 概要
現在stdio対応のUnity MCP ServerをStreamable HTTP対応に移行する作業計画です。

## 現状分析

### 現在の構成
- rmcp: 0.5.0（transport-io機能使用）
- メイン実装: McpService（ServerHandler実装済み）
- トランスポート: stdio（`serve_stdio()`メソッド）
- ツールルーター: `#[tool_router]`マクロと`#[tool]`マクロを使用済み
- Unity Bridge: IPC経由でUnityと連携済み

### 移行対象
- stdio → Streamable HTTP（Axum + Hyper）
- rmcp 0.5.0 → 0.6.x

## 作業項目

### フェーズ1: 依存関係とプロジェクト設定

#### 1.1 Cargo.toml更新
**ファイル**: `server/Cargo.toml`

**変更内容**:
```toml
# rmcpバージョンアップと機能追加
rmcp = { version = "0.6", features = [
  "server",
  "transport-streamable-http-server",
  "transport-streamable-http-server-session"
] }

# HTTP関連の新規依存関係
axum = "0.8"
hyper = { version = "1" }

# 既存の依存関係（必要に応じて）
# anyhow, tokio, tracing, serde, schemarsなどは維持
```

**タスク**:
- [ ] rmcpを0.6.xにアップデート
- [ ] transport-streamable-http-server機能を追加
- [ ] transport-streamable-http-server-session機能を追加
- [ ] axum 0.8を追加
- [ ] hyper 1.0を追加
- [ ] 既存機能との互換性確認

#### 1.2 既存コードの互換性チェック
**タスク**:
- [ ] `cargo build`でビルドエラー確認
- [ ] APIの変更点を調査・対応
- [ ] 廃止された機能の置き換え

### フェーズ2: McpService構造体の修正

#### 2.1 tool_routerフィールドの修正
**ファイル**: `server/src/mcp/service.rs`

**現在の問題**: `#[tool_router]`マクロでは、構造体に`tool_router: ToolRouter<Self>`フィールドが必要

**変更内容**:
```rust
#[derive(Clone)]
pub struct McpService {
    tool_router: ToolRouter<Self>, // publicに変更が必要かもしれない
    // 他のフィールドは維持
    ipc: Arc<RwLock<Option<IpcClient>>>,
    bridge_state: Arc<RwLock<BridgeState>>,
    operations: Arc<Mutex<HashMap<String, OperationState>>>,
    notification_sender: Arc<Mutex<Option<NotificationSender>>>,
    pub sent_finished_notifications: Arc<Mutex<HashSet<String>>>,
}

#[tool_router]
impl McpService {
    pub async fn new() -> anyhow::Result<Self> {
        // ...
        Ok(Self {
            tool_router: Self::tool_router(), // マクロが生成するメソッド
            // 他のフィールド初期化
        })
    }
    
    // 既存のツールメソッドはそのまま維持
    // #[tool]アトリビュートも維持
}
```

**タスク**:
- [ ] tool_routerフィールドの適切な初期化
- [ ] 既存のツールメソッドの互換性確認
- [ ] `make_tool_router()`から`Self::tool_router()`への移行

### フェーズ3: Streamable HTTPサーバーの実装

#### 3.1 新しいserve_httpメソッドの実装
**ファイル**: `server/src/mcp/service.rs`

**変更内容**:
```rust
impl McpService {
    pub async fn serve_http(self, bind_addr: &str) -> anyhow::Result<()> {
        use axum::Router;
        use rmcp::transport::streamable_http_server::tower::{
            StreamableHttpServerConfig, StreamableHttpService,
        };
        use std::time::Duration;

        // Build the Streamable HTTP Tower service
        let svc = StreamableHttpService::new(
            || Ok(McpService::new().await.unwrap()), // ファクトリ関数
            Default::default(),                       // セッションマネージャー
            StreamableHttpServerConfig {
                sse_keep_alive: Some(Duration::from_secs(15)),
                stateful_mode: true,
            },
        );

        // Mount at /mcp
        let app = Router::new().route_service("/mcp", svc);

        // Run HTTP server
        let listener = tokio::net::TcpListener::bind(bind_addr).await?;
        tracing::info!("Streamable HTTP server listening on http://{}/mcp", bind_addr);
        
        axum::serve(listener, app).await?;
        Ok(())
    }

    // 既存のserve_stdioメソッドは互換性のため維持
    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        // 既存実装を維持
    }
}
```

**タスク**:
- [ ] serve_httpメソッドの実装
- [ ] StreamableHttpServiceの設定
- [ ] セッション管理の設定
- [ ] SSEキープアライブの設定

#### 3.2 main.rsの更新
**ファイル**: `server/src/main.rs`

**変更内容**:
```rust
use server::{mcp::service::McpService, observability};
use std::env;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    observability::init_tracing();

    let svc = McpService::new().await?;
    
    // 環境変数で動作モードを切り替え
    match env::var("MCP_TRANSPORT").as_deref() {
        Ok("http") | Ok("streamable-http") => {
            let bind_addr = env::var("MCP_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
            svc.serve_http(&bind_addr).await
        }
        _ => {
            // デフォルトはstdio（既存動作を維持）
            svc.serve_stdio().await
        }
    }
}
```

**タスク**:
- [ ] 環境変数による動作モード切り替え
- [ ] デフォルト動作の維持（stdio）
- [ ] 設定パラメータの外部化

### フェーズ4: テストと検証

#### 4.1 基本動作テスト
**タスク**:
- [ ] `cargo build`の成功確認
- [ ] stdio モードでの既存動作確認
- [ ] Unity Bridge接続テスト
- [ ] 既存ツールの動作確認

#### 4.2 Streamable HTTPテスト
**タスク**:
- [ ] HTTP サーバーの起動確認
- [ ] `POST /mcp` でのinitialize確認
- [ ] `GET /mcp` でのSSEストリーム確認
- [ ] tools/listとtools/callの動作確認
- [ ] セッション管理の動作確認
- [ ] cURLでの基本動作テスト

#### 4.3 統合テスト
**ファイル**: 新規作成 `server/tests/streamable_http_integration.rs`

**内容**:
```rust
// Streamable HTTP統合テスト
// - 初期化フロー
// - ツール呼び出し
// - セッション管理
// - エラーハンドリング
```

**タスク**:
- [ ] HTTP統合テストの実装
- [ ] Unity Bridgeとの連携テスト
- [ ] エラーケースのテスト

### フェーズ5: ドキュメントと設定

#### 5.1 README.md更新
**ファイル**: `README.md`

**タスク**:
- [ ] Streamable HTTP使用方法の追加
- [ ] 環境変数設定の説明
- [ ] curlでのテスト例の追加
- [ ] クライアント接続方法の追加

#### 5.2 CLAUDE.md更新
**ファイル**: `CLAUDE.md`

**タスク**:
- [ ] 開発コマンドの更新
- [ ] 新しい環境変数の説明
- [ ] HTTPモードでのテスト方法

#### 5.3 設定ファイル対応
**タスク**:
- [ ] config.tomlでのHTTP設定サポート
- [ ] 環境変数の整理
- [ ] セキュリティ設定（認証など）

### フェーズ6: CI/CDとデプロイメント

#### 6.1 CI更新
**ファイル**: `.github/workflows/ci.yml`

**タスク**:
- [ ] HTTP モードでのテスト追加
- [ ] 両方のモード（stdio/http）でのテスト
- [ ] 統合テストの実行

#### 6.2 Docker対応
**タスク**:
- [ ] Dockerfileの更新（必要に応じて）
- [ ] docker-compose.ymlでのHTTPモード対応

## 実装優先順位

### 高優先度 (必須)
1. Cargo.tomlの依存関係更新
2. McpServiceの構造修正
3. serve_httpメソッドの実装
4. main.rsの更新
5. 基本動作テスト

### 中優先度 (推奨)
6. 統合テスト実装
7. ドキュメント更新
8. 設定外部化

### 低優先度 (将来対応)
9. CI/CD更新
10. セキュリティ強化
11. パフォーマンス最適化

## 注意事項

### 互換性維持
- 既存のstdio動作は維持する
- Unity Bridgeとの連携は影響を受けない
- 既存のツール実装は変更不要

### セキュリティ考慮
- HTTPモードでは認証の検討が必要
- CORS設定の検討
- セッション管理のセキュリティ

### パフォーマンス
- SSEキープアライブの適切な設定
- セッションタイムアウトの設定
- メモリ使用量の監視

## 完了条件

- [ ] stdio/http両モードでの動作確認
- [ ] Unity Bridgeとの連携確認
- [ ] 全ツールの動作確認
- [ ] 基本的なHTTPテストの実行成功
- [ ] ドキュメント更新完了

## 予想作業時間
- フェーズ1-3: 2-3日
- フェーズ4-5: 1-2日
- フェーズ6: 1日

総計: 4-6日程度