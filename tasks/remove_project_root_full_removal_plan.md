# 作業計画: `project_root` の完全削除（プロトコル/実装から排除）

この計画は、既存の「任意化」案を破棄し、より単純な仕様へ振り切るものです。ハンドシェイクから `project_root` を完全に削除し、クライアントが Unity プロジェクトパスを知る必要をなくします。

## 目的 / 方針
- 目的: コードと設定を単純化し、MCP クライアントがパスを一切扱わなくても接続できるようにする。
- 方針:
  - Proto から `IpcHello.project_root` フィールドを削除（ブレイキング変更）。
  - Unity 側はカレントディレクトリ（Editor 実行時のプロジェクトルート）を唯一の基準とする。
  - Rust 側は `MCP_PROJECT_ROOT` を廃止。関連コード・ドキュメント・テストを削除。
  - セキュリティはトークン認証 + バージョン/スキーマ検証で担保。

## スコープ
- In
  - Proto（`proto/mcp/unity/v1/ipc_control.proto`）から `project_root` 削除
  - Rust/C# 生成コードの再生成と差分更新（スキーマハッシュ更新含む）
  - Unity 実装（EditorIpcServer）から `ValidateProjectRoot` 系の完全削除
  - Rust 実装から `MCP_PROJECT_ROOT`、正規化処理、ステータス出力の `project_root` を削除
  - 該当テスト/ドキュメント/サンプルの調整
- Out
  - PathPolicy のポリシー自体は変更しない（プロジェクトルートは Editor の CWD で一意）

## 変更詳細

### 1) プロトコル（Proto）
- 対象: `proto/mcp/unity/v1/ipc_control.proto`
- 変更: `message IpcHello` から行ごと削除
  - `string project_root = 5;`
- フィールド番号 5 は再利用しない（将来予約）。
- コメント更新（クライアント環境メタは `meta` に任意で入れられる旨のみ記載）。

再生成手順（参考）:
- Rust: `server/scripts/generate-rust-proto.sh` を実行（要 `protoc`）
- C#: `bridge/Tools/generate-csharp.sh` を実行
- `server/src/generated/schema.pb` と `server/src/generated/schema_hash.rs` を更新（CI でも再生成）

### 2) Unity 側実装
- 対象: `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/EditorIpcServer.cs`
- 変更:
  - `ValidateProjectRoot(string)` を完全削除（メソッド、呼び出し、関連ログを除去）
  - ハンドシェイク組み立て部から当該検証分岐を削除
  - 以降のロジックは現行どおり（`PathPolicy` は `Directory.GetCurrentDirectory()` を基準とするので影響なし）
- テスト:
  - `HandshakeRefactorTests.TestHandshakeProjectRootReject` を削除
  - `MockIpcClient` 内の `hello.ProjectRoot = ...` を削除（該当箇所複数）
  - 必要なら「接続成功」系の回帰テストのみ維持/強化

### 3) Rust 側実装
- 対象: `server/src/ipc/client.rs`
  - `IpcHello` の組み立てから `project_root` を削除
  - `normalize_project_root()` を削除（未使用化）
- 対象: `server/src/ipc/path.rs`
  - `IpcConfig` から `project_root: Option<String>` を削除
  - 付随する `Default` 実装の環境変数参照（`MCP_PROJECT_ROOT`）を削除
- 対象: `server/src/mcp/service.rs`（現状の接続可視化）
  - `BridgeState` と `unity_bridge_status` の `project_root` フィールドを削除（簡素化）
- ドキュメント/サンプル:
  - `docs/quickstart.md` から `MCP_PROJECT_ROOT` の節とトラブルシュートの記述を削除
  - `.mcp.json` 例からも該当環境変数を削除

### 4) 互換性/バージョン
- Proto 破壊的変更のため、スキーマバージョン/ハッシュが変わる
  - Unity パッケージ: `0.1.(x+1)` へ
  - Rust サーバー: パッチ/マイナー（`0.1.1` など）で明記
- ミックス環境の考慮
  - 新Unity ↔ 旧Rust（`project_root` を送る）: Proto 不一致でスキーマ不一致になるため同時リリースを推奨
  - 新Rust ↔ 旧Unity（`project_root` が必須）: 握手不可（Reject）→ 同時更新必須

## 受け入れ基準（テスト）
- ハンドシェイクが `project_root` 不在で成功する（トークン・バージョン・スキーマ一致が前提）
- 旧 `project_root` 関連テストがすべて削除/改修され、CI 緑
- `unity_assets_*`, `unity_health` など機能ツールが回帰
- スキーマハッシュが Rust/C# で一致
- ドキュメントから `MCP_PROJECT_ROOT` の案内が消えている

## リスクと対策
- 破壊的変更: サーバー/ブリッジの同時更新が必要
  - 対策: リリースノートで強調、タグ/UPM バージョンを揃える
- 複数 Editor 起動時の誤接続
  - 対策: トークンをプロジェクト単位で分ける運用を明示

## 作業ブレークダウン（目安）
1. Proto変更+生成（Rust/C#）: 0.5d
2. Unity 実装削除+テスト整理: 0.5d
3. Rust 実装削除+ビルド確認: 0.5d
4. ドキュメント更新: 0.25d
合計: 約 1.75d（バッファ込み 2d）

## ロールバック
- 直前タグへ戻す（Proto/生成物を含む）
- 一時回避は不可（Proto互換がないため）→ ロールバックは両側同時に実施

