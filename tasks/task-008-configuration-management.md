# Task 008: gRPCエンドポイントと設定のための設定管理の追加

## 説明

サーバーエンドポイント、クライアント接続設定、セキュリティオプション、およびランタイム設定を含む、gRPC通信システムの包括的な設定管理を実装します。これは環境変数による上書きと検証機能を備えた、RustサーバーとUnityクライアントの両方の設定をサポートします。

## 受け入れ基準

- [ ] gRPC設定のためのRustサーバー設定を拡張
- [ ] Unityクライアント設定管理を追加
- [ ] 設定検証とエラーハンドリングを実装
- [ ] 環境変数による上書きをサポート
- [ ] 該当する場合はランタイム設定の更新を追加
- [ ] 設定ドキュメントと例を作成
- [ ] 設定の読み込みと検証をテスト
- [ ] 設定変更が動作に正しく影響することを確認

## 実装メモ

**Rustサーバー設定:**
```toml
# server/config/default.toml
[grpc]
enabled = true
host = "127.0.0.1"
port = 50051
max_connections = 100
request_timeout_ms = 30000
keep_alive_interval_ms = 60000
compression = "gzip"

[grpc.tls]
enabled = false
cert_path = ""
key_path = ""
```

**Unityクライアント設定:**
```csharp
[Serializable]
public class McpClientConfig
{
    public string ServerHost = "127.0.0.1";
    public int ServerPort = 50051;
    public int ConnectionTimeoutMs = 5000;
    public int RequestTimeoutMs = 30000;
    public bool EnableRetry = true;
    public int MaxRetryAttempts = 3;
    public bool EnableCompression = true;
}
```

**設定ソース:**
1. デフォルト設定ファイル
2. 環境変数 (MCP_GRPC_HOST, MCP_GRPC_PORT など)
3. コマンドライン引数 (Rustサーバー用)
4. Unity Editorの設定 (Unityクライアント用)
5. ランタイム設定API

**技術的考慮事項:**
- Rust設定のデシリアライズ用のSerde統合
- UnityのScriptableObjectまたはJSONベースの設定
- 設定のホットリロード機能
- 検証ルールとエラー報告
- TLS/認証設定のセキュリティ考慮事項

## 作成/修正するファイル

- `server/config/default.toml` - gRPC設定を拡張
- `server/src/config/mod.rs` - 設定管理モジュール
- `server/src/config/grpc.rs` - gRPC固有の設定
- `bridge/Runtime/Configuration/McpClientConfig.cs` - Unityクライアント設定
- `bridge/Editor/Configuration/McpConfigEditor.cs` - Unity設定UI
- 設定検証と読み込みユーティリティ

## テスト要件

- 設定ファイルが正しく解析されること
- 環境変数による上書きが動作すること
- 不正な設定が明確なエラーメッセージを生成すること
- 設定変更がサーバー/クライアントの動作に影響すること
- デフォルト値が適切で文書化されていること
- 設定検証が一般的なエラーを捉えること
- 実装されている場合はホットリロードが動作すること
- Unity Editorの設定UIが適切に機能すること

## 依存関係

- **必要:** Task 002 (Rust gRPCサーバーの依存関係)
- **必要:** Task 004 (Unity gRPCクライアントの依存関係)
- **統合すべき:** Task 006 (Rust gRPCトランスポート統合)

## ブロック

- Task 009: 統合テストの作成
- Task 010: エラーハンドリングと再接続ロジックの追加

## 実装優先度

**中優先度** - プロダクション使用には重要だが、包括的な設定なしでも基本機能は動作する。

## 設定カテゴリ

**接続設定:**
- サーバーホストとポートの設定
- 接続タイムアウトとリトライ設定
- Keep-aliveとハートビート間隔
- コネクションプールパラメータ

**セキュリティ設定:**
- TLS/SSL設定
- 証明書管理
- 認証トークンまたはキー
- ネットワークセキュリティポリシー

**パフォーマンス設定:**
- メッセージ圧縮オプション
- 同時接続制限
- バッファサイズとメモリ制限
- リクエストタイムアウト設定

**開発設定:**
- デバッグログレベル
- 開発モード vs プロダクションモード
- モックサーバー設定
- テスト用オーバーライド

## Unity統合

**エディター設定:**
- ProjectSettings統合
- 設定用カスタムプロパティドローワー
- Unity Inspectorでの検証フィードバック
- 設定のエクスポート/インポート機能

**ランタイム設定:**
- ScriptableObjectベースの設定アセット
- ResourcesまたはAddressablesからの設定読み込み
- ランタイム設定オーバーライド機能
- 設定キャッシュと永続化

## 環境変数サポート

**標準環境変数:**
- `MCP_GRPC_HOST` - サーバーホストを上書き
- `MCP_GRPC_PORT` - サーバーポートを上書き
- `MCP_GRPC_TLS_ENABLED` - TLSを有効/無効化
- `MCP_DEBUG_LEVEL` - デバッグレベルを設定
- `MCP_CONFIG_PATH` - カスタム設定ファイルパス