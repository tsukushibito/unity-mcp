# Unity MCP Server - プロジェクト概要 (2025年8月更新)

## プロジェクト目的
Unity MCP Server は、Rust MCP サーバーと Unity Editor ブリッジを gRPC で接続した双方向システムです。rmcp SDK を使用し、プロトコルバッファーによる型安全な通信を実現しています。

## 現在の実装状況

### 完成済みコンポーネント

**Rust Server側（`server/`）:**
- ✅ rmcp SDK ベースの MCP サーバー基盤実装済み
- ✅ gRPC クライアント設定（`GrpcConfig`）と接続管理（`ChannelManager`）
- ✅ プロトコルバッファ定義（6つのprotoファイル）とコード生成
- ✅ 統合テスト（`smoke.rs`）でgRPCラウンドトリップ接続テスト
- ✅ トレーシング・ログ・観測性設定
- ✅ `McpService` 実装（`unity_health` ツール含む）

**プロトコルバッファ（`proto/mcp/unity/v1/`）:**
- ✅ `common.proto` - 共通データ型定義
- ✅ `editor_control.proto` - Editor制御（PlayMode等）
- ✅ `assets.proto` - アセット管理とパス操作
- ✅ `build.proto` - ビルドパイプライン制御
- ✅ `operations.proto` - 非同期操作管理
- ✅ `events.proto` - イベントストリーミング

**Unity Bridge側（`bridge/`）:**
- ✅ UPMパッケージ構造（`com.example.mcp-bridge`）
- ✅ C# gRPCライブラリ（Google.Protobuf, Grpc.Net.Client等）
- ✅ プロトコルバッファからのC#コード生成済み
- ✅ gRPCクライアント実行時ライブラリ統合

**開発基盤:**
- ✅ CI/CD設定（Ubuntu/macOS マトリックステスト）
- ✅ build.rs による自動コード生成設定
- ✅ `server-stubs` フィーチャーフラグでテスト対応

### アーキテクチャ特徴
- **Single Repository**: 高速フィードバックループを実現
- **Multi-Transport**: stdio/WebSocket 対応（rmcp SDK）
- **Type-Safe Communication**: プロトコルバッファによる型安全性
- **Token-Based Auth**: gRPC接続でのトークン認証
- **Async Operations**: 非同期操作と進捗ストリーミング対応

## 技術スタック

### Rust Server
- **rmcp 0.5.0** - MCP プロトコル実装（server, transport-io features）
- **tonic 0.14.1** - gRPC クライアント（transport, tls-webpki-roots）
- **tokio 1.47.1** - 非同期ランタイム（multi-thread）
- **anyhow 1.0.99** - エラーハンドリング
- **tracing 0.1.41** - 構造化ログ

### Unity Bridge (C#)
- **Unity 2022.3 LTS+** 対応
- **Grpc.Net.Client** - HTTP/2 gRPCクライアント
- **Google.Protobuf** - プロトコルバッファランタイム

## プロジェクト構造
```
unity-mcp/
├── server/                    # Rust MCP サーバー
│   ├── src/
│   │   ├── main.rs           # エントリーポイント（McpService.serve_stdio）
│   │   ├── mcp/              # MCP関連実装
│   │   │   ├── service.rs    # McpService（ServerHandler実装）
│   │   │   └── tools/        # MCP ツール（health.rs等）
│   │   ├── grpc/             # gRPC クライアント
│   │   │   ├── config.rs     # GrpcConfig設定
│   │   │   └── channel.rs    # ChannelManager接続管理
│   │   └── generated/        # プロトコルバッファ生成コード
│   ├── tests/                # 統合テスト
│   │   └── smoke.rs          # gRPC接続テスト
│   └── build.rs              # tonic-prost-build設定
├── bridge/                   # Unity プロジェクト
│   └── Packages/com.example.mcp-bridge/
│       ├── Editor/
│       │   ├── Generated/Proto/  # C# gRPCコード
│       │   └── Plugins/Grpc/     # gRPCライブラリ
│       └── package.json      # UPMパッケージ定義
├── proto/mcp/unity/v1/       # プロトコルバッファ定義
├── tasks/                    # タスク管理とプロジェクト計画
└── docs/                     # 設計ドキュメント
```

## 次期実装計画
現在のプロジェクトステータス（`tasks/project_status_and_next_tasks.md`）に基づく優先順位：

1. **Unity Bridge gRPCサーバー実装** - C#側のEditorControlサービス
2. **MCP Tools拡張** - Asset管理、Build制御ツール
3. **Operation管理** - 非同期操作の状態管理とストリーミング
4. **PlayMode制御** - 実際のUnity Editor操作統合

## 開発環境
- **作業ディレクトリ**: `/workspaces/unity-mcp` (ルート), `/workspaces/unity-mcp/server` (Rust)
- **必須ツール**: protoc 3.21.12, Rust, Unity 2022.3+
- **開発環境**: VS Code + Dev Container（推奨）