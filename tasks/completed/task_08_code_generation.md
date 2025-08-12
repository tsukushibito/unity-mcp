# Task 8: コード生成設定

## 目的
proto ファイルから Rust gRPC クライアントスタブを生成するための `build.rs` スクリプトを作成し、ビルドプロセスを設定する。

## 依存関係
- Task 3: Rust 依存関係の設定（tonic-build の存在）
- Task 4-7: すべての proto ファイルの作成

## 要件
- proto ファイルから Rust コードの自動生成
- gRPC **クライアント**スタブのみ生成（サーバーは Unity Bridge 側）
- 再現可能なビルドプロセス
- proto ファイル変更時の自動再ビルド

## 実行手順

### `server/build.rs` の作成
以下の内容で **正確に** ファイルを作成する：

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = &[
        "proto/mcp/unity/v1/common.proto",
        "proto/mcp/unity/v1/editor_control.proto",
        "proto/mcp/unity/v1/assets.proto",
        "proto/mcp/unity/v1/build.proto",
        "proto/mcp/unity/v1/operations.proto",
        "proto/mcp/unity/v1/events.proto",
    ];

    tonic_build::configure()
        .build_server(false) // Rust side = gRPC client only
        .compile(protos, &["proto"]) ?;

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
```

## コード詳細解説

### プロトコルファイルリスト
```rust
let protos = &[
    "proto/mcp/unity/v1/common.proto",
    "proto/mcp/unity/v1/editor_control.proto", 
    "proto/mcp/unity/v1/assets.proto",
    "proto/mcp/unity/v1/build.proto",
    "proto/mcp/unity/v1/operations.proto",
    "proto/mcp/unity/v1/events.proto",
];
```
- 全6つの proto ファイルを一括処理
- 新しい proto ファイル追加時はこのリストに追加

### tonic_build 設定
```rust
tonic_build::configure()
    .build_server(false) // gRPC クライアントのみ
    .compile(protos, &["proto"]) ?;
```

#### 重要な設定項目
- `.build_server(false)`: サーバースタブを生成しない
  - Unity Bridge が gRPC サーバーとして動作
  - Rust 側は gRPC **クライアント**として動作
- `.compile(protos, &["proto"])`: 
  - `protos`: コンパイル対象ファイル
  - `&["proto"]`: インクルードパス（proto/ ディレクトリ）

### 再ビルドトリガー
```rust
println!("cargo:rerun-if-changed=proto");
```
- proto ディレクトリ配下のファイル変更時に自動再ビルド
- 効率的な増分ビルドを実現

## 生成されるコード

### 出力先
- Cargo の `OUT_DIR` 環境変数で指定されるディレクトリ
- 通常 `target/debug/build/server-[hash]/out/`
- **注意**: カスタム `out_dir` は設定しない

### 生成されるファイル
```
OUT_DIR/
├─ mcp.unity.v1.rs       # すべてのサービスとメッセージ
└─ [その他の生成ファイル]
```

### パッケージマッピング
- proto パッケージ: `mcp.unity.v1`
- Rust モジュール: `mcp_unity_v1` (ドット→アンダースコア)

## 受入基準
1. `server/build.rs` ファイルが正確な内容で存在する
2. 6つの proto ファイルがすべてリストに含まれている
3. `build_server(false)` が設定されている
4. `cargo build -p server` が初回実行時にコード生成を実行する
5. proto ファイル変更時に再ビルドが自動実行される

## 検証コマンド
```bash
cd server

# 初回ビルド（コード生成を含む）
cargo clean && cargo build -v

# 生成されたファイルの確認
find target -name "*.rs" -path "*/out/*" | grep mcp

# proto 変更時の再ビルドテスト
touch ../proto/mcp/unity/v1/common.proto
cargo build -v  # 再ビルドが実行されることを確認
```

## トラブルシューティング
| 症状 | 原因 | 修正 |
|---|---|---|
| `protoc: command not found` | Protocol Buffers コンパイラ未インストール | Task 1 の前提条件を確認 |
| `file not found: mcp/unity/v1/common.proto` | インクルードパスの設定ミス | `&["proto"]` の設定を確認 |
| `duplicate symbol` エラー | パッケージ名の重複 | すべての proto ファイルで `package mcp.unity.v1;` を確認 |

## 次のタスク
- Task 9: メインアプリケーション実装（生成されたスタブの使用）

## メモ
- `OUT_DIR` は Cargo が管理するため、直接指定しない
- 新しい proto ファイル追加時は、このリストの更新が必要
- クライアントのみ生成により、Unity Bridge との役割分担が明確化