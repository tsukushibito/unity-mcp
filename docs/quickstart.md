# Quickstart — Unity MCP Server (15分でE2E)

この手順では、新規クローンから Unity Editor と Rust 例を使って、接続（Handshake）とヘルスチェック、ログイベント受信の最小E2Eを15分以内で再現します。

## 前提
- Unity Editor がインストール済み（LTS/最新版のいずれか）。
- Rust toolchain（stable）と `cargo` が利用可能。

## 1) クローンと初期セットアップ

```sh
git clone <this-repo>
cd unity-mcp
./scripts/bootstrap-hooks.sh  # Windows: .\\scripts\\bootstrap-hooks.ps1
```

## 2) Unity で `bridge/` を開く
- Unity Hub から `bridge/` を開いてプロジェクトを起動します。
- 起動後にエディタログへ `EditorIpcServer` の待受ログが出ることを確認します（127.0.0.1:7777）。

## 3) `MCP.IpcToken` を設定（必須）
Unity 側は `EditorUserSettings["MCP.IpcToken"]` のみを参照します（環境変数や `EditorPrefs` は無視されます）。

方法A: Project Settings から設定（推奨）

- Unity メニューから `MCP Bridge/Setup/Open Project Settings` を選択、または `Edit > Project Settings... > MCP Bridge` を開きます。
- `Token` フィールドに値（例: `test-token`）を入力して保存されます（自動）。

方法B: 一時エディタスクリプトを実行

1. 任意のエディタスクリプト（例）を作成して実行します。

```csharp
// Assets/Editor/SetIpcToken.cs
using UnityEditor;
using UnityEngine;

public static class SetIpcToken
{
    [MenuItem("MCP Bridge/Setup/Set Test Token")]
    public static void SetToken()
    {
        EditorUserSettings.SetConfigValue("MCP.IpcToken", "test-token");
        Debug.Log("[Quickstart] Set MCP.IpcToken = test-token");
    }
}
```

2. Unity のメニューから `MCP Bridge/Setup/Set Test Token` を実行します。

方法C: スクリプト実行せずにC#コンソールから設定（任意）
- `EditorUserSettings.SetConfigValue("MCP.IpcToken", "test-token")` を実行します。

確認:
- `EditorIpcServer` 実装上、未設定/空のトークンや不一致は `UNAUTHENTICATED` で拒否されます。

## 4) `MCP_PROJECT_ROOT` を設定（推奨）
Rust クライアントは `IpcHello.project_root` を Unity のプロジェクトルートの絶対パスと一致させる必要があります。サンプルは `MCP_PROJECT_ROOT` が設定されていればそれを使用し、未設定時はカレントディレクトリ（`.`）を正規化して送信します。

- 典型的には、Unity で開いているのはリポジトリ内の `bridge/` フォルダです。絶対パスを指定してください。

Windows (PowerShell):

```powershell
$env:MCP_PROJECT_ROOT = 'C:\path\to\unity-mcp\bridge'
# or, safer (resolves absolute path):
$env:MCP_PROJECT_ROOT = (Resolve-Path ..\bridge).Path
```

macOS/Linux (bash/zsh):

```sh
export MCP_PROJECT_ROOT="/path/to/unity-mcp/bridge"
```

Note (Windows): PowerShell ではバックスラッシュはエスケープではありません（エスケープはバッククォート ` ）。`C:\path\to` のように通常の 1 本の `\` を使ってください。Rust クライアント側でパスは正規化され、必要に応じて `\\?\` プレフィックスも除去されます。

代替（環境変数なしで実行）:
- `bridge/` をカレントにして `--manifest-path` でサンプルを実行すると、`.` がプロジェクトルートになります。

```powershell
cd bridge
cargo run --manifest-path ..\server\Cargo.toml --example test_unity_ipc
```

```sh
cd bridge
cargo run --manifest-path ../server/Cargo.toml --example test_unity_ipc
```

## 5) Rust 例を実行

ターミナルで以下を実行:

```sh
cd server
cargo run --example test_unity_ipc
```

期待される出力（例）:
- `✓ Successfully connected`、`✓ Handshake completed`、`✓ Health response received`

続いて、ログtail例でUnityログを取得:

```sh
cargo run --example unity_log_tail
```

期待される動作:
- 10秒間 `events.log` のログイベントを購読し、受信件数とレベル別集計（info/warn/error）を出力。
- 終了時にサマリを表示。`error>0` の場合は終了コード 1 で終了。

## トラブルシュート
- `UNAUTHENTICATED: Missing or empty token` → `EditorUserSettings["MCP.IpcToken"]` を設定（上記参照）。
- `FAILED_PRECONDITION: schema mismatch` → C# 側の SCHEMA_HASH が Rust と一致していません。CI/再生成手順を参照し更新してください。
- `FAILED_PRECONDITION: project_root mismatch` → 以下を確認してください。
  - `MCP_PROJECT_ROOT` が Unity で開いているプロジェクト（例: `bridge/`）の絶対パスになっている
  - または `bridge/` をカレントにして `--manifest-path` で実行している
  - シンボリックリンク/ショートカットではなく実パスで一致している（内部で正規化・大文字小文字無視で比較）
- `UNAVAILABLE: editor compiling/updating` → Unity のコンパイル/更新完了後に再試行。
- `tcp://127.0.0.1:7777` に接続不可 → Editor が起動しているか、ポートの占有/Firewallを確認。

## 参考
- 例コード: `server/examples/test_unity_ipc.rs`、`server/examples/unity_log_tail.rs`
- 機能フラグ: `server/src/ipc/features.rs`（`events.log` を含む）
- 追加の背景とタスク計画: `tasks/details/phase_C_dx.md`
