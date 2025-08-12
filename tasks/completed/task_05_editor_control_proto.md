# Task 5: EditorControl proto ファイルの作成

## 目的
Unity Editor の基本制御機能を定義する `editor_control.proto` ファイルを作成し、ヘルスチェックと Play Mode 制御を実装する。

## 依存関係
- Task 4: 共通 proto ファイルの作成（common.proto の存在）

## 要件
- Unity Editor の基本的なライフサイクル制御
- ヘルスチェック機能
- Play Mode の取得と設定
- common.proto の適切な import

## 実行手順

### `proto/mcp/unity/v1/editor_control.proto` の作成
以下の内容で **正確に** ファイルを作成する：

```proto
// STATUS: PROVISIONAL — Breaking changes allowed until Schema Freeze.
// PACKAGE: mcp.unity.v1
syntax = "proto3";
package mcp.unity.v1;
import "mcp/unity/v1/common.proto";

service EditorControl {
  rpc Health(HealthRequest) returns (HealthResponse);
  rpc GetPlayMode(Empty) returns (GetPlayModeResponse);
  rpc SetPlayMode(SetPlayModeRequest) returns (SetPlayModeResponse);
}

message HealthRequest {}
message HealthResponse { string status = 1; /* e.g., "OK" */ }

message GetPlayModeResponse { bool is_playing = 1; }
message SetPlayModeRequest { bool play = 1; }
message SetPlayModeResponse { bool applied = 1; }
```

## サービス定義の説明

### EditorControl サービス
Unity Editor の基本制御を行うメインサービス

#### Health RPC
```proto
rpc Health(HealthRequest) returns (HealthResponse);
```
- **目的**: Unity Editor の稼働状態確認
- **リクエスト**: 空のメッセージ
- **レスポンス**: status フィールドで状態を返す（例：\"OK\"）

#### GetPlayMode RPC
```proto
rpc GetPlayMode(Empty) returns (GetPlayModeResponse);
```
- **目的**: 現在の Play Mode 状態を取得
- **リクエスト**: Empty（common.proto から）
- **レスポンス**: is_playing で Play/Edit Mode を示す

#### SetPlayMode RPC
```proto
rpc SetPlayMode(SetPlayModeRequest) returns (SetPlayModeResponse);
```
- **目的**: Play Mode の開始/停止を制御
- **リクエスト**: play フィールドで true（開始）/false（停止）を指定
- **レスポンス**: applied で操作の成功/失敗を返す

## メッセージ定義の詳細

### ヘルスチェック関連
- `HealthRequest`: パラメーターなし（将来の拡張に備えて空メッセージ）
- `HealthResponse`: 文字列ステータス（\"OK\", \"ERROR\" など）

### Play Mode 制御関連
- `GetPlayModeResponse`: boolean で現在の状態
- `SetPlayModeRequest`: boolean で設定したい状態
- `SetPlayModeResponse`: boolean で操作結果

## 受入基準
1. ファイルが正確なパスに存在する：`proto/mcp/unity/v1/editor_control.proto`
2. common.proto が正しく import されている
3. パッケージ名が `mcp.unity.v1` で統一されている
4. 3つの RPC メソッドがすべて定義されている
5. 未解決のインポートが存在しない

## 検証コマンド
```bash
# ファイルの存在確認
ls -la proto/mcp/unity/v1/editor_control.proto

# import の確認
grep -n "import.*common.proto" proto/mcp/unity/v1/editor_control.proto

# 構文チェック（protoc がインストール済みの場合）
protoc --proto_path=proto --include_imports --descriptor_set_out=/dev/null proto/mcp/unity/v1/editor_control.proto
```

## 次のタスク
- Task 6: Services proto ファイルの作成（Assets, Build, Operations）

## メモ
- このサービスは Unity MCP Server の中核機能
- 最小限の実装でありながら、実際の Unity Editor 制御に必要な基本機能を含む
- Rust 側では gRPC クライアントとして実装される