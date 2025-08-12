# Task 1: 前提条件のインストール

## 目的
Unity MCP Server の gRPC プロトコルコンパイレーション環境のために、必要な前提条件ツールをインストールする。

## 依存関係
- なし（初期タスク）

## 要件
- **Rust toolchain**: stable（1.79+）
- **protoc**: Protocol Buffers コンパイラ ≥ 3.21
- **システム**: macOS および Ubuntu 対応

## 実行手順

### macOS の場合
```bash
brew install protobuf rustup-init
rustup toolchain install stable
rustup default stable
protoc --version  # must be >= 3.21
```

### Ubuntu の場合
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler curl build-essential pkg-config
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
rustup default stable
protoc --version  # must be >= 3.21
```

## 受入基準
1. `protoc --version` が ≥ 3.21 を表示する
2. `rustc --version` が stable を表示する
3. 両方のコマンドがエラーなく実行される

## トラブルシューティング
| 症状 | 原因 | 修正 |
|---|---|---|
| `protoc: command not found` | Protobuf がインストールされていない、またはPATHが設定されていない | `protobuf-compiler` (Ubuntu) / `brew install protobuf` (macOS) をインストール。シェルを再起動してPATHを更新。 |
| `rustc: command not found` | Rust toolchain がインストールされていない、またはPATHが設定されていない | rustup をインストールし、シェルを再起動してPATHを更新。 |

## 次のタスク
- Task 2: リポジトリスケルトンの作成

## メモ
- このタスクは開発環境のセットアップのため、CI/CD環境でも同様の手順が必要
- バージョン固定により再現可能なビルド環境を確保