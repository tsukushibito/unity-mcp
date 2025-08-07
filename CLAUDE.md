# CLAUDE.md

このファイルは、このリポジトリでコードを扱う際にClaude Code (claude.ai/code) にガイダンスを提供します。

## プロジェクト概要

Unity MCP Server は、Rust MCP サーバーと Unity Editor ブリッジを組み合わせた双方向コンポーネントプロジェクトです。現在はスケルトン段階で最小限のコードですが、明確なアーキテクチャ方向性を持っています。

**アーキテクチャ:**
- `server/` - rmcp SDK を使用した複数トランスポート対応（stdio/WebSocket）のRust MCP サーバー
- `bridge/` - Rust サーバーを起動し連携するための Unity Editor ツール
- 高速フィードバックループを目的とした単一リポジトリアプローチ、将来のワークスペース拡張を想定

## 開発コマンド

**Rust Server (server/):**
```bash
# ビルドとチェック
cargo build
cargo check
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

# テスト
cargo test
cargo test <module_or_test_name>  # 特定のテストを実行
```

**Unity Bridge (bridge/):**
```bash
# テスト（後にCI経由で実行）
Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
```

**全般:**
- ラッパーコマンド追加時はscripts/ディレクトリを使用
- Dev Containerが事前設定済みツールチェーンを提供

## コード規約

**言語とフレームワーク:**
- サーバー用Rust（rmcp、tracing、非同期用tokio）
- Unity Editor/Runtime コンポーネント用C#

**インポート整理:**
- Rust: std → 外部クレート → ローカルモジュール
- C#: System → UnityEngine/UnityEditor → プロジェクト名前空間

**命名規則:**
- Rust: snake_case アイテム、CamelCase 型、SCREAMING_SNAKE_CASE 定数
- C#: PascalCase 型/メソッド、camelCase フィールド、UPPER_CASE 定数

**エラーハンドリング:**
- Rust: アプリケーションレベルはanyhow、ドメインエラーはthiserror、本番でunwrap/expectは避ける
- C#: try/catchとUnityEngine.Debugログ出力
- リクエストハンドラーでのパニックは禁止

**設定:**
- server/config/のTOMLファイル
- CLIフラグオーバーライドは後に予定

## プロジェクト構造

**主要ディレクトリ:**
- `bridge/Assets/MCP/Editor/` - Unity Editor 統合（MVP重視）
- `bridge/Packages/com.example.mcp-bridge/` - 再利用性のためのUPMパッケージ
- `docs/` - アーキテクチャドキュメント（日本語ディレクトリ構造ガイドを含む）

## テスト戦略

- Rust: cfg(test)でモジュール内にユニットテストを配置
- 追加時はserver/tests/に統合テスト
- テストは決定論的に保ち、デフォルトでネットワーク依存を避ける
- Unity: Unity Test RunnerによるEditModeテスト

## コミュニケーションガイドライン

**対話言語:**
- 回答は日本語で行う

## 作業ガイドライン

**作業指示:**
- 指示の内容に確証がない場合は作業を進めずに質問すること

## コミット規約

**コミットメッセージ:**
- コミットメッセージはConventionalCommitに従った英語のメッセージとしてください