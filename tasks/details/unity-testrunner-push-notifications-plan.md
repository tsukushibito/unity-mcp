# Unity TestRunner Push 通知 実装計画

目的: Rust MCP サーバーから MCP クライアントへ `unity.tests.started` / `unity.tests.finished` を送出し、テスト実行の開始/完了を即時に伝達する。既存のファイルベース検知（status(-<runId>).json, latest.json）はフォールバックとして継続。

## 背景とねらい
- 現状でも `unity_run_tests` の同期レスポンス＋ファイル監視で完結可能だが、通知により以下が改善される:
  - 長時間テストや `mode=all` 直列実行のUX向上（開始/完了を即時反映）。
  - 複数ラン並行時の相関付け（runIdで誤対応を避ける）。
  - 後続処理（レポート生成、Slack/Issue投稿など）の自動トリガー。

## スコープ（決定事項を反映）
- 追加する通知メソッド（v1）
  - `unity.tests.started`
  - `unity.tests.finished`
- 進捗通知（`unity.tests.progress`）は将来拡張。今回のスコープ外。
- resultsPath は「Unityプロジェクトディレクトリからの相対パス」。固定で `<ProjectName>/Temp/AI/tests/run-<runId>.json` を指す（区切りは "/" に正規化）。
- Unity 側の出力先は固定（<ProjectRoot>/Temp/AI/tests）。サーバ環境変数 `UNITY_MCP_TESTS_PATH` はサーバーからの参照先変更のみで、Unity 側の書き込み先は変更しない。

## 非スコープ
- 個々のテストケースを通知で逐次送出しない（サイズ/スパム回避）。
- 通知の再送/確実配送（ベストエフォート運用。結果はファイルで取得）。

## 仕様（イベントとペイロード）
- 共通: `eventVersion: 1` を付与。未使用フィールドは将来用に拡張可能。
- 命名規約: 全フィールドは camelCase（runId, testFilter, startedAt, finishedAt, resultsPath など）。

### unity.tests.started
```jsonc
{
  "eventVersion": 1,
  "runId": "2025-08-30T12:34:56Z-1a2b3c4d",
  "mode": "edit|play|all",
  "testFilter": "<string>",
  "categories": ["<string>"],
  "startedAt": "2025-08-30T12:34:56.789Z"
}
```

### unity.tests.finished
```jsonc
{
  "eventVersion": 1,
  "runId": "2025-08-30T12:34:56Z-1a2b3c4d",
  "mode": "edit|play|all",
  "finishedAt": "2025-08-30T12:36:12.012Z",
  "summary": { "total": 123, "passed": 120, "failed": 2, "skipped": 1, "durationSec": 75.22 },
  "truncated": false,
  "resultsPath": "<ProjectName>/Temp/AI/tests/run-<runId>.json" // Unityプロジェクト相対パス
}
```

備考:
- テスト明細（tests配列）は通知に含めない。必要に応じて `resultsPath` を取得して `unity_get_test_results` を呼ぶ。
- `mode=all` は Edit→Play 直列実行の結合結果を表す。
- startedAt はサーバー時刻（ISO8601、UTC）を使用。finishedAt は Unity の結果 JSON から採用。

## 設定と互換性
- 環境変数 `UNITY_MCP_NOTIFICATIONS`（`on`/`off`、デフォルト: `on`）。
- 通知未対応クライアント向けに、従来の `status(-<runId>).json` と同期レスポンスは維持。
- MCPクライアント側に特別な capability 宣言は不要（未知通知は無視可能）。

## 実装タスク
1) 通知送信ユーティリティの追加（Rust）
   - `McpService` に `notify(&self, method: &str, payload: serde_json::Value) -> Result<(), Error>` を追加。
   - `UNITY_MCP_NOTIFICATIONS` を参照し、`off` の場合は no-op。
   - 送信失敗は `warn!` ログにとどめる（ベストエフォート）。
   - 送信ハンドルの取り回し: `serve_stdio()` で確立される rmcp サービスハンドルに対して、
     - 方針A: サービス起動後に `McpService` 内部へ通知 Sender を注入し、バックグラウンドで drain して `send_notification` を呼ぶ。
     - 方針B: rmcp の提供するコンテキスト API を `notify()` 内から直接呼び出す。
     - 本実装では方針Aを採用（非同期チャネル＋ドレイナタスク）し、起動順の依存を解消。

2) 送信タイミングの配線（Rust）
   - started: リクエストファイル作成・runId確定直後（既存 `send_test_started_notification` を置換）。
   - finished: `wait_for_test_completion` 正常終了後（既存 `send_test_finished_notification` を置換）。
   - ペイロード作成時に `resultsPath` は `<ProjectName>/Temp/AI/tests/run-<runId>.json` を設定。

3) 冪等/多重送信ガード
   - `McpService` に `sent_finished: HashSet<String>` を持たせ、runId 単位で finished を一度だけ送信。
   - エッジ: 同一プロセス内の重複起動や再試行でも runId でデデュープ。

4) ロギングと観測性
   - 送信前後で `info!` を出す（method, runId, mode）。
   - 失敗時は `warn!` に例外内容。

5) ドキュメント更新
   - README に通知メソッド/ペイロード例/フォールバック戦略を追記。

6) 最小テスト
   - ユニット: 通知ユーティリティが `off` で no-op、`on` で呼び出し成功を返す（送信層をモック化/抽象化）。
   - 回帰: `unity_run_tests` の同期レスポンスが従来通り返ることを確認。

## 受信側の想定
- IDE/エージェントは `unity.tests.started/finished` を受け取り、
  - started: UIに「テスト実行中」を表示する。
  - finished: 要約を表示し、必要なら `resultsPath` 参照で詳細をロード。
  - 未対応クライアントは通知を無視しても動作に影響なし。

## リスクと緩和
- 配信順序・重複: ベストエフォート。重複は runId でデデュープ。
- ペイロード肥大化: 要約のみとしリンクで詳細取得。
- 互換性: eventVersion で将来拡張を吸収。

## 受け入れ条件（AC）
- `UNITY_MCP_NOTIFICATIONS=on` で started/finished が送出される。
- 通知未受信でも `unity_run_tests` と `unity_get_test_results` で従来通り完結。
- mode=all の結合ランでも `run-<runId>.json` と整合する要約が届く。
- `resultsPath` は常に `<ProjectName>/Temp/AI/tests/run-<runId>.json`（camelCase・正規化スラッシュ）で提供される。

## 運用チェックリスト（抜粋）
- 通知ON/OFF設定が機能する。
- 並列ランA/Bで runId が正しく相関する。
- 通知が失敗しても実行フローが止まらない。
