# Unity MCP Server - アーキテクチャ詳細

## システム全体構成

### 通信フロー
```
MCP Client → [stdio] → Rust MCP Server → [gRPC] → Unity Editor Bridge
     ↑                       ↓                              ↓
  JSON-RPC               Channel Manager              gRPC Services
  Protocol                 (Token Auth)               (C# Implementation)
```

## Rust MCP Server アーキテクチャ

### 主要コンポーネント

**1. McpService (`src/mcp/service.rs`)**
- `rmcp::ServerHandler` 実装
- stdio transport での MCP プロトコル処理
- Unity固有のMCPツール提供

**2. ChannelManager (`src/grpc/channel.rs`)**  
- gRPC接続管理とコネクションプール
- トークンベース認証処理
- 接続再試行ロジック

**3. GrpcConfig (`src/grpc/config.rs`)**
- 環境変数からのgRPC設定読み込み
- エンドポイント、認証トークン管理

### MCP Tools実装
**現在実装済み:**
- `unity_health` - Unity Bridge接続確認

**実装予定:**
- `unity_assets` - アセット管理（パス⇄GUID変換）
- `unity_build` - ビルドパイプライン制御
- `unity_playmode` - PlayMode制御

### 生成コード管理
**build.rs設定:**
```rust
tonic_prost_build::Builder::new()
    .server(cfg!(feature = "server-stubs"))  // テスト用のみ
    .client(true)  // 常に生成
    .compile(&proto_files, &["proto"]);
```

**フィーチャーフラグ:**
- `server-stubs`: gRPCサーバースタブ生成（統合テスト用）
- デフォルト: クライアントコードのみ生成

## Unity Bridge アーキテクチャ

### UPMパッケージ構造
```
Packages/com.example.mcp-bridge/
├── Editor/
│   ├── Generated/Proto/      # gRPCコード生成先
│   └── Plugins/Grpc/         # gRPCランタイムライブラリ
├── Runtime/                  # 将来のPlayMode用
└── package.json             # UPMメタデータ
```

### gRPCライブラリ統合
**含まれるライブラリ:**
- `Google.Protobuf.dll` - プロトコルバッファランタイム
- `Grpc.Net.Client.dll` - HTTP/2 gRPCクライアント
- `Grpc.Net.Common.dll` - 共通ユーティリティ
- `Microsoft.Extensions.Logging.Abstractions.dll` - ログ抽象化

**メタファイル設定:**
- Editor Only: Unity Editor専用設定
- .NET Standard 2.1 対応

## gRPCプロトコル設計

### サービス構成
```protobuf
// 1. EditorControlService - Editor基本制御
service EditorControlService {
  rpc GetPlayMode(GetPlayModeRequest) returns (GetPlayModeResponse);
  rpc SetPlayMode(SetPlayModeRequest) returns (SetPlayModeResponse);
  rpc GetHealth(GetHealthRequest) returns (GetHealthResponse);
}

// 2. AssetsService - アセット管理
service AssetsService {
  rpc PathToGuid(PathToGuidRequest) returns (PathToGuidResponse);
  rpc GuidToPath(GuidToPathRequest) returns (GuidToPathResponse);
  rpc RefreshAssets(RefreshAssetsRequest) returns (Operation);
}

// 3. BuildService - ビルド制御
service BuildService {
  rpc BuildPlayer(BuildPlayerRequest) returns (Operation);
  rpc GetBuildSettings(GetBuildSettingsRequest) returns (GetBuildSettingsResponse);
}
```

### 非同期操作パターン
```protobuf
message Operation {
  string id = 1;           // 一意識別子
  OperationStatus status = 2;  // 進行状況
  google.protobuf.Any metadata = 3;  // 操作固有データ
  repeated LogEntry logs = 4;  // 進捗ログ
}

enum OperationStatus {
  PENDING = 0;
  RUNNING = 1;
  COMPLETED = 2;
  FAILED = 3;
}
```

## データフロー設計

### 同期操作フロー
```
MCP Tool → gRPC Request → Unity API → gRPC Response → MCP Response
```

### 非同期操作フロー  
```
MCP Tool → gRPC Request → Operation ID 返却
    ↓
MCP Tool → Operation進捗確認 → Status + Logs
    ↓
完了まで繰り返し → 最終結果
```

### エラーハンドリング
**Rust側:**
```rust
match channel_manager.call_service().await {
    Ok(response) => Ok(mcp_response),
    Err(grpc_err) => Err(anyhow!("Unity Bridge error: {}", grpc_err)),
}
```

**Unity側:**
```csharp
try {
    var response = await service.GetHealthAsync(request);
    return response;
} catch (RpcException ex) {
    Debug.LogError($"gRPC call failed: {ex.Status}");
    throw;
}
```

## セキュリティ・認証

### トークンベース認証
- 環境変数: `MCP_BRIDGE_TOKEN`
- gRPCメタデータ経由でトークン送信
- Unity Bridge側でトークン検証

### 接続セキュリティ
- デフォルト: localhost接続のみ
- TLS設定対応（本番環境用）
- ファイアウォール設定は不要（内部通信）

## テスト戦略

### Rust統合テスト (`tests/smoke.rs`)
```rust
// gRPCラウンドトリップテスト
#[tokio::test]
async fn test_grpc_health_roundtrip() {
    let service = setup_test_service().await;
    let response = service.unity_health().await.unwrap();
    assert_eq!(response.status, "healthy");
}
```

### Unity EditModeテスト
- Unity Test Runner使用
- gRPCサービス実装のユニットテスト
- Editor APIとの統合テスト

## パフォーマンス考慮

### 接続管理
- `ChannelManager`による接続プール
- 再利用可能なgRPCチャネル
- 接続タイムアウト・再試行設定

### メモリ管理
- プロトコルバッファのゼロコピー最適化
- 非同期ストリームによる大容量データ処理
- Unity GC圧迫回避

## 拡張計画

### 短期拡張
1. Unity Bridge gRPCサーバー実装
2. AssetsService実装（パス⇄GUID変換）
3. ビルドパイプライン統合

### 中長期拡張
1. Runtime統合（PlayMode時のMCP操作）
2. Unity Package Manager API統合
3. カスタムEditor Window統合
4. プロファイラー・メトリクス収集