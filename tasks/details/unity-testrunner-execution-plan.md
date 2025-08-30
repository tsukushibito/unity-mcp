# Unity TestRunner 実行と結果通知 機能計画（MVP）

## 目的 / ゴール

- Unity の EditMode/PlayMode テストをエディタ内で実行し、結果を JSON で `bridge/Temp/AI/tests/` 配下に保存する。
- Rust MCP サーバーからテスト実行をトリガし、完了時に結果を取得できる API を提供する。
- 実行開始/完了を MCP 通知でクライアントに知らせる（push）。
- 依存は Unity 標準 API（`TestRunnerApi` / `JsonUtility`）のみ。外部 JSON ライブラリは禁止。

## スコープ（MVP）

- 対応テスト: EditMode / PlayMode（両方または片方）。
- フィルタ: `testFilter`（部分一致）, `categories`（含む）, `mode`（edit/play/all）。
- 実行トリガ: Rust から `runTests` リクエストファイルを配置 → Unity Editor 側が検知して実行。
- 結果収集: 成功/失敗/スキップ、所要時間、メッセージ/スタック、カテゴリ、アセンブリ名。
- 出力ファイル: `latest.json` と `run-<id>.json`（<id> は ISO8601+短ハッシュ）。
- 通知: `unity.tests.started`, `unity.tests.finished` を MCP で発火。
- セキュリティ/制限: `bridge` 配下のみアクセス、~2MB 超はエラー、タイムアウト制御。

非スコープ（将来）

- Code Coverage 収集・レポート統合
- Retry/Flaky 自動判定、Rerun 失敗のみ
- 並列ワーカー/複数 Unity セッション連携

## アーキテクチャ概要

1) Rust MCP ツール `unity_run_tests` が `bridge/Temp/AI/requests/runTests-<id>.json` を作成。
2) Unity Editor の `McpTestRunner` がポーリング（`EditorApplication.update`）で検知、`TestRunnerApi` で実行。
3) 実行中はメモリに結果を蓄積。完了で `bridge/Temp/AI/tests/run-<id>.json` と `latest.json` に `JsonUtility` で保存。
4) Unity 側が `started`/`finished` をブリッジへシグナル（ファイル・フラグ更新）。
5) Rust 側は結果ファイル生成/更新を待機し、内容を読み取りレスポンス返却、並びに MCP 通知発火。

ディレクトリ（相対: repo ルート）

- `bridge/Packages/com.example.mcp-bridge/Editor/McpTestRunner.cs`
- `bridge/Temp/AI/requests/`（Rust→Unity）
- `bridge/Temp/AI/tests/`（Unity→Rust）

## MCP ツール設計

### ツール: `unity_run_tests`

- 入力
  - `mode`: `"edit"|"play"|"all"`（既定: `"edit"`）
  - `test_filter`: string（任意）
  - `categories`: string[]（任意, OR）
  - `timeout_sec`: number（既定: 180）
  - `max_items`: number（既定: 2000）
  - `include_passed`: boolean（既定: true）

- 出力
  - `run_id`: string
  - `summary`: { `total`, `passed`, `failed`, `skipped`, `duration_sec` }
  - `tests`: TestResult[]（上限・フィルタ済み）
  - `truncated`: boolean

- 通知（MCP notification）
  - `unity.tests.started`: { `run_id`, `mode`, `filter`, `categories` }
  - `unity.tests.finished`: { `run_id`, `summary`, `truncated` }

### ツール: `unity_get_test_results`

- 入力
  - `run_id?`: string（省略時は `latest.json`）
  - `max_items?`, `include_passed?`

- 出力: `unity_run_tests` と同一スキーマ

## JSON スキーマ（Unity→Rust）

