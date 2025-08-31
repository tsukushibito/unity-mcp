# コミュニケーションガイドライン
**対話言語:**
- 回答は日本語で行う

# リポジトリガイドライン

## プロジェクト構造・モジュール構成
- `server/`: Rust MCP サーバー。エントリポイントは `src/main.rs`、モジュールは `src/grpc/`、`src/unity.rs`、クレート設定は `Cargo.toml`、ビルドスクリプトは `build.rs`（`proto/unity_mcp.proto` から gRPC コードを生成）
- `bridge/`: Unity プロジェクト（Assets、Packages、ProjectSettings）。Unity Editor でこのフォルダを開いてください
- `proto/`: Protocol buffers（`unity_mcp.proto`）
- `docs/`: アーキテクチャとコーディングメモ（`directory-structure.md`、`rust-coding-guidelines.md` を参照）
- `tasks/`: 実行計画とマイルストーン仕様

## ビルド・テスト・開発コマンド
- `cd server && cargo build`: Rust サーバーをコンパイルし、gRPC バインディングを生成
- `cd server && cargo run`: MCP サーバーを開始（env フィルター付き `tracing` を使用）
- `cd server && cargo test`: ユニット/統合テスト（存在する場合）を実行
- `cd server && cargo fmt --all`: Rust コードベースをフォーマット
- `cd server && cargo clippy --all-targets -- -D warnings`: リントを実行し、警告がある場合は失敗
- Unity: Unity Editor で `bridge/` を開いてください。クライアント側実験には Play Mode を使用

## コーディングスタイル・命名規則
- Rust: `rustfmt` デフォルトに従い、PR 前にフォーマットを実行。`snake_case`（関数/モジュール）、`UpperCamelCase`（型）、`SCREAMING_SNAKE_CASE`（定数）を推奨。テスト以外のコードでは `unwrap`/`expect` を避け、`anyhow::Result` と `?` を使用
- モジュール: 2018+ スタイル（`mod.rs` なし）。ファイルは `module.rs` として配置、サブモジュールは `module/child.rs` として `pub mod child;` と記述

## テストガイドライン
- フレームワーク: Rust 組み込みテストで `#[tokio::test]` による async 対応
- 配置: ユニットテストは `#[cfg(test)]` 下にインライン配置、統合テストは `server/tests/` に配置
- 命名: テストファイル/関数は動作を反映（例：`handles_stream_disconnect.rs`、`test_reconnect_backoff`）。`cargo test` で実行

## コミット・プルリクエストガイドライン
- コミット: 実用的な範囲で Conventional Commits を推奨（`feat:`、`chore:`、`fix:`）。命令法現在時制を使用、件名は約72文字以下（例：`feat: add Unity CI workflow`）
- PR: 明確な説明を提供、issue をリンク、破壊的変更を注記、ユーザー向け更新にはログ/スクリーンショットを含める。`fmt`/`clippy` が通ることを確認し、ドキュメントを更新

## セキュリティ・設定のヒント
- 設定: デフォルト設定は `server/config/` 下に保存（TOML/YAML）。実装に応じてフラグ/env でオーバーライド
- ログ: `RUST_LOG` で詳細度を制御。例：`RUST_LOG=server=debug cargo run`

## 利用可能な MCP ツール

### Unity Bridge Tools
- `unity_bridge_status`: Bridge接続状態を取得
- `unity_health`: Unity Editorのヘルスチェック
- `unity_assets_import`: アセットインポート
- `unity_assets_move`: アセット移動
- `unity_assets_delete`: アセット削除
- `unity_assets_refresh`: AssetDatabase更新
- `unity_assets_guid_to_path`: GUID → パス変換
- `unity_assets_path_to_guid`: パス → GUID変換

### Unity C# Compile Diagnostics
- `unity_get_compile_diagnostics`: C#コンパイル診断結果を取得
  - パラメータ: `severity` (error/warning/info/all), `max_items`, `assembly`, `changed_only`
  - Unity側で Unityプロジェクト直下の `Temp/AI/latest.json` に診断データを出力
  - 環境変数 `UNITY_MCP_DIAG_PATH` でプロジェクトルートからの相対パスを上書き可能

### Unity TestRunner Execution
- `unity_run_tests`: Unity EditMode/PlayMode テストを実行
  - パラメータ: `mode` (edit/play/all), `test_filter`, `categories`, `timeout_sec`, `max_items`, `include_passed`
  - Unity側で `bridge/Temp/AI/tests/latest.json` にテスト結果を出力
  - 環境変数 `UNITY_MCP_REQ_PATH`, `UNITY_MCP_TESTS_PATH` でパスカスタマイズ可能
- `unity_get_test_results`: テスト実行結果を取得
  - パラメータ: `run_id`, `max_items`, `include_passed`
  - 特定の実行IDまたは最新結果を取得可能
