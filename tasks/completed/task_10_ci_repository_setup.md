# Task 10: CI/CD とリポジトリ設定

## 目的
Unity MCP Server プロジェクトのための CI/CD パイプライン、Git 管理、変更管理ガイドラインを設定する。

## 依存関係
- Task 1-9: すべての前段階タスクの完了
- GitHub リポジトリの存在

## 要件
- macOS と Ubuntu での自動ビルドテスト
- 適切な Git 無視設定
- スキーマ変更管理ガイドライン
- 再現可能な CI 環境

## 実行手順

### 1. .gitignore の作成・更新
`unity-mcp/.gitignore` を作成または更新：

```gitignore
/target
**/*.rs.bk
.DS_Store

# IDE files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
Thumbs.db
```

### 2. GitHub Actions CI の設定
`.github/workflows/rust-ci.yml` を作成：

```yaml
name: Rust CI
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install protoc
        run: |
          sudo apt-get update
          sudo apt-get install -y protobuf-compiler
          protoc --version
      - uses: dtolnay/rust-toolchain@stable
      - name: Build server
        run: cargo build -p server --locked --verbose
      - name: Run tests
        run: cargo test -p server --locked --verbose
      - name: Check formatting
        run: cargo fmt --all -- --check
      - name: Run clippy
        run: cargo clippy --all-targets -- -D warnings
```

### 3. プロジェクトルートの README.md 更新（オプション）
Unity MCP Server の概要説明を含む README.md の作成

## 変更管理ガイドライン

### スキーマフリーズまで（PROVISIONAL 期間）
- **破壊的変更許可**: フィールド番号変更、メッセージ/サービス名変更、分割/統合
- **変更プロセス**: 
  1. PR に `proto:breaking` ラベルを付与
  2. コンシューマー（Unity Bridge）との調整
  3. `cargo clean && cargo build -p server` でクライアント再生成を必須化

### スキーマフリーズ後
- **フリーズ実行**:
  1. エンドツーエンド検証完了
  2. `schema-freeze-v1` タグの作成
  3. `docs/SCHEMA_FREEZE.md` でコミットハッシュ記録

- **フリーズ後の変更ルール**:
  1. **後方互換のみ**: 新しいオプショナルフィールドを新しい番号で追加
  2. **削除禁止**: `reserved` を使用してフィールド番号を予約
  3. **番号再利用禁止**: ワイヤープロトコル契約の維持
  4. **変更ログ**: `docs/PROTO_CHANGELOG.md` の維持

### 破壊的変更が必要な場合
- 新パッケージブランチ: `mcp.unity.v2`
- googleapis 検討は `v2` でのみ

## ファイル構成

### 生成されるファイル
```
unity-mcp/
├─ .github/
│  └─ workflows/
│     └─ rust-ci.yml         # CI 設定
├─ .gitignore                # Git 無視設定
├─ proto/                    # Protocol Buffers 定義
│  └─ mcp/unity/v1/
│     ├─ common.proto
│     ├─ editor_control.proto
│     ├─ assets.proto
│     ├─ build.proto
│     ├─ operations.proto
│     └─ events.proto
└─ server/                   # Rust gRPC クライアント
   ├─ Cargo.toml
   ├─ build.rs
   └─ src/
      └─ main.rs
```

### 除外されるファイル
- `target/` - Rust ビルド成果物
- 生成された Rust コード（`OUT_DIR` 配下）
- IDE 固有ファイル
- OS 固有ファイル

## 受入基準
1. `.gitignore` ファイルが適切に設定されている
2. GitHub Actions workflow が正しく設定されている
3. CI が Ubuntu 環境で以下を実行する：
   - protoc のインストール確認
   - `cargo build -p server --locked` の成功
   - `cargo test -p server --locked` の成功（テストがある場合）
   - `cargo fmt --check` の成功
   - `cargo clippy` の成功（警告なし）
4. 変更管理ガイドラインが文書化されている

## 検証コマンド

### ローカル環境
```bash
# .gitignore のテスト
git status  # target/ が無視されることを確認

# CI で実行されるコマンドのローカル確認
cargo build -p server --locked --verbose
cargo test -p server --locked --verbose
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

### CI 環境
- GitHub に push/PR 作成時の自動実行
- ビルドの成功確認
- 全チェックの通過確認

## トラブルシューティング
| 症状 | 原因 | 修正 |
|---|---|---|
| CI で `protoc: command not found` | protoc インストールステップの問題 | workflow の `Install protoc` ステップを確認 |
| `cargo build --locked` 失敗 | Cargo.lock の不整合 | ローカルで `cargo update` 後に Cargo.lock をコミット |
| `cargo fmt --check` 失敗 | フォーマット不統一 | `cargo fmt` を実行してフォーマット修正 |

## 完了基準（全体の Definition of Done）
1. `cargo build -p server` が macOS と Ubuntu で成功
2. `cargo run -p server` が成功メッセージを表示
3. GitHub Actions CI がクリーンなクローンで通過
4. 6つの proto ファイルがすべて存在しコンパイル可能
5. 後続の proto 変更が `.proto` と `build.rs` の編集のみで完了

## 次のステップ（このタスクリストの対象外）
- Unity Bridge の gRPC サーバー実装
- 実際のクライアント呼び出し実装
- モック Bridge に対する契約テスト（オプション）

## メモ
- CI は Ubuntu のみ（macOS サポートは開発者環境で確保）
- proto ファイルは単一リポジトリで管理され、変更時は Unity Bridge 側も調整が必要
- L0 ポリシーにより googleapis 依存は回避