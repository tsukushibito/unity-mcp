# unity-mcp
Unity MCP Server

## 開発セットアップ

クローン直後に Git Hooks を有効化してください。これにより、コミット時に `server/` のフォーマットチェック（rustfmt）とリンティング（clippy）が自動実行されます。

- Linux/macOS（シェル）:

  ```sh
  ./scripts/bootstrap-hooks.sh
  ```

- Windows（PowerShell）:

  ```powershell
  .\scripts\bootstrap-hooks.ps1
  ```

上記スクリプトは `git config core.hooksPath .githooks` を設定し、POSIX 環境ではフックに実行権限を付与します。

## Quickstart（15分E2E）

- ガイド: `docs/quickstart.md`
- ゴール: Unity Editor を起動し、`MCP.IpcToken` を設定した上で、Rust 例 `test_unity_ipc` と `unity_log_tail` を実行して疎通とログ受信を確認します。

最短手順（要約）

1) Unity で `bridge/` を開く（Editor 起動）
2) `MCP.IpcToken` を設定（例: `test-token`）
   - Unity: `Edit > Project Settings... > MCP Bridge` で Token を入力（または `MCP Bridge/Setup/Open Project Settings`）
3) Rust 例を実行

```sh
cd server
cargo run --example test_unity_ipc
cargo run --example unity_log_tail
```

詳細手順、トラブルシュート、期待される出力は Quickstart を参照してください。

## Unity C# コンパイル診断機能

Unity の C# コンパイル結果（エラー、警告、情報）を MCP ツールで取得できます。

### 基本動作

1. Unity でスクリプトを変更・保存してコンパイルを実行
2. 診断結果が `bridge/Temp/AI/latest.json` に JSON 形式で出力
3. MCP クライアントから `unity.get_compile_diagnostics` ツールで取得・フィルタ

### 環境変数設定（オプション）

診断ファイルのパスをカスタマイズできます：

```bash
# デフォルトパスを変更したい場合
export UNITY_MCP_DIAG_PATH="/custom/path/to/diagnostics.json"
cd server && cargo run
```

### ツール使用例

```json
{
  "name": "unity.get_compile_diagnostics",
  "arguments": {
    "severity": "error",
    "max_items": 100,
    "assembly": "Assembly-CSharp"
  }
}
```

パラメータ:
- `severity`: `"error"`, `"warning"`, `"info"`, `"all"` (デフォルト: `"all"`)
- `max_items`: 取得件数上限 (デフォルト: 500)
- `assembly`: アセンブリ名でフィルタ (例: `"Assembly-CSharp"`)
- `changed_only`: 直近変化分のみ (将来実装予定)

## Unity TestRunner 実行機能

Unity の EditMode/PlayMode テストを MCP ツールで実行し、結果を取得できます。

**最小対応 Unity バージョン**: Unity 2019.4 LTS 以降（Unity Test Framework 1.1.0 以降）

### 基本動作

1. Rust MCP サーバーから `unity_run_tests` でテスト実行をトリガ
2. Unity Editor の `McpTestRunner` がリクエストを検知し、`TestRunnerApi` でテスト実行
3. 結果が `bridge/Temp/AI/tests/latest.json` に JSON 形式で出力
4. MCP クライアントから結果取得・フィルタ

補足:
- `mode = "all"` の場合、EditMode → PlayMode を直列に実行し、結果を結合して1つのランとして保存します。
- ステータスファイルは `status.json` に加えて、デバッグ容易化のため `status-<runId>.json` も併置します。

### 環境変数設定（オプション）

テストリクエスト・結果ファイルのパスをカスタマイズできます：

```bash
# リクエストファイルの配置先
export UNITY_MCP_REQ_PATH="/custom/path/to/requests"

# テスト結果ファイルの配置先  
export UNITY_MCP_TESTS_PATH="/custom/path/to/tests"

cd server && cargo run
```

### ツール使用例

**テスト実行:**
```json
{
  "name": "unity_run_tests",
  "arguments": {
    "mode": "edit",
    "test_filter": "PlayerService*",
    "categories": ["fast"],
    "timeout_sec": 300,
    "max_items": 1000,
    "include_passed": true
  }
}
```

**結果取得:**
```json
{
  "name": "unity_get_test_results", 
  "arguments": {
    "run_id": "2025-08-30T12:00:00Z-abc12345",
    "max_items": 500,
    "include_passed": false
  }
}
```

パラメータ:

**unity_run_tests:**
- `mode`: `"edit"`, `"play"`, `"all"` (デフォルト: `"edit"`)
- `test_filter`: テスト名フィルタ (任意)
  - **注意**: Unity TestRunnerApi は完全一致のみサポート。部分一致は将来対応予定
- `categories`: カテゴリ配列 (任意, OR条件)  
- `timeout_sec`: タイムアウト時間 (デフォルト: 180)
- `max_items`: 結果件数上限 (デフォルト: 2000)
- `include_passed`: 成功テストも含める (デフォルト: true)

**unity_get_test_results:**
- `run_id`: 特定の実行IDを指定 (省略時は `latest.json`)
- `max_items`: 結果件数上限 (デフォルト: 2000) 
- `include_passed`: 成功テストも含める (デフォルト: true)

### 実装上の注意

- **assembly名**: テスト結果のassembly名は完全な名前から推定したものです。参考情報として扱ってください
- **file/line情報**: スタックトレースから抽出。取得できない場合は空欄になります
- **通知機能**: 現状はログ出力のみ。必要に応じて `unity.tests.started` / `unity.tests.finished` のMCP通知を追加予定