```json
{
  "runId": "2025-08-30T05:12:10Z_8f3a",
  "startedAt": "2025-08-30T05:12:10Z",
  "finishedAt": "2025-08-30T05:12:31Z",
  "mode": "edit",
  "filter": "Player*",
  "categories": ["fast"],
  "summary": {"total": 120, "passed": 118, "failed": 1, "skipped": 1, "durationSec": 21.2},
  "tests": [
    {
      "assembly": "Game.EditModeTests",
      "suite": "PlayerServiceTests",
      "name": "Save_ShouldPersist",
      "fullName": "Game.Tests.PlayerServiceTests.Save_ShouldPersist",
      "status": "passed", // failed | skipped | inconclusive
      "durationSec": 0.031,
      "message": "",
      "stackTrace": "",
      "categories": ["fast"],
      "owner": "",
      "file": "Assets/Tests/PlayerServiceTests.cs",
      "line": 42
    }
  ]
}
```

備考

- `file`/`line` はスタックトレースから推定（最初のユーザーコードフレーム）。取得不可なら省略。
- `JsonUtility` で表現できるフラット/配列構造に限定（辞書は避ける）。

## Unity Editor 実装方針

- 場所: `bridge/Packages/com.example.mcp-bridge/Editor/McpTestRunner.cs`
- API: `UnityEditor.TestTools.TestRunner.Api.TestRunnerApi`
  - `Execute(ExecutionSettings)` を使用。
  - `ITestRunCallback` ではなく `ITestRunListener` 経由のイベントで集計。
- 監視: `EditorApplication.update` で `requests/runTests-*.json` をポーリング（100–300ms 間隔）。
- 直列実行: 進行中は次のリクエストを待機キューへ。
- JSON 出力: `JsonUtility.ToJson`（`prettyPrint=false`）。
- バッファクリア: 実行開始時に収集バッファを初期化。
- ファイル命名: `run-<id>.json`, `latest.json`。`<id>` は `startedAt` + 先頭4桁ハッシュ。
- 例外/失敗: 実行不能時は `summary.failed = total` かつ `message` に理由を格納。

## Rust 実装方針

- ファイル: `server/src/mcp/tools/tests.rs` を新規追加し `tools.rs` で登録。
- リクエスト生成: `UNITY_MCP_REQ_PATH`（既定: `bridge/Temp/AI/requests`）。
- 結果読取: `UNITY_MCP_TESTS_PATH`（既定: `bridge/Temp/AI/tests`）。
- セキュリティ: `bridge` 配下検証、最大サイズ ~2MB、タイムアウトは入力または既定で終了。
- 通知: 実行直後に `started`、結果取得後に `finished` を送出。
- 失敗時ガイダンス: 「Unity エディタが開いているか」「ブリッジがインストール済みか」を返す。

## 受け入れ基準（AC）

- AC1: `unity_run_tests` が `mode=edit` で実行し、120件中 1 failed を含む `summary` を返す。
- AC2: `test_filter`/`categories` が反映され、対象テストのみ `tests[]` に含まれる。
- AC3: 実行完了まで待機し、`timeout_sec` 超過でエラー（ガイダンス付き）。
- AC4: `unity_get_test_results` が `latest.json` を読み出し、上限/フィルタを適用して返す。
- AC5: JSON は `JsonUtility` 互換スキーマで、Newtonsoft に依存しない。
- AC6: `started`/`finished` 通知が 1 回ずつ発火する。

## テスト計画

- Rust 単体: リクエスト/結果パス解決、サイズ/タイムアウト、フィルタ/上限、通知呼び出し。
- Unity プレイバック: 失敗/成功/スキップ混在の最小テストアセンブリで JSON 生成確認。
- 耐障害: Unity 未起動・パス不在・壊れた JSON 時のエラーハンドリング。

## 作業ブレークダウン / 見積

1) 仕様固定・スキーマ決定（本ドキュメント）: 0.5d
2) Unity 側実装（McpTestRunner + JSON 出力）: 1.0d
3) Rust ツール（run/get + 通知）: 1.0d
4) 動作検証/微修正（タイムアウト・パス）: 0.5d
5) ドキュメント/README 追記: 0.5d

## リスク / 対応

- TestRunnerApi のイベント差異（Unity バージョン依存）→ 最小バージョンを README に明記。
- スタックトレースからファイル/行抽出失敗 → 任意項目にして回避。
- 大量出力で 2MB 超 → `max_items` 既定と `include_passed=false` を推奨。

