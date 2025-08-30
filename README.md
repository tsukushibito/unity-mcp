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
