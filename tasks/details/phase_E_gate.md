# フェーズE — マイルストーン検証（ゲート）（詳細作業書）

## 概要
- 目的: 新規クローン環境でのE2E再現とCIグリーンにより、MVP到達をゲート判定する。
- スコープ: E2E手順の最終確定、検証ログ/スクリーンショット収集、CI結果の証跡化。
- 非スコープ: 長期運用ドキュメントの整備（必要があれば別途）。
- 対応する計画項目: `mvp_work_plan_direct_ipc_v1.md` フェーズE（E1/E2）

## 現状サマリ（2025-08-27 時点）
- ローカル（この環境）
  - Rust: `cargo fmt --check` / `cargo clippy -D warnings` / `cargo test` いずれもグリーンを確認。
  - Rust 生成物: `server/scripts/generate-rust-proto.sh` はレジストリ権限の都合でローカル再生成は失敗ログあり（CIでは成功想定）。差分チェックでは生成物ドリフトは検出されず。
  - E2E 手順: `docs/quickstart.md` に15分以内の手順を整備済み。`unity_log_tail` は `MCP_TAIL_SECS` で tail 時間を延長可、ハートビートで再接続検証もしやすい。
- CI（GitHub Actions）
  - `.github/workflows/ci.yml` で以下を実施:
    - マトリクス（Linux/macOS/Windows）で Rust の fmt/clippy/build/test
    - `parity-check` ジョブで Rust/C# の Proto & Schema Hash パリティ検証（再生成→drift検出）
  - 判定: 本フェーズでは両ジョブがグリーンであることをゲート条件とする。

## 前提/依存
- フェーズA〜Dが完了し、手順が最新化されていること。

## 作業項目一覧

### E1: 新規クローンでE2E再現（<15分）
- E1-1: 手順の最終確定（Quickstartをベースに差分反映）
- E1-2: タイムボックス検証（15分以内で接続/Health/ログ受信まで）
- E1-3: ログ/スクリーンショットの収集・掲載（保存先の標準化）

実行手順（最終版）
1) リポジトリをクローンし初期化
```
git clone <repo-url>
cd unity-mcp
./scripts/bootstrap-hooks.sh  # Windowsは .\\scripts\\bootstrap-hooks.ps1
```

2) Unity で `bridge/` を開く（Editor起動）
- 127.0.0.1:7777 で待受ログが出ることをConsoleで確認

3) Token 設定（必須）
- `EditorUserSettings["MCP.IpcToken"]` のみを使用。Quickstartの手順A/B/Cのいずれかで `test-token` を設定。

4) Project Root 設定（推奨）
- `MCP_PROJECT_ROOT` に `bridge/` の絶対パスを設定。未設定時はカレント `.` が送信されるため、`bridge/` をカレントにして実行でも可。

5) Rust 例の実行（接続→Health）
```
cd server
cargo run --example test_unity_ipc
```
- 期待: `Connected / Handshake completed / Health response` が表示。

6) ログTailの実行（ログ受信の可視化）
```
cargo run --example unity_log_tail           # 既定 ~10s
# あるいは
MCP_TAIL_SECS=30 cargo run --example unity_log_tail
```
- 期待: tail中に `events.log` を受信し、サマリで info/warn/error 件数が表示。
- 注意: error>0 の場合は終了コード1。

エビデンス保存（標準）
- 保存先: `docs/evidence/phase_E/` ディレクトリ（新規作成可）
  - `e1_test_unity_ipc_<YYYYMMDD-HHMM>.log`（標準出力を保存）
  - `e1_unity_log_tail_<YYYYMMDD-HHMM>.log`（標準出力を保存）
  - `screenshots/` 以下に Unity Console のスクショ（待受ログ、ログイベント受信時）

想定ハマり点と対処
- `UNAUTHENTICATED: Missing or empty token`
  - Token が未設定/空。`EditorUserSettings` で設定する（環境変数やEditorPrefsは無視される）。
- `FAILED_PRECONDITION: project_root mismatch`
  - `MCP_PROJECT_ROOT` を `bridge/` の実パスに合わせる、もしくは `bridge/` をカレントにして実行。
- 接続不可（tcp://127.0.0.1:7777）
  - Editor起動/ポート占有/Firewallを確認。

### E2: CIグリーン
- E2-1: Rust build/test/clippy/fmt ＋ Proto & Schema parity のグリーン確認
- E2-2: 失敗時の再現手順と修正反映（ドキュメント/スクリプト更新含む）

確認ポイント（CI）
- ワークフロー: `.github/workflows/ci.yml`
  - `build-test`（OSマトリクス）: fmt / clippy / build / test が ALL PASS
  - `parity-check`: 
    - Rust再生成→`src/generated/` に drift なし
    - C#生成→`Editor/Generated/` に drift なし
    - `schema.pb` の SHA-256 と `SchemaHash.cs` の `SCHEMA_HASH_HEX` が一致

ローカル再現コマンド（開発者向け）
```
cd server && cargo fmt --all -- --check
cd server && cargo clippy --all-targets -- -D warnings
cd server && cargo test -- --nocapture

# Rust生成物の再作成（環境によってはレジストリ権限が必要）
cd server && ./scripts/generate-rust-proto.sh
git diff --exit-code server/src/generated/

# C#生成物の再作成
cd bridge && ./Tools/generate-csharp.sh
git diff --exit-code bridge/Packages/com.example.mcp-bridge/Editor/Generated/
```

失敗時の標準対応
- Rust 側 drift: `cd server && ./scripts/generate-rust-proto.sh` を実行し生成物をコミット
- C# 側 drift: `cd bridge && ./Tools/generate-csharp.sh` を実行し生成物をコミット
- Schema Hash 不一致: 上記C#生成物を更新し `SchemaHash.cs` をコミット
- clippy/fmt: ローカルで修正→再実行→PR に反映

## 受け入れ条件（DoD）
- E2Eが15分以内に再現でき、ログ/スクショが `docs/evidence/phase_E/` に保存されている
- CIがグリーン（`build-test` と `parity-check` の両方）で、実行リンクとコンソール出力要約が記録されている

## テスト
- 手動: 新規環境でのE2E実施
- 自動: CIの完全グリーン

## リスク/ロールバック
- 新規環境特有の依存抜け: Quickstartの追記やツールバージョン固定で緩和（`protoc`/Rust toolchain）
- 生成物ドリフト: CI のパリティチェックで早期検知。再生成スクリプトに修正を反映。

## 監査ログ
- 手順書（`docs/quickstart.md` の参照と差分）、収集ログ、スクリーンショット、CI実行リンク
- 保存場所の標準: `docs/evidence/phase_E/` 以下（コミット可）

## 参照
- `tasks/mvp_work_plan_direct_ipc_v1.md` フェーズE
- `tasks/mvp_worklist_checklist.md`
- `docs/quickstart.md`
- `.github/workflows/ci.yml`
