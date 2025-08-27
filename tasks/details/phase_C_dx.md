# フェーズC — 開発者体験（DX）（詳細作業書）

## 概要
- 目的: 新規クローンから15分以内でE2Eを再現可能にする開発者体験の整備。
- スコープ: Quickstart、例の拡充（ログtail、成功/失敗の見える化）。
- 非スコープ: 包括的チュートリアルや動画教材の作成。
- 対応する計画項目: `mvp_work_plan_direct_ipc_v1.md` フェーズC（C1/C2）

## 前提/依存
- フェーズA/Bの仕様とUI文言が確定していること。

## 作業項目一覧

### C1: Quickstart作成
- C1-1: Unity起動→`cargo run --example test_unity_ipc` の通し手順の記述
- C1-2: EditorUserSettingsで`MCP.IpcToken`を設定する手順の明記（Env/EditorPrefsを不使用と明示）
- C1-3: トラブルシュート項目（Unityバージョン差・DLL等）

### C2: 例の拡充
- C2-1: Rust側でUnityログイベントを~10秒tailする例を追加
- C2-2: 成功/失敗の判定出力を明確化（色/タグなどは任意）

## 受け入れ条件（DoD）
- Quickstartに従い、15分以内にE2Eが再現できる
- 例の出力で成功/失敗が一目で分かる

## テスト
- 手動: 新規クローン環境でQuickstartを実施、タイムボックス内で再現
- 自動: 最低限のスクリプト体験（ヘルスチェック例）の実行確認

## リスク/ロールバック
- Unity環境差による躓き: トラブルシュートを厚めに用意

## 監査ログ
- スクリーンショット、ログ抜粋、PRリンク

## 参照
- `tasks/mvp_work_plan_direct_ipc_v1.md` フェーズC
- `tasks/mvp_worklist_checklist.md`

---

## 現状確認（2025-08-27）
- Rust 側
  - 例: `server/examples/test_unity_ipc.rs` が存在（接続＋Handshake＋Health の疎通確認）。
  - イベント: `IpcClient::events()` により `pb::IpcEvent` を購読可能（`events.log` フラグあり）。ログtail用の専用例は未実装。
  - 機能フラグ: `events.log` を含むクライアント対応が `server/src/ipc/features.rs` に実装済み。
  - README: Quickstart未掲載（hooksブートストラップのみ記載）。
- Unity 側（bridge）
  - トークン: `EditorUserSettings["MCP.IpcToken"]` を厳格に使用（Env/EditorPrefsは無効）。`EditorIpcServer.cs` で検証ロジック実装済み（空/不一致は `UNAUTHENTICATED`）。
  - スキーマ: SCHEMA_HASH 検証あり（不一致は `FAILED_PRECONDITION`）。
  - 設定UI: `MCP.IpcToken` を設定するメニュー/SettingsProviderは未実装（テストコードでの設定例はあり）。

判定: C1 はドキュメント中心（Quickstart整備＋トラブルシュート）、C2 は Rust 例の追加（ログtail）で実現可能。コード大改修は不要。

---

## 作業詳細（C1/C2）

### C1: Quickstart（詳細）
- やること
  - Q1: Quickstart ドキュメント新設（`docs/quickstart.md`）し、README からリンク。
  - Q2: 手順整備（Unity起動→トークン設定→Rust例の実行）。
  - Q3: トラブルシュート（Unityバージョン差、`MCP.IpcToken` 未設定、ポート競合、schema mismatch 等）。
  - Q4: 成功の見える化（例の出力スクリーンショット／サンプルログを掲載）。
- 具体手順（ドキュメントに記載）
  1) リポジトリをクローンし、`./scripts/bootstrap-hooks.sh` を実行。
  2) Unity Editor で `bridge/` を開く。
  3) `MCP.IpcToken` を設定（例: `test-token`）。方法は以下のいずれかを記載:
     - 一時的なエディタスクリプト（`Assets/Editor/SetIpcToken.cs`）で `EditorUserSettings.SetConfigValue("MCP.IpcToken", "test-token")` を実行（サンプルコード提示）。
     - 既存テストを流用する場合の注意（推奨はしないが参考として記載）。
  4) Unity が待受（127.0.0.1:7777）していることを確認（コンソールに `EditorIpcServer` のログ）。
  5) Rust 例を実行: `cd server && cargo run --example test_unity_ipc`（例内のトークンは `test-token`）。
  6) `✓ Handshake completed` と `✓ Health response` が出力されれば成功。
- 変更ファイル
  - `README.md`: Quickstart へのリンクと1スクリーン分のダイジェストを追記。
  - `docs/quickstart.md`: 新規作成（上記手順・Troubleshooting・FAQ）。
- 受け入れ条件
  - 新規クローン→Quickstartに沿って <15 分で `test_unity_ipc` が成功する。
  - `MCP.IpcToken` の設定方法が明確（Unityのどこで・どうやって）で、Env/EditorPrefs不使用が明記されている。
- 検証観点
  - Windows/macOS/Linux で手順に差異がないか（パスや改行差異を注記）。
  - スキーマ不一致・ポート占有時のガイダンス（メッセージ引用と対処）。

### C2: 例の拡充（詳細）
- やること
  - E1: 新規例 `server/examples/unity_log_tail.rs` を追加。`IpcClient::events()` を購読し、`ipc_event::Payload::Log` を ~10 秒間 tail して表示。
  - E2: 成功/失敗の判定出力を明確化。
    - Handshake 成功 → `[OK]` 表示。
    - `events.log` が交渉済みであることを表示（未交渉は `[WARN]`）。
    - ログ受信数が0件の場合は `[WARN] no logs received`。`Error/Warn` を受信したら件数集計。
    - 終了時にサマリ: `info=N warn=N error=N`。`error>0` の場合は非0で終了（終了コード1）。
- 変更ファイル
  - `server/examples/unity_log_tail.rs`: 新規作成。
  - （任意）`server/examples/test_unity_ipc.rs`: 出力整形（成功/失敗のタグを少し明確化）。
- 受け入れ条件
  - `cargo run --example unity_log_tail` が実行でき、10秒間のログを受信・集計してサマリを表示。
  - `events.log` 未交渉時は分かりやすい案内が出る（Unity側設定またはバージョン差を疑う）。
  - エラー出力が視覚的に識別しやすい（記号・タグ・色のいずれか）。
- 検証観点
  - Editor の Play/Stop, Asset Refresh などでログが流れることを確認（簡易手動）。
  - タイムアウトや切断時のメッセージが分かりやすいこと。

---

## 実施順序と所要目安
1) C1-Quickstart 文書化（0.5d）
2) C2-ログtail例 追加（0.5d）
3) README 整理・スクリーンショット反映（0.25d）
4) 手動E2Eリハーサル（0.25d）

---

## トラブルシュート追記案（Quickstartに同梱）
- `UNAUTHENTICATED: Missing or empty token` → Unity の `EditorUserSettings["MCP.IpcToken"]` を設定（例: `test-token`）。
- `FAILED_PRECONDITION: schema mismatch` → C# 側 SCHEMA_HASH を再生成（CI/手順参照）。
- `FAILED_PRECONDITION: project_root mismatch` → Rust 側 `IpcHello.project_root` が Unity のプロジェクト直下の絶対パスと一致しているか確認。
- `UNAVAILABLE: editor compiling/updating` → Unity のコンパイル/更新完了後に再試行。
- ポート `127.0.0.1:7777` に接続不可 → 他プロセス占有/Firewall/Editor 起動状態を確認。

---

## 成果物の場所（予定）
- `docs/quickstart.md` — Quickstart 本文（スクショ/ログ例含む）
- `README.md` — Quickstart 概要とリンク
- `server/examples/test_unity_ipc.rs` — ハッピーパスの基本疎通
- `server/examples/unity_log_tail.rs` — ログイベントの10秒tail＋集計

---

## 実装メモ（例コード方針）
- `unity_log_tail.rs`（擬似コード）
  - `IpcClient::connect(cfg)` → `client.events()` を `tokio::select!` で 10 秒間購読。
  - `ipc_event::Payload::Log` を match し、`log_event::Level` ごとにカウント。
  - 最後にサマリ出力。`error>0` で `std::process::exit(1)`。
  - トークン/エンドポイントは `test_unity_ipc.rs` と同等の最低限（将来は CLI 引数化）。

