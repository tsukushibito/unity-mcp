# Task 9: メインアプリケーション実装

## 目的
生成された gRPC クライアントスタブを使用して、Unity MCP Server のメインアプリケーションを実装する。

## 依存関係
- Task 8: コード生成設定（build.rs による Rust コード生成）
- Task 3: Rust 依存関係の設定

## 要件
- 生成されたクライアントスタブの適切な統合
- 基本的な接続テスト機能
- エラーハンドリングと非同期実行
- 実行時検証（Unity Bridge なしでもビルド・起動可能）

## 実行手順

### `server/src/main.rs` の完全置換
以下の内容で `server/src/main.rs` を **完全に置き換える**：

```rust
use anyhow::Result;

// Maps to files generated in OUT_DIR by tonic-build. Keep package path identical to .proto.
pub mod mcp_unity_v1 {
    pub mod editor_control { tonic::include_proto!("mcp.unity.v1"); }
}

// You can include more modules if you want separated namespaces; minimum demo uses EditorControl only.

#[tokio::main]
async fn main() -> Result<()> {
    // Just prove the types exist and the binary links; connection can fail if bridge is not running.
    // When the bridge is available at 127.0.0.1:50051, this will connect.
    let _ = tonic::transport::Channel::from_static("http://127.0.0.1:50051");

    println!("gRPC client stubs compiled and binary runs.");
    Ok(())
}
```

## コード詳細解説

### モジュール定義
```rust
pub mod mcp_unity_v1 {
    pub mod editor_control { tonic::include_proto!("mcp.unity.v1"); }
}
```

#### include_proto! マクロ
- `tonic::include_proto!("mcp.unity.v1")`: proto パッケージ名を指定
- `OUT_DIR` から生成されたコードを読み込む
- パッケージ名は proto ファイルの `package mcp.unity.v1;` と一致

#### モジュール構造オプション
**単一モジュール** (現在の実装):
```rust
pub mod editor_control { tonic::include_proto!("mcp.unity.v1"); }
```
- すべてのサービスが一つのモジュールに含まれる
- シンプルで最小限の実装

**複数モジュール** (選択肢):
```rust
pub mod editor_control { tonic::include_proto!("mcp.unity.v1"); }
pub mod assets         { tonic::include_proto!("mcp.unity.v1"); }
pub mod build          { tonic::include_proto!("mcp.unity.v1"); }
// 名前空間を分けたい場合
```

### メイン関数
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let _ = tonic::transport::Channel::from_static("http://127.0.0.1:50051");
    println!("gRPC client stubs compiled and binary runs.");
    Ok(())
}
```

#### 重要なポイント
- **`#[tokio::main]`**: 非同期実行環境
- **Channel 作成**: Unity Bridge への接続準備（実際の接続は試行しない）
- **`let _ = ...`**: 接続失敗を無視（Unity Bridge がなくても実行可能）
- **成功メッセージ**: ビルドとリンクの成功を示す

## 生成されるクライアント型

### 期待される型（生成後）
```rust
// これらの型が生成される（使用例）
use mcp_unity_v1::editor_control::*;

// クライアント型
EditorControlClient<tonic::transport::Channel>
AssetsClient<tonic::transport::Channel>
BuildClient<tonic::transport::Channel>
OperationsClient<tonic::transport::Channel>
EventsClient<tonic::transport::Channel>

// メッセージ型
HealthRequest, HealthResponse
GetPlayModeResponse
SetPlayModeRequest, SetPlayModeResponse
// 他多数...
```

### 将来の拡張例（参考）
```rust
// Task 完了後の実際の使用例（実装しない）
let mut client = EditorControlClient::connect("http://127.0.0.1:50051").await?;
let response = client.health(HealthRequest {}).await?;
println!("Unity Editor Status: {}", response.into_inner().status);
```

## 受入基準
1. `server/src/main.rs` が指定されたコードで正確に置き換えられている
2. `cargo build -p server` がエラーなく成功する
3. `cargo run -p server` が指定されたメッセージを出力する
4. 生成されたクライアント型がコンパイルされている
5. Unity Bridge への実際の接続は不要（スタブ検証のみ）

## 検証コマンド
```bash
cd server

# ビルドテスト
cargo build -v

# 実行テスト
cargo run

# 期待される出力の確認
# -> "gRPC client stubs compiled and binary runs."
```

## トラブルシューティング
| 症状 | 原因 | 修正 |
|---|---|---|
| `include_proto!` でファイルが見つからない | パッケージ名の不一致 | proto ファイルの `package mcp.unity.v1;` を確認 |
| コンパイルエラー（重複定義） | 複数の include_proto で同じパッケージ | モジュール構造を単一にする |
| `cannot find EditorControlClient` | コード生成の失敗 | `cargo clean && cargo build` で再生成 |

## 次のタスク
- Task 10: CI/CD とリポジトリ設定

## メモ
- この段階では Unity Bridge への実接続は不要
- コード生成とクライアントスタブの動作確認が主目的
- 将来の実装では、実際の gRPC 呼び出しとストリーミング処理を追加