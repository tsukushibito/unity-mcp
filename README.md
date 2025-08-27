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
