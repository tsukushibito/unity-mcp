# タスク 005: Unity gRPC クライアントラッパーと接続管理の実装

## 説明

RustサーバーへのgRPC接続を管理し、接続ライフサイクルを処理し、Unity Editorツール向けに使いやすいAPIを提供する高レベルC#クライアントラッパーを作成します。接続管理、エラーハンドリング、UnityのAsyncパターンとの統合が含まれます。

## 受入基準

- [ ] RuntimeアセンブリにおけるMcpGrpcClientクラスの作成
- [ ] 接続確立と管理の実装
- [ ] UnityのAsync支援を使った適切なasync/awaitパターンの追加
- [ ] 接続再試行とエラーハンドリングロジックの作成
- [ ] 優雅な切断とクリーンアップの実装
- [ ] UnityのDebugシステムとのログ統合の追加
- [ ] クライアントライフサイクル管理の作成（シングルトンまたはサービスパターン）
- [ ] Rust gRPCサーバーへの接続動作の確認
- [ ] 接続の復旧性とエラーシナリオのテスト

## 実装ノート

**クライアントアーキテクチャ:**
```csharp
namespace MCP.Bridge.Runtime
{
    public class McpGrpcClient : IDisposable
    {
        // 接続管理
        public async Task<bool> ConnectAsync(string address, int port);
        public async Task DisconnectAsync();
        public bool IsConnected { get; }
        
        // MCP操作
        public async Task<List<McpTool>> ListToolsAsync();
        public async Task<McpResult> CallToolAsync(string name, object args);
        // ... その他のMCPメソッド
        
        // Unity固有の操作
        public async Task<ProjectInfo> GetProjectInfoAsync();
        // ... その他のUnityメソッド
    }
}
```

**技術的考慮事項:**
- ライブラリコードでのConfigureAwait(false)の使用
- 適切なキャンセレーショントークンサポートの実装
- 特定の操作におけるUnityのメインスレッド要件への対応
- gRPCエラー用の適切な例外タイプの作成
- 複数の同時接続が必要な場合のコネクションプールの検討
- スレッドセーフな接続状態管理

**エラーハンドリング:**
- gRPC例外をドメイン固有の例外でラップ
- 一時的なネットワークエラーの再試行ロジック
- 永続的な障害に対するサーキットブレーカーパターン
- 接続問題のデバッグ用ログ出力

## 作成/変更するファイル

- `bridge/Packages/com.example.mcp-bridge/Runtime/McpGrpcClient.cs` - メインクライアントラッパー
- `bridge/Packages/com.example.mcp-bridge/Runtime/Exceptions/` - カスタム例外タイプ
- `bridge/Packages/com.example.mcp-bridge/Runtime/Models/` - レスポンス/リクエストモデル
- `bridge/Packages/com.example.mcp-bridge/Runtime/Configuration/McpClientConfig.cs` - クライアント設定

## テスト要件

- クライアントが実行中のRustサーバーに正常に接続すること
- サーバーが利用できない場合の適切なエラーハンドリング
- 接続再試行ロジックが正常に動作すること
- 非同期操作がUnity Editorをブロックしないこと
- メモリリークの回避（適切な破棄処理）
- 複数の接続/切断サイクルが正常に動作すること
- キャンセレーショントークンが適切に処理されること
- Unity Debug.Log統合が動作すること

## 依存関係

- **必要:** Task 001（Protocol Buffersサービス定義）
- **必要:** Task 004（Unity gRPCクライアント依存関係）
- **統合すべき:** Task 003（Rust gRPCサーバースタブ）

## ブロック

- Task 008: 設定管理の追加
- Task 009: 統合テストの作成
- Task 010: エラーハンドリングと再接続ロジックの追加

## 実装優先度

**高優先度** - MCP Bridge操作に必要なコアUnityクライアント機能。

## Unity固有の考慮事項

**スレッド処理:**
- 特定のAPI呼び出しにおけるUnityのメインスレッド要件
- UnityのAsync/Await支援の使用
- EditorとRuntimeコンテキストの適切な処理

**ライフサイクル管理:**
- Unityライフサイクル（ドメインリロード、プレイモード変更）との統合
- Editor終了時の適切なクリーンアップ
- 必要に応じたUnityのシリアライゼーション要件への対応