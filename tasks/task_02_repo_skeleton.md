# Task 2: リポジトリスケルトンの作成

## 目的
Unity MCP Server プロジェクトのためのディレクトリ構造とCargo プロジェクトの基本スケルトンを作成する。

## 依存関係
- Task 1: 前提条件のインストール（Rust toolchain と protoc のインストール）

## 要件
- 正規のディレクトリ構造を作成する
- Cargo バイナリプロジェクトを初期化する
- proto ファイル用のディレクトリ構造を準備する

## 実行手順

### 1. ディレクトリ構造の作成
```bash
mkdir -p proto/mcp/unity/v1
cd unity-mcp  # プロジェクトルート
```

### 2. Cargo プロジェクトの初期化
```bash
cargo new server --bin
```

## 期待されるディレクトリ構造
作成後の構造：
```
unity-mcp/
├─ proto/
│  └─ mcp/unity/v1/
│     ├─ common.proto              (Task 4 で作成)
│     ├─ editor_control.proto      (Task 5 で作成)
│     ├─ assets.proto              (Task 6 で作成)
│     ├─ build.proto               (Task 6 で作成)
│     ├─ operations.proto          (Task 6 で作成)
│     └─ events.proto              (Task 7 で作成)
└─ server/
   ├─ Cargo.toml
   ├─ build.rs                     (Task 8 で作成)
   └─ src/
      └─ main.rs
```

## 受入基準
1. `proto/mcp/unity/v1/` ディレクトリが存在する
2. `server/Cargo.toml` ファイルが存在し、有効な Cargo プロジェクトである
3. `server/src/main.rs` ファイルが存在する
4. `cargo check -p server` がエラーなく実行される

## 検証コマンド
```bash
# ディレクトリ構造の確認
ls -la proto/mcp/unity/v1/
ls -la server/

# Cargo プロジェクトの確認
cd server && cargo check
```

## 次のタスク
- Task 3: Rust 依存関係の設定

## メモ
- この時点では proto ファイルは空のディレクトリのみ
- server/Cargo.toml はデフォルトの内容で、次のタスクで依存関係を追加する