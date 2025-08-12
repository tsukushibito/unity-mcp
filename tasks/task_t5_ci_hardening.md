# T5 — CI hardening

## 概要
CI環境の強化を行い、安定したビルドとテスト実行を確保します。

## 成果物

### CI設定の更新
- `protoc`バージョンの固定
- Cargoキャッシュの追加
- clippy、fmtチェックの追加
- テスト実行の追加

## 実装詳細

### protoc バージョン固定
- 特定のprotocバージョンを指定してセットアップ
- ローカルとCI環境の一貫性を確保

### Cargoキャッシュ設定
- 依存関係のキャッシュでビルド時間短縮
- `~/.cargo/registry` と `~/.cargo/git` のキャッシュ

### 品質チェック追加
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- 既存テストに加えてスモークテストの実行

### CI ワークフロー例

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Install protoc
      run: |
        sudo apt-get update
        sudo apt-get install -y protobuf-compiler=3.21.*
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        components: rustfmt, clippy
        override: true
    
    - name: Cache cargo
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          server/target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Check formatting
      run: cargo fmt --all -- --check
      working-directory: server
    
    - name: Lint
      run: cargo clippy --all-targets -- -D warnings
      working-directory: server
    
    - name: Build
      run: cargo build
      working-directory: server
    
    - name: Run tests
      run: cargo test
      working-directory: server
      env:
        TONIC_BUILD_SERVER: 1
```

## macOS対応

### ローカル開発環境
- Homebrewでのprotocインストール手順
- 同じバージョン制約の適用

```bash
# macOS setup
brew install protobuf@21
export PATH="/opt/homebrew/opt/protobuf@21/bin:$PATH"
```

## 受入条件
- CI環境でのビルドが安定すること
- 全てのチェック（fmt、clippy、テスト）がパス
- キャッシュによるビルド時間短縮が機能すること
- ローカル（macOS/Ubuntu）とCI環境で一貫した結果が得られること