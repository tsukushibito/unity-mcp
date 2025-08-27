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

方法A: 一時エディタスクリプトを実行（推奨）

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

方法B: スクリプト実行せずにC#コンソールから設定（任意）
- `EditorUserSettings.SetConfigValue("MCP.IpcToken", "test-token")` を実行します。

確認:
- `EditorIpcServer` 実装上、未設定/空のトークンや不一致は `UNAUTHENTICATED` で拒否されます。

## 4) Rust 例を実行

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
- `FAILED_PRECONDITION: schema mismatch` → C# 側の SCHEMA_HASH がRustと一致していません。CI/再生成手順を参照し更新してください。
- `FAILED_PRECONDITION: project_root mismatch` → Rust 側 `IpcHello.project_root` が Unity プロジェクト直下の絶対パスと一致しているか確認してください。
- `UNAVAILABLE: editor compiling/updating` → Unity のコンパイル/更新完了後に再試行。
- `tcp://127.0.0.1:7777` に接続不可 → Editor が起動しているか、ポートの占有/Firewallを確認。

## 参考
- 例コード: `server/examples/test_unity_ipc.rs`、`server/examples/unity_log_tail.rs`
- 機能フラグ: `server/src/ipc/features.rs`（`events.log` を含む）
- 追加の背景とタスク計画: `tasks/details/phase_C_dx.md`

