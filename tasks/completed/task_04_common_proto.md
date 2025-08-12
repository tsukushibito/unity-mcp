# Task 4: 共通 proto ファイルの作成

## 目的
Unity MCP Server gRPC プロトコルの共通メッセージとタイプを定義する `common.proto` ファイルを作成する。

## 依存関係
- Task 2: リポジトリスケルトンの作成（proto ディレクトリの存在）
- Task 3: Rust 依存関係の設定

## 要件
- L0 ポリシー：google.protobuf.Empty の代わりに独自の Empty メッセージを定義
- 他の proto ファイルで参照される共通型の定義
- 最小限だが実用的な実装

## 実行手順

### `proto/mcp/unity/v1/common.proto` の作成
以下の内容で **正確に** ファイルを作成する：

```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
syntax = "proto3";
package mcp.unity.v1;

// Generic empty placeholder to avoid google.protobuf.Empty at L0.
message Empty {}

// Minimal operation reference for streaming/event demos.
message OperationRef {
  string id = 1; // opaque identifier
}
```

## ファイル構造の説明

### ヘッダー
- **STATUS コメント**: スキーマフリーズまで破壊的変更を許可
- **PACKAGE コメント**: パッケージ名の明示
- **syntax**: proto3 を明示指定
- **package**: `mcp.unity.v1` - 他のすべての proto ファイルと統一

### メッセージ定義

#### Empty メッセージ
```proto
message Empty {}
```
- google.protobuf.Empty の L0 代替
- 空のレスポンスやパラメーターなしリクエストに使用
- 他の proto ファイルで `import "mcp/unity/v1/common.proto"` として参照

#### OperationRef メッセージ
```proto
message OperationRef {
  string id = 1; // opaque identifier
}
```
- 非同期操作の参照に使用
- ストリーミング/イベントのデモンストレーション用
- 不透明な識別子として設計

## 受入基準
1. ファイルが正確なパスに存在する：`proto/mcp/unity/v1/common.proto`
2. パッケージ名が `mcp.unity.v1` で統一されている
3. 未解決のインポートが存在しない
4. proto3 構文に準拠している

## 検証コマンド
```bash
# ファイルの存在確認
ls -la proto/mcp/unity/v1/common.proto

# 構文チェック（protoc がインストール済みの場合）
protoc --proto_path=proto --include_imports --descriptor_set_out=/dev/null proto/mcp/unity/v1/common.proto
```

## 次のタスク
- Task 5: EditorControl proto ファイルの作成

## メモ
- このファイルは他の全ての proto ファイルの基盤となる
- L0 ポリシーにより googleapis への依存を回避
- スキーマフリーズまで破壊的変更が許可されている