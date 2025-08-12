# T1 — Add configuration module

## 概要
gRPCクライアント用の型付き環境設定モジュールを実装します。

## 成果物

### `server/src/config.rs`
- `GrpcConfig { addr: String, token: Option<String>, default_timeout_secs: u64 }` 構造体を定義
- 環境変数から設定を読み込む `GrpcConfig::from_env()` メソッドを実装
- ブリッジアドレス、トークン、タイムアウトの環境変数読み込み機能

## テスト要件
- デフォルト値とパースのカバー範囲をテストするユニットテスト

## 実装詳細

### 設定構造体
```rust
#[derive(Debug, Clone)]
pub struct GrpcConfig {
    pub addr: String,
    pub token: Option<String>,
    pub default_timeout_secs: u64,
}
```

### 環境変数マッピング
- `MCP_BRIDGE_ADDR` → `addr`
- `MCP_BRIDGE_TOKEN` → `token`
- `MCP_BRIDGE_TIMEOUT` → `default_timeout_secs`

### デフォルト値
- `addr`: `"http://localhost:8080"`
- `token`: `None`
- `default_timeout_secs`: `30`

## 受入条件
- 設定モジュールが正常にビルドされること
- ユニットテストがパス
- 環境変数の有無に関係なく適切なデフォルト値を返すこと