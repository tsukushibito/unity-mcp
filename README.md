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
3) Rust 例を実行

```sh
cd server
cargo run --example test_unity_ipc
cargo run --example unity_log_tail
```

詳細手順、トラブルシュート、期待される出力は Quickstart を参照してください。
