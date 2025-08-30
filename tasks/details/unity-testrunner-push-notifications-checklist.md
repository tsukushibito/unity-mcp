# Unity TestRunner Push 通知 実装チェックリスト

- 設計
  - [ ] イベント名・ペイロード（started/finished）確定
  - [ ] `eventVersion`=1 で前方互換ポリシー明記
  - [ ] フィールド命名は camelCase に統一（runId, startedAt, finishedAt, resultsPath など）
  - [ ] resultsPath は Unityプロジェクト相対 `<ProjectName>/Temp/AI/tests/run-<runId>.json` に固定

- サーバ実装（Rust）
  - [ ] `UNITY_MCP_NOTIFICATIONS` を読み取り（デフォルト on）
  - [ ] `McpService::notify(method, payload)` ユーティリティ実装
  - [ ] `send_test_started_notification` を実通知に置換
  - [ ] `send_test_finished_notification` を実通知に置換
  - [ ] 送信失敗時は warn ログのみで継続
  - [ ] runId単位の finished 多重送信ガード（`HashSet<String>`）
  - [ ] 通知送信ハンドル: サービス起動後に Sender を注入し、ドレイナタスクで `send_notification` を呼び出す（方針A）

- 連携ポイント
  - [ ] `resultsPath` が `<ProjectName>/Temp/AI/tests/run-<runId>.json` を指すこと（"/" 正規化）
  - [ ] mode=all（直列結合）の要約値が JSON と一致
  - [ ] startedAt はサーバー時刻、finishedAt は Unity の結果 JSON から採用

- テスト/検証
  - [ ] ユニット: 通知ON/OFFの分岐をテスト
  - [ ] 回帰: `unity_run_tests` の同期レスポンス不変
  - [ ] 手動: 簡易クライアントで通知受信（runId, mode, summary）確認

- ドキュメント
  - [ ] README に通知仕様・サンプル追記（camelCase・resultsPath 仕様を含む）
  - [ ] フォールバック戦略（ファイル取得）の明記（通知なしでも従来どおり取得可能）
