# Task 6: Services proto ファイルの作成

## 目的
Unity Editor の主要機能（Assets、Build、Operations）を定義する3つの proto ファイルを作成する。

## 依存関係
- Task 4: 共通 proto ファイルの作成（common.proto の存在）

## 要件
- アセット管理機能（Assets サービス）
- ビルド制御機能（Build サービス）
- 操作管理機能（Operations サービス）
- 非同期操作のサポート

## 実行手順

### 1. `proto/mcp/unity/v1/assets.proto` の作成
```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
syntax = "proto3";
package mcp.unity.v1;

service Assets {
  rpc ImportAsset(ImportAssetRequest) returns (ImportAssetResponse);
}

message ImportAssetRequest { string path = 1; }
message ImportAssetResponse { bool queued = 1; string op_id = 2; }
```

### 2. `proto/mcp/unity/v1/build.proto` の作成
```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
syntax = "proto3";
package mcp.unity.v1;

service Build {
  rpc BuildPlayer(BuildPlayerRequest) returns (BuildPlayerResponse);
}

message BuildPlayerRequest { string target = 1; /* e.g., "Android" */ }
message BuildPlayerResponse { bool started = 1; string op_id = 2; }
```

### 3. `proto/mcp/unity/v1/operations.proto` の作成
```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
syntax = "proto3";
package mcp.unity.v1;
import "mcp/unity/v1/common.proto";

service Operations {
  rpc GetOperation(OperationGetRequest) returns (OperationGetResponse);
  rpc CancelOperation(OperationCancelRequest) returns (OperationCancelResponse);
}

message OperationGetRequest { string id = 1; }
message OperationGetResponse { string id = 1; string state = 2; string message = 3; }

message OperationCancelRequest { string id = 1; }
message OperationCancelResponse { bool accepted = 1; }
```

## サービス定義の詳細

### Assets サービス
Unity のアセット管理機能を提供

#### ImportAsset RPC
- **目的**: アセットのインポートを非同期で開始
- **入力**: ファイルパス
- **出力**: キューに追加されたかの確認と操作ID

### Build サービス
Unity のビルド機能を提供

#### BuildPlayer RPC
- **目的**: プレイヤービルドを非同期で開始
- **入力**: ターゲットプラットフォーム（\"Android\", \"iOS\" など）
- **出力**: ビルド開始の確認と操作ID

### Operations サービス
非同期操作の管理を提供

#### GetOperation RPC
- **目的**: 操作の現在の状態を取得
- **入力**: 操作ID
- **出力**: 操作ID、状態、メッセージ

#### CancelOperation RPC
- **目的**: 実行中の操作をキャンセル
- **入力**: 操作ID
- **出力**: キャンセル要求が受け入れられたかの確認

## 設計パターン

### 非同期操作パターン
1. Assets/Build サービスで操作を開始 → `op_id` を返す
2. Operations サービスで進捗を監視
3. 必要に応じて Operations サービスでキャンセル

### 操作状態の例
- \"queued\" - 待機中
- \"running\" - 実行中
- \"completed\" - 完了
- \"failed\" - 失敗
- \"cancelled\" - キャンセル済み

## 受入基準
1. 3つのファイルすべてが正確なパスに存在する：
   - `proto/mcp/unity/v1/assets.proto`
   - `proto/mcp/unity/v1/build.proto`
   - `proto/mcp/unity/v1/operations.proto`
2. operations.proto が common.proto を正しく import している
3. パッケージ名がすべて `mcp.unity.v1` で統一されている
4. すべてのサービスとメッセージが定義されている
5. 未解決のインポートが存在しない

## 検証コマンド
```bash
# ファイルの存在確認
ls -la proto/mcp/unity/v1/{assets,build,operations}.proto

# import の確認
grep -n "import.*common.proto" proto/mcp/unity/v1/operations.proto

# 構文チェック（protoc がインストール済みの場合）
protoc --proto_path=proto --include_imports --descriptor_set_out=/dev/null \
  proto/mcp/unity/v1/assets.proto \
  proto/mcp/unity/v1/build.proto \
  proto/mcp/unity/v1/operations.proto
```

## 次のタスク
- Task 7: Events proto ファイルの作成（ストリーミング対応）

## メモ
- 非同期操作パターンにより、長時間のタスク（ビルド、アセットインポート）を適切に処理
- Operations サービスは他のサービスの操作監視に使用される
- L0 ポリシーにより、google.rpc.* を使わずシンプルな文字列フィールドで状態管理