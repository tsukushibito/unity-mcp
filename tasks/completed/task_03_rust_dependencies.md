# Task 3: Rust 依存関係の設定

## 目的
Unity MCP Server の gRPC クライアント実装に必要な Rust 依存関係を `server/Cargo.toml` に追加し、設定する。

## 依存関係
- Task 2: リポジトリスケルトンの作成（server プロジェクトの存在）

## 要件
- tonic/prost によるgRPCクライアントスタブ生成
- 再現可能なビルドのためのバージョン固定
- macOS および Ubuntu での動作保証

## 実行手順

### server/Cargo.toml の完全置換
`server/Cargo.toml` のコンテンツを以下で**完全に置き換える**：

```toml
[package]
name = "server"
version = "0.1.0"
edition = "2021"

[dependencies]
tonic = { version = "0.11", features = ["transport", "tls"] }
prost = "0.12"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
anyhow = "1"

[build-dependencies]
tonic-build = "0.11"
```

## 依存関係の詳細説明

### 実行時依存関係 ([dependencies])
- **tonic 0.11**: gRPC クライアント実装
  - `transport`: HTTP/2 トランスポート
  - `tls`: TLS サポート
- **prost 0.12**: Protocol Buffers の Rust 実装
- **tokio 1.x**: 非同期ランタイム
  - `macros`: `#[tokio::main]` などのマクロ
  - `rt-multi-thread`: マルチスレッドランタイム
- **anyhow 1.x**: エラーハンドリング

### ビルド時依存関係 ([build-dependencies])
- **tonic-build 0.11**: proto ファイルからRustコード生成

## 受入基準
1. `cargo metadata -p server` がエラーなく実行される
2. 依存関係のダウンロードが成功する
3. `cargo check -p server` がエラーなく実行される

## 検証コマンド
```bash
cd server
cargo metadata --format-version 1 | grep -E "(tonic|prost|tokio|anyhow|tonic-build)"
cargo check
```

## トラブルシューティング
| 症状 | 原因 | 修正 |
|---|---|---|
| `error: failed to parse manifest` | TOML 構文エラー | TOML 構文を確認し、インデントとクォーテーションを修正 |
| `error: no matching package found` | バージョンが存在しない | 指定されたバージョンを crates.io で確認 |
| 依存関係解決エラー | バージョン競合 | `cargo update` を実行するか、互換性のあるバージョンに調整 |

## 次のタスク
- Task 4: 共通 proto ファイルの作成

## メモ
- gRPC **クライアント**スタブのみ生成（`build_server(false)` を Task 8 で設定）
- Unity ブリッジ側が gRPC サーバーを公開する構成
- バージョンは再現可能なビルドのために固定