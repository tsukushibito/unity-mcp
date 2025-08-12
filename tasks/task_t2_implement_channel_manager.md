# T2 — Implement ChannelManager

## 概要
再利用可能なChannelManagerを実装し、gRPCクライアント接続を管理します。

## 成果物

### `server/src/grpc/channel.rs`
- ChannelManagerの実装
- 接続管理とクライアント生成機能
- トークンベースの認証メタデータ注入

## 実装詳細

### 必要メソッド
- `connect`: gRPCサーバーへの接続を確立
- `with_meta`: 認証メタデータの注入
- `editor_control_client`: EditorControlクライアントの生成

### 設定要件
- `Endpoint`を`timeout(Duration::from_secs(cfg.default_timeout_secs))`で構成
- オプショナルなトークン注入サポート
- 生成されたprotoコードの一回のinclude

### アーキテクチャ制約
- レガシーな `Request::set_timeout` 使用法を削除
- エンドポイントレベルのタイムアウトに依存

## 実装スケルトン

### 構造体定義
```rust
pub struct ChannelManager {
    channel: Channel,
    token: Option<String>,
}
```

### 基本メソッド
```rust
impl ChannelManager {
    pub async fn connect(config: &GrpcConfig) -> Result<Self> { ... }
    pub fn with_meta(&self, req: Request<T>) -> Request<T> { ... }
    pub fn editor_control_client(&self) -> EditorControlClient<Channel> { ... }
}
```

## 受入条件
- ChannelManagerが正常にビルドされること
- 接続とメタデータ注入が動作すること
- EditorControlクライアントが適切に生成されること
- エンドポイントレベルのタイムアウト設定が機能すること