# Unity MCP Server - プロジェクト状況と次期タスク

## プロジェクトの現在の状態

### 完成済みのコンポーネント

**Rust Server側（`server/`）:**
- ✅ 基本的な gRPC クライアント設定（`GrpcConfig`）と接続管理（`ChannelManager`）
- ✅ プロトコルバッファ定義（6つのprotoファイル）完成
- ✅ ビルドスクリプト（`build.rs`）でgRPCコード生成設定済み
- ✅ 統合テスト（`smoke.rs`）でgRPCラウンドトリップ接続テスト実装
- ✅ トレーシング・ログ設定

**プロトコルバッファ（`proto/`）:**
- ✅ 6つのサービス定義完成：`common`, `editor_control`, `assets`, `build`, `operations`, `events`
- ✅ gRPCサービス仕様が設計書通り実装済み

**Unity Bridge側（`bridge/`）:**
- ✅ ディレクトリ構造のみ存在（`.keep`ファイル）
- ✅ UPMパッケージ定義（`package.json`）のみ

### 未実装部分（重要度順）

**最も重要な欠落：**
1. **MCP Server本体** - rmcp SDKを使用した実際のMCPサーバー実装
2. **Unity Bridge** - C# gRPCサーバー実装
3. **MCP Tools** - Unity操作のためのMCPツール定義
4. **Operation管理** - 非同期操作の状態管理とストリーミング

## 次に進めるべき優先タスク

設計書のMVP実装順序（Health + Logs → Path⇄GUID → Import/Refresh → BuildPlayer → PlayMode）に基づく実装計画：

### フェーズ1: MCPサーバー基盤（最優先）
1. **rmcp SDK依存関係追加** - Cargo.tomlにrmcp クレート追加
2. **MCP Server実装** - `server/src/mcp/` モジュール作成
3. **基本MCPツール定義** - Health check用の最小限ツール
4. **gRPCクライアント統合** - 既存のChannelManagerとMCPサーバーの連携

### フェーズ2: Unity Bridge基盤
1. **Unity C# gRPCサーバー** - `bridge/Assets/MCP/Editor/BridgeServer.cs`
2. **EditorControlサービス実装** - Health, GetPlayMode, SetPlayMode
3. **基本的なOperation管理** - 同期的なレスポンス処理

### フェーズ3: 最小ビルド可能な状態
1. **Health check** - MCP → Rust → Unity → レスポンスのE2Eテスト
2. **PlayMode制御** - 実際のUnityエディター操作
3. **統合テスト拡張** - 実際のUnity Editorを使用したテスト

### 実装の順序理由

この順序により、各段階で動作確認しながら段階的に機能を追加できます。特に：

- **フェーズ1**：gRPCインフラは整っているため、MCPサーバー部分の実装が最優先
- **フェーズ2**：Unity側のgRPCサーバー実装でE2E接続を確立
- **フェーズ3**：実際の動作確認により、アーキテクチャの妥当性を検証

## 技術的考慮事項

- `server-stubs`フィーチャーフラグがテストに必要
- プロトコルバッファ変更時は`cargo clean`が必要
- CI/CDでprotoc 3.21.12が必要
- 全コマンドは`server/`ディレクトリから実行

---

*生成日時: 2025-08-13*
*参照: docs/unity_mcp_server_architecture_overview_rmcp_g_rpc_bridge.md*