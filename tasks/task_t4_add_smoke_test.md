# T4 — Add a smoke test (no Unity Bridge required)

## 概要
Unity Bridgeを必要としないスモークテストを実装し、ChannelManagerの動作を検証します。

## 成果物

### `server/tests/smoke.rs`
- 最小限のtonicサーバーを起動
- ChannelManagerで接続しラウンドトリップをテスト
- サーバースタブ生成のための環境フラグ対応

## 実装詳細

### テスト構成
- EditorControl用の最小限tonicサーバー
- インプロセスでのサーバー起動
- ChannelManagerを使用したクライアント接続
- ラウンドトリップの検証

### 環境フラグ対応
`build.rs`が`build_server(false)`を使用しているため、テストビルド時にサーバースタブ生成を有効にする環境フラグを追加：
- `TONIC_BUILD_SERVER=1`

### テストシナリオ
1. テストサーバー起動
2. ChannelManager接続
3. Health RPC呼び出し
4. レスポンス検証
5. クリーンアップ

## 実装スケルトン

```rust
use std::time::Duration;
use tokio::time::timeout;
use tonic::{transport::Server, Request, Response, Status};

// テスト用サーバー実装
#[derive(Debug, Default)]
pub struct TestEditorControlService {}

#[tonic::async_trait]
impl EditorControl for TestEditorControlService {
    async fn health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            status: "OK".to_string(),
        }))
    }
}

#[tokio::test]
async fn test_channel_manager_roundtrip() {
    // テストサーバー起動
    let addr = "127.0.0.1:0".parse().unwrap();
    let service = TestEditorControlService::default();
    
    let server = Server::builder()
        .add_service(EditorControlServer::new(service))
        .serve_with_shutdown(addr, async { /* shutdown logic */ });
    
    // ChannelManager接続テスト
    let config = GrpcConfig {
        addr: format!("http://{}", addr),
        token: None,
        default_timeout_secs: 5,
    };
    
    let manager = ChannelManager::connect(&config).await.unwrap();
    let mut client = manager.editor_control_client();
    
    // Health RPC テスト
    let response = timeout(Duration::from_secs(5), client.health(HealthRequest {}))
        .await
        .unwrap()
        .unwrap();
    
    assert_eq!(response.into_inner().status, "OK");
}
```

## ビルド設定

### `build.rs` 更新
```rust
fn main() {
    let build_server = std::env::var("TONIC_BUILD_SERVER")
        .map(|v| v == "1")
        .unwrap_or(false);
    
    tonic_prost_build::Builder::new()
        .build_server(build_server)
        // ... 他の設定
        .compile(&["proto/mcp/unity/v1/editor_control.proto"], &["proto"])
        .unwrap();
}
```

## 受入条件
- スモークテストが正常にパス
- サーバースタブ生成が適切に制御されること
- ChannelManagerのラウンドトリップが検証されること
- テスト実行時にUnity Bridgeが不要であること