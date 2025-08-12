# Task 7: Events proto ファイルの作成

## 目的
Unity Editor からのリアルタイム通知を処理するための gRPC サーバーストリーミング機能を定義する `events.proto` ファイルを作成する。

## 依存関係
- Task 4: 共通 proto ファイルの作成（common.proto の OperationRef）

## 要件
- gRPC サーバーストリーミング（Unity Bridge → Rust Client）
- 操作の進捗通知システム
- L0 ポリシーに準拠したシンプルなペイロード設計

## 実行手順

### `proto/mcp/unity/v1/events.proto` の作成
以下の内容で **正確に** ファイルを作成する：

```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
syntax = "proto3";
package mcp.unity.v1;
import "mcp/unity/v1/common.proto";

service Events {
  // Server streaming example (bridge -> client). The Rust side will be a client receiving the stream.
  rpc SubscribeOperation(OperationRef) returns (stream OperationEvent);
}

message OperationEvent {
  string id = 1;     // operation id
  string kind = 2;   // e.g., "progress", "completed", "error"
  string payload = 3;// free-form JSON string at L0
}
```

## サービス定義の詳細

### Events サービス
Unity Editor からのリアルタイム通知を配信

#### SubscribeOperation RPC（サーバーストリーミング）
```proto
rpc SubscribeOperation(OperationRef) returns (stream OperationEvent);
```
- **パターン**: サーバーストリーミング（1 リクエスト → N レスポンス）
- **方向**: Unity Bridge（サーバー） → Rust MCP Server（クライアント）
- **目的**: 特定の操作に関するリアルタイム更新の受信
- **入力**: `OperationRef` - 監視したい操作の ID
- **出力**: `stream OperationEvent` - 連続的なイベントストリーム

## メッセージ定義の詳細

### OperationEvent
操作に関するイベントを表現

```proto
message OperationEvent {
  string id = 1;     // operation id
  string kind = 2;   // e.g., "progress", "completed", "error"
  string payload = 3;// free-form JSON string at L0
}
```

#### フィールド説明
- **id**: 操作識別子（Task 6 の Assets/Build サービスから返される op_id と対応）
- **kind**: イベントの種類
  - \"progress\" - 進捗更新
  - \"completed\" - 操作完了
  - \"error\" - エラー発生
  - \"cancelled\" - キャンセル済み
- **payload**: 自由形式の JSON 文字列（L0 ポリシー準拠）

### payload の例
```json
// progress イベント
{\"percentage\": 45, \"current_step\": \"Compiling scripts\"}

// completed イベント
{\"result\": \"success\", \"output_path\": \"/path/to/build/app.apk\"}

// error イベント
{\"error_code\": \"COMPILE_ERROR\", \"message\": \"Script compilation failed\"}
```

## 使用パターン

### 典型的なフロー
1. **Assets/Build サービス**: 非同期操作を開始 → `op_id` を返す
2. **Events サービス**: `SubscribeOperation(op_id)` でストリーム開始
3. **Unity Bridge**: 操作の進捗に応じて `OperationEvent` を送信
4. **Rust Client**: ストリームを受信して進捗表示やログ出力

### Rust 側での受信例（概念的）
```rust
// Task 9 で実装される内容の予告
let mut stream = events_client.subscribe_operation(operation_ref).await?;
while let Some(event) = stream.message().await? {
    match event.kind.as_str() {
        "progress" => println!("Progress: {}", event.payload),
        "completed" => println!("Completed: {}", event.payload),
        "error" => eprintln!("Error: {}", event.payload),
        _ => {}
    }
}
```

## 受入基準
1. ファイルが正確なパスに存在する：`proto/mcp/unity/v1/events.proto`
2. common.proto が正しく import されている
3. パッケージ名が `mcp.unity.v1` で統一されている
4. SubscribeOperation がサーバーストリーミング RPC として定義されている
5. OperationEvent メッセージが3つのフィールドを持つ
6. 未解決のインポートが存在しない

## 検証コマンド
```bash
# ファイルの存在確認
ls -la proto/mcp/unity/v1/events.proto

# import とストリーミングの確認
grep -n "import.*common.proto" proto/mcp/unity/v1/events.proto
grep -n "stream.*OperationEvent" proto/mcp/unity/v1/events.proto

# 構文チェック（protoc がインストール済みの場合）
protoc --proto_path=proto --include_imports --descriptor_set_out=/dev/null proto/mcp/unity/v1/events.proto
```

## 次のタスク
- Task 8: コード生成設定（build.rs の作成）

## メモ
- L0 ポリシーにより構造化された型ではなく JSON 文字列を使用
- サーバーストリーミングにより効率的なリアルタイム通知を実現
- Rust 側は gRPC **クライアント**として Unity Bridge からストリームを受信