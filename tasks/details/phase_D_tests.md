# フェーズD — テスト強化と最終仕上げ（詳細作業書）

## 概要
- 目的: Rust/Unity双方のテストを拡充し、最終ゲート前の品質を担保する。
- スコープ: Rust統合テスト、Unity EditModeテスト、再接続の仕上げ（任意）。
- 非スコープ: 長期耐久・負荷試験。
- 対応する計画項目: `mvp_work_plan_direct_ipc_v1.md` フェーズD（D1/D2/D3）

## 前提/依存
- フェーズA/B/Cの仕様が確定していること。

## 作業項目一覧

### D1: Rust統合テスト追加
- D1-1: schema mismatch（済）
  - 現状: `server/tests/ipc_integration.rs::test_schema_hash_mismatch_rejection` で検証済み。
  - 期待: `FAILED_PRECONDITION`／メッセージに `Schema hash mismatch` を含む。

- D1-2: project_root mismatch（追加）
  - 目的: `IpcHello.project_root` が Unity 側の `GetFullPath(プロジェクト直下)` と一致しない場合、`FAILED_PRECONDITION: project_root mismatch` で拒否されることを検証。
  - 実装方針:
    - 既存 `MockUnityServer` を拡張し、期待プロジェクトルート（絶対パス）をオプションで受け取り、`hello.project_root` と比較して不一致なら `IpcReject(FailedPrecondition, "project_root mismatch")` を返す分岐を追加。
    - 新規テスト: `test_project_root_mismatch_rejection`
      - 不一致の `project_root` を送出し、`IpcClient::connect` が `IpcError::FailedPrecondition` を返し、メッセージに `project_root mismatch` を含むことを確認。
      - Windows 正規化（`\\?\`, `\\?\UNC\` の除去、区切り/末尾セパレータ整形）に依存しないテストデータにする。
  - 参考: Rust クライアントは `normalize_project_root()`（Windows 前置詞除去・区切り正規化）を実装済み。`MCP_PROJECT_ROOT` が設定されていればそれを優先。

- D1-3: 機能交渉（unknown featureのドロップ）（済）
  - 現状: `test_feature_negotiation_intersection`、`test_unknown_features_filtered_during_negotiation` で検証済み。
  - 期待: 未知機能は交渉結果に含まれないこと。

### D2: Unity EditModeテスト更新
- D2-1: トークン必須（追加）
  - 目的: トークン未設定/空/不一致で `UNAUTHENTICATED` となることを、EditMode 単体テストで検証。
  - 実装方針:
    - 既存のテストアクセサ `EditorIpcServerAccessor` を拡張し、`ValidateToken(expected, client)` と `LoadTokenFromPrefs()` へリフレクションでアクセス可能にする。
    - ケース: 未設定（null/empty）、不一致（"invalid token"）、一致（成功）を確認。
  - 期待メッセージ: `Missing or empty token. Set EditorUserSettings: MCP.IpcToken`／`Invalid token. Check EditorUserSettings: MCP.IpcToken`

- D2-2: 基本ハッピーパス（Health/Assets/Build）（追加）
  - Health: 既存テスト（Fast/Strict）でOK。継続利用。
  - Assets（軽量）:
    - `AssetsHandler` を反射で呼び出し、`P2G("Assets") → GUID`、`G2P(GUID) → "Assets"` の往復と、`Refresh(force=false)` 成功を確認。
    - ファイル書き込みを伴わない範囲で完結（CI 安定性優先）。
  - Build（軽量）:
    - `BuildHandler` を反射で呼び出し、`BuildAssetBundles` を `Library/McpTestBundles` 等の一時出力へ最小実行し、`StatusCode=0` を期待。
    - Player ビルドは重いのでCI対象外（別途 `#[ignore]` 統合テストに委譲）。

- D2-3: トークン取得経路の検証（EditorUserSettingsのみ有効、Env/EditorPrefsは無視）（追加）
  - 目的: `EditorUserSettings["MCP.IpcToken"]` のみをソースとし、環境変数や `EditorPrefs` は無視されることを確認。
  - 実装方針:
    - テスト内で `EditorUserSettings.SetConfigValue` に値A、`Environment.SetEnvironmentVariable` に値B、`EditorPrefs.SetString` に値C を設定。
    - `LoadTokenFromPrefs()` が常に値Aを返し、B/Cを参照しないことを検証。

### D3: （任意）再接続の仕上げ
- D3-1: `spawn_supervisor` の writer 差し替えを実装
- D3-2: 手動検証（Unity再起動→自動再接続）

## 受け入れ条件（DoD）
- 上記テストが追加・更新され、`cargo test` とUnity EditModeテストがグリーン

## テスト
- 自動: `server/tests/**`、Unity EditMode
- 手動: 再接続挙動の簡易検証

## 実行手順（想定）
- Rust 側: `cargo test -p server`（Unity不要のテストは即時実行可。`#[ignore]` が付くUnity依存は除外）
- Unity 側: Test Runner で EditMode テストを実行（新規 D2 テストを含む）。
- 環境変数: `MCP_PROJECT_ROOT` を必要に応じ設定（例: `bridge` の絶対パス）。Windows は `Resolve-Path` で絶対パス解決を推奨。

## 実装メモ/変更予定ファイル（テストのみ）
- Rust
  - `server/tests/ipc_integration.rs`: `MockUnityServer` に project_root 検証を追加、`test_project_root_mismatch_rejection` を新規追加。
- Unity（EditMode）
  - `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/Tests/HandshakeTests.cs`: Token 必須/経路テストを追加、アクセサ拡張。
  - 新規 or 既存テストに追記: Assets/G2P/P2G/Refresh の軽量ハッピーパス、BuildAssetBundles の最小ハッピーパス。

## リスク/回避策 追記
- Build の最小ケースでも環境差で時間がかかる可能性 → 出力先を `Library/` 配下に限定し、タイムアウトを十分に確保。
- テストフレーク → 明確なタイムアウトとリトライ回数の制御、Unity API を触る箇所はできる限りメインスレッド実行を保証。

## リスク/ロールバック
- テストフレーク: リトライや適切なタイムアウト設定

## 監査ログ
- CI実行URL、テストレポート、PRリンク

## 参照
- `tasks/mvp_work_plan_direct_ipc_v1.md` フェーズD
- `tasks/mvp_worklist_checklist.md`
