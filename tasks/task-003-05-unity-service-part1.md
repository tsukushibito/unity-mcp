# Task 3.5: Unity操作サービススタブ実装（前半）

## 説明

Unity固有機能の前半3つのRPCメソッド（GetProjectInfo、ImportAsset、MoveAsset）のスタブ実装を行います。MCPコアサービスで確立されたパターンを踏襲してUnity特化機能を実装します。

## 受け入れ基準

- [ ] `GetProjectInfo`メソッドのスタブ実装（ダミープロジェクト情報）
- [ ] `ImportAsset`メソッドのスタブ実装（基本的な成功レスポンス）
- [ ] `MoveAsset`メソッドのスタブ実装（基本的な成功レスポンス）
- [ ] 各メソッドで構造化ログ出力
- [ ] Unity固有のエラーケース処理
- [ ] リクエストパラメータの基本検証

## 実装内容

**実装メソッド:**
1. **GetProjectInfo**
   - ダミーの`ProjectInfo`を返す
   - プロジェクト名: "Unity MCP Test Project"
   - Unityバージョン: "2023.3.0f1"（例）

2. **ImportAsset**
   - asset_pathパラメータの基本検証（空文字列チェック等）
   - ダミーの`UnityAsset`情報を生成
   - GUID生成（簡易的なUUID）
   - 成功時のログ出力

3. **MoveAsset**
   - src_path/dst_pathパラメータの検証
   - パスの重複チェック
   - ダミーの移動成功レスポンス
   - 適切なエラーメッセージ

**共通実装:**
- MCPコアサービスと統一されたログパターン
- Unity固有のエラータイプ考慮
- 将来のUnity Editor統合への準備

## 技術的考慮事項

- Unity Asset GUIDの形式（32文字の16進数文字列）
- Unity Asset Pathの形式（"Assets/"で始まる）
- Unity固有のエラーケース（重複、無効パス等）
- 既存のコードパターンとの一貫性

## 依存関係

- **前提条件:** Task 3.4完了（MCPコア完成により実装パターン確立）

## ブロック対象

- Task 3.6: Unity操作サービススタブ実装（後半）

## 検証方法

- Unity特化のレスポンス構造が正しい
- Asset情報の形式が適切（GUID、パス等）
- エラーケースでのUnity固有メッセージ
- ログ出力でUnity操作であることが識別可能

## 実装優先度

**中優先度** - Unity統合の準備段階として重要