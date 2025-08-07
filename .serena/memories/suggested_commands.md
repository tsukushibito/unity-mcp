# Unity MCP Server - 推奨開発コマンド

## Rust Server コマンド (server/ ディレクトリで実行)

### ビルドとチェック
```bash
# 基本ビルド
cargo build

# 高速チェック（型チェックのみ）
cargo check

# フォーマット確認
cargo fmt --all -- --check

# フォーマット適用
cargo fmt --all

# Lint チェック
cargo clippy --all-targets -- -D warnings
```

### テスト
```bash
# 全テスト実行
cargo test

# 特定のテストを実行
cargo test <module_or_test_name>

# テスト時詳細出力
cargo test -- --nocapture
```

### サーバー実行
```bash
# サーバー起動（stdio モード）
cargo run
```

## Unity Bridge コマンド

### テスト実行（Unity CLI）
```bash
# EditMode テスト実行
Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
```

## 全般コマンド

### Git コマンド（Conventional Commits）
```bash
# コミットメッセージ例
git commit -m "feat: add Unity MCP handler"
git commit -m "fix: resolve connection timeout issue"
git commit -m "docs: update README with setup instructions"
```

### CI/CD
```bash
# GitHub Actions で自動実行されるコマンド
# - cargo fmt --all -- --check
# - cargo clippy --all-targets -- -D warnings  
# - cargo build --verbose
# - cargo test --verbose
```

## Linux システムコマンド
```bash
# ファイル操作
ls -la          # ディレクトリ一覧表示
find . -name    # ファイル検索
grep -r         # テキスト検索
cd              # ディレクトリ移動

# プロセス管理
ps aux          # プロセス一覧
kill -9         # プロセス強制終了
```

## 開発ツール
```bash
# VS Code Dev Container 使用時
# 必要なツールチェーンは自動でセットアップされます
# - Rust toolchain (stable)
# - .NET SDK
# - Node.js
# - GitHub CLI
```