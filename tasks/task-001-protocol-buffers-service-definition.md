# タスク001: Unity-MCP gRPC通信用Protocol Buffersサービス定義の作成

## 概要

Rust MCP サーバーと Unity Editor ブリッジ間の契約として機能する Protocol Buffers (.proto) サービス定義を作成します。これは gRPC サービスインターフェース、メッセージ型、MCP操作とUnity固有機能の通信パターンを定義します。

## 受け入れ基準

- [ ] サービス定義を含む `proto/unity_mcp.proto` ファイルを作成
- [ ] 基本的なMCP操作（ツール、リソース、プロンプト）を持つUnityMcpServiceを定義
- [ ] Unity固有の操作（プロジェクト情報、アセット操作、シーン管理）を定義
- [ ] リアルタイム通信のための双方向ストリーミングサポートを含める
- [ ] リクエスト/レスポンスの包括的なメッセージ型を定義
- [ ] 適切なproto3構文とimport文を追加
- [ ] protoファイルがエラーなしでコンパイルされることを検証

## 実装ノート

**サービス構造:**
- `UnityMcpService` - メインgRPCサービス
- MCPコア操作のメソッド（list_tools、call_tool、list_resourcesなど）
- Unity固有操作のメソッド（GetProjectInfo、ListAssetsなど）
- リアルタイム更新と双方向通信のためのストリーミングRPC

**メッセージ型:**
- 各RPCメソッドのリクエスト/レスポンスペア
- 共通型: McpTool、McpResource、UnityAsset、ProjectInfo
- エラーハンドリング型: McpError、UnityError
- ストリーミングメッセージラッパー

**技術的考慮事項:**
- 最大互換性のためproto3構文を使用
- gRPC命名規則に従う（サービス/メッセージはPascalCase、フィールドはsnake_case）
- 適切なフィールド番号とoptional/repeatedマーカーを含める
- メッセージ設計で将来の拡張性を考慮

## 作成するファイル

- `proto/unity_mcp.proto` - メインProtocol Bufferサービス定義
- `proto/build.rs` - Protocol Bufferコンパイル用ビルドスクリプト（将来のタスク用に準備）

## テスト要件

- protobufコンパイラを使用してprotoファイル構文を検証
- 全メッセージ型が適切に定義されていることを確認
- サービスメソッドがコアMCPおよびUnity操作をカバーしていることを検証
- RustとUnityコンテキスト両方でのprotoコンパイルをテスト（後続のタスクで）

## 依存関係

なし - これはgRPC実装の基盤タスクです。

## ブロック対象

- タスク002: Rust gRPCサーバー依存関係のセットアップ
- タスク004: Unity gRPCクライアント依存関係のセットアップ
- 後続のすべてのgRPC実装タスクはこのプロトコル定義に依存