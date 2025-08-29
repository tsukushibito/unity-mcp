# Direct IPC Unity MCP Server — 作業計画（MVP収束）

本ドキュメントは、MVPのDoD達成に向けた「実施順序」「依存関係」「ゲート条件」を示す作業計画です。進行中の個別アイテムのチェック状況は `tasks/mvp_worklist_checklist.md` を更新してください。

## 目的
- Handshakeの不変条件（トークン必須・スキーマ一致）を確立し、通信の健全性を担保。
- プロト整合をCIで自動検出し、差分の取りこぼしを防止。
- 新規クローンでも15分以内にE2Eを再現可能な開発者体験を整える。

## 前提
- 直IPC（TCP 127.0.0.1:7777）、EditorDispatcherによるメインスレッド実行、Assets/Build/Logsの基本機能は実装済み。
- `tasks/direct_ipc_unity_mcp_server_mvp_task_list.md` はロードマップ参照用として凍結済み。

## 実施順序（フェーズ）
1) フェーズA — ハンドシェイク不変条件の確立（最優先）
- A1: スキーマハッシュ検証（Unity側）
  - C#側に `SCHEMA_HASH` 定数を事前生成してリポジトリに配置（Git管理）。
  - `IpcHello.schema_hash` と一致判定。相違は `FAILED_PRECONDITION` でReject。
  - Rust統合テストに「不一致→SchemaMismatch」を追加。
- A2: トークン必須（No Dev Mode）
  - 期待トークン未設定でも、空/未設定トークンをReject（`UNAUTHENTICATED`）。
  - エラー文面に設定方法を簡潔に案内（Unity: EditorUserSettings のみ、MCPサーバー: プロセス起動時のenvまたは`.cargo/config.toml`）。
  - Unity側は「EditorUserSettingsのみ」から取得する方針に変更（環境変数 `MCP_IPC_TOKEN` と `EditorPrefs` の使用は廃止）。
  - EditorUserSettings対応の簡易設定UI（任意・`SettingsProvider`）を検討（MVP後でも可）。

2) フェーズB — CIとSSoTの固定化
- B1: Rust側 proto 再生成＋差分検出をCIへ追加（失敗時に明快なメッセージ）。
- B2: C#の `SCHEMA_HASH_HEX` はRust `SCHEMA_HASH` から事前生成してGit管理。CIはRust↔C#のパリティチェックで不整合を検出し、再生成手順を提示。

3) フェーズC — 開発者体験（DX）
- C1: Quickstart作成（Unity起動→`cargo run --example test_unity_ipc`）。
  - 設定手順に「Unity: EditorUserSettings で `MCP.IpcToken` を設定（環境変数/EditorPrefsは不使用）」を明記。
- C2: 例の拡充（ログイベントを~10秒tailし、成功/失敗の出力を明確化）。

4) フェーズD — テスト強化と最終仕上げ
- D1: Rust統合テスト追加
  - Schema mismatch、project_root mismatch、機能交渉（unknown featureのドロップ）。
- D2: Unity EditModeテスト更新
  - トークン必須、基本ハッピーパス（Health/Assets/Build）。
  - トークン取得経路の検証（EditorUserSettingsのみ有効、環境変数/EditorPrefsが設定されていても無視される）。
- D3: （任意）再接続の仕上げ
  - `spawn_supervisor` の writer 差し替えを実装。最低限の手動検証（Unity再起動→自動再接続）。

5) フェーズE — マイルストーン検証（ゲート）
- E1: 新規クローンでE2E再現（<15分）
  - Handshake OK（features＋schema hash一致）。
  - `unity.health` 正常応答。
  - UnityログがRustで確認可能。
  - Assets基本操作成功（p2g/g2p/import/refresh）。
  - Minimal Build開始→イベント受信→完了確認。
- E2: CIグリーン
  - Rust build/test/clippy/fmt＋proto parity check。

## 依存関係と並行実行
- A（不変条件）はB～Dの前提。A完了までは他の変更は抑制（ドリフト防止）。
- B1/B2は互いに独立だが、C# `SchemaHash` の生成仕様が固まってからQuickstartで言及。
- CはA/Bの影響を受けるため、基本はA/Bのレビュー完了後に着手。
- Dは随時追加可能だが、最終ゲートEの直前に必ず再実行。
- D3（再接続）は時間制約に応じて後回し可（MVPストレッチ）。

## 成果物のマッピング（チェックリスト対応）
- A1: 「Schema Hash 検証」
- A2: 「トークン必須ポリシー」
- B1/B2: 「CI & Proto Parity」
- C1/C2: 「Developer Quickstart」「Examples」
- D1/D2/D3: 「Tests（Rust/Unity）」「Stability（任意）」
- E: 「Milestone Verification」

## レビュー/承認ゲート
- 各フェーズ完了時に担当外1名が確認（テスト結果とCIログのスクリーンショット/ログ添付）。
- EのE2E検証は、新規クローン環境で実施（手順に準拠してタイムボックス内に再現できること）。

## リスク/緩和
- プロトドリフト: B1でCIに差分検出を導入し、PR時に止める。
- Unity依存（DLL/Editorバージョン差）: Quickstartにトラブルシュート項目を新設。
- イベント/書き込み競合: 既存のスレッドセーフ書き込みで回避。高負荷時の改善はポストMVPで検討。

リンク
- チェックリスト: `tasks/mvp_worklist_checklist.md`
- ロードマップ（凍結）: `tasks/direct_ipc_unity_mcp_server_mvp_task_list.md`
 - 詳細作業書（フェーズ別）:
   - フェーズA — ハンドシェイク不変条件: `tasks/details/phase_A_handshake.md`
   - フェーズB — CIとSSoTの固定化: `tasks/details/phase_B_ci_ssot.md`
   - フェーズC — 開発者体験（DX）: `tasks/details/phase_C_dx.md`
   - フェーズD — テスト強化と最終仕上げ: `tasks/details/phase_D_tests.md`
   - フェーズE — マイルストーン検証（ゲート）: `tasks/details/phase_E_gate.md`
