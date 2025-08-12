# T3 — Wire up a minimal client (EditorControl)

## 概要
EditorControlクライアントを使用した最小限のクライアント実装を行います。

## 成果物

### `server/src/main.rs` の更新
- tracingの初期化
- 設定読み込みとChannelManager接続
- Health RPC呼び出しとログ出力

## 実装詳細

### 初期化シーケンス
1. トレーシング初期化
2. 設定読み込み (`GrpcConfig::from_env()`)
3. ChannelManager接続
4. EditorControlクライアント取得

### Health RPC呼び出し
- `Health` RPCを一度呼び出し
- 結果またはエラーをログ出力
- オフライン時でもパニックしない

### エラーハンドリング
- 接続エラー時の適切なログ出力
- プロセス終了時の適切な処理

## 実装スケルトン

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // トレーシング初期化
    tracing_subscriber::fmt::init();
    
    // 設定読み込み
    let config = GrpcConfig::from_env();
    
    // ChannelManager接続
    let manager = ChannelManager::connect(&config).await?;
    
    // Health RPC呼び出し
    let mut client = manager.editor_control_client();
    match client.health(HealthRequest {}).await {
        Ok(response) => tracing::info!("Health check successful: {:?}", response),
        Err(e) => tracing::error!("Health check failed: {}", e),
    }
    
    Ok(())
}
```

## 受入条件
- main.rsが正常にビルドされること
- Health RPC呼び出しが実行されること
- オフライン時にパニックしないこと
- 適切なログ出力が行われること