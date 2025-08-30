# Unity TestRunner 実装レビュー（MVP）

作成日: 2025-08-30
対象変更:
- Unity: `bridge/Packages/com.example.mcp-bridge/Editor/McpTestRunner.cs`
- Rust: `server/src/mcp/tools/tests.rs`, `server/src/mcp/tools.rs`, `server/Cargo.toml`

## 総評
- 実装の方向性は計画どおり。Rust 側はルーティング、パス検証、サイズ上限、タイムアウト待機が適切。
- クリティカルな不整合が Unity 側 TestRunner API 呼び出しと `status.json` 出力にあり、MVP の動作に直結。下記の修正を優先してください。

## クリティカル（要修正）
- TestRunner API の呼び出しシグネチャ
  - 現状: `testRunnerApi.Execute(executionSettings, filter);`（不正）
  - 正: `executionSettings.filters = new[] { filter }; testRunnerApi.Execute(executionSettings);`
- `status.json` のシリアライズ
  - 現状: 匿名型を `JsonUtility.ToJson` に渡しており失敗する。
  - 正: `[Serializable] class TestRunStatus { public string status; public string runId; public string timestamp; }` を定義して使用。

## 高優先（機能の穴埋め）
- `includePassed`/`maxItems` の Unity 側適用
  - 目的: 出力量を抑制し、Rust 側 2MB 上限に抵触しにくくする。
  - 変更点: `OnRunFinished` で `request.includePassed` を適用し、`Take(request.maxItems)` の結果で `truncated` を算出。
- `prettyPrint` の無効化
  - 変更点: `JsonUtility.ToJson(results, false)` に変更（`latest.json`/`run-<id>.json`）。
- `mode=all` の扱い
  - 推奨: `EditMode`→完了→`PlayMode` の直列2回実行で結合。単一呼び出しより安定。
- `targetPlatform` の固定解除
  - 現状: `StandaloneWindows64` 固定。Editor 実行で不要。クロスプラットフォームのため削除/条件化。

## 中優先（堅牢性/互換性）
- イベント購読方式
  - 現状のイベントで動作可だが、Unity Test Framework のバージョン差異に注意。最小対応バージョンを README に明記し、将来的に `RegisterCallbacks` 実装へ移行検討。
- フィルタ意味の明確化
  - `Filter.testNames` は原則「完全一致」。計画の「部分一致」は将来対応。現状は README で明記。
- `ExtractAssemblyName` ヒューリスティクス
  - 現実装は誤判定の恐れあり。参考情報扱いとし、将来はメタ情報で補強。

## 低優先（改善）
- `status-<runId>.json` の併置
  - デバッグ容易性向上のため。Rust 側は `status.json` 優先で問題なし。
- スタックトレース抽出の正規表現化
  - 複数フォーマットに頑健。

## Rust 側レビュー
- ルータ/セキュリティ: 問題なし。
- 通知: `send_test_started_notification`/`send_test_finished_notification` は TODO。MVP の AC 充足のため、簡易 push 実装が必要。
- `generate_run_id` のテスト
  - 現状は固定文字列を検査しており実関数未使用。実関数を呼び、フォーマット検証に変更推奨。
- `canonicalize` の扱い
  - 存在しないパスでの `canonicalize()` は失敗。現実害は小さいが、親ディレクトリを起点に比較する方法がより厳格。

## 具体的修正案（パッチ指針）

Unity: McpTestRunner.cs
- Execute 修正:
  - `executionSettings.filters = new[] { filter };`
  - `testRunnerApi.Execute(executionSettings);`
- Status 型追加と出力修正:
  - ` [Serializable] class TestRunStatus { public string status; public string runId; public string timestamp; }`
  - `var s = new TestRunStatus { status = statusStr, runId = results.runId, timestamp = UtcNowIso() };`
  - `File.WriteAllText(statusPath, JsonUtility.ToJson(s, false));`
- 出力最適化/フィルタ:
  - `if (!request.includePassed) filteredResults = filteredResults.Where(r => r.status != "passed");`
  - `var limited = filteredResults.Take(request.maxItems).ToArray();`
  - `currentResults.truncated = limited.Length < filteredResults.Count();`
  - `currentResults.tests = limited;`
  - `JsonUtility.ToJson(results, false)` に変更。
- `mode=all`:
  - Edit と Play の2回実行で `collectedResults` を結合（時間は合算）。
- `targetPlatform`:
  - 削除またはプラットフォーム条件で分岐（MVP は削除）。

Rust: tests.rs
- 通知の実装（簡易）:
  - 既存のイベント送信チャンネルがあれば `unity.tests.started/finished` を送出。
- テストの修正:
  - `test_generate_run_id` で `generate_run_id()` を呼び、ISO8601 + `-xxxxxxxx` の形式を正規表現で検証。

## 受け入れ基準（AC）との照合
- AC1: 実行とサマリ返却 → OK（Unity 側出力 + Rust 側読取）。
- AC2: フィルタ反映 → Unity 側/または Rust 側で担保。Unity 側にも適用して出力量を抑制推奨。
- AC3: タイムアウト → Rust の `wait_for_test_completion` で担保。
- AC4: `unity_get_test_results` で最新/指定 ID 取得 → OK。
- AC5: `JsonUtility` 導入 → OK（匿名型修正が必要）。
- AC6: push 通知 → 未実装（要対応）。

## 修正チェックリスト
- [ ] McpTestRunner: `Execute` シグネチャ修正（filters 設定 → Execute）
- [ ] McpTestRunner: `TestRunStatus` 型追加と `status.json` を `JsonUtility` で出力
- [ ] McpTestRunner: `includePassed`/`maxItems` を JSON 出力前に適用
- [ ] McpTestRunner: `prettyPrint=false` に変更
- [ ] McpTestRunner: `targetPlatform` 固定削除
- [ ] McpTestRunner: `mode=all` を直列2回実行に変更（任意/推奨）
- [ ] Rust: 通知（started/finished）を実装
- [ ] Rust: `test_generate_run_id` を実装呼び出しベースに修正
- [ ] README: 最小対応 Unity バージョン、フィルタの完全一致注意点を追記

## 次アクション提案
1) クリティカル修正（Execute/Status/prettyPrint）を先行反映
2) Unity 側フィルタ/上限適用 → 出力縮小
3) 通知（push）実装 → AC6 完了
4) README 追記 → 最後に一括

---
このレビューの差分作成と適用をご希望であれば、次のターンでパッチを用意します。
