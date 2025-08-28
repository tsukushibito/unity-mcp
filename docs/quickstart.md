# Quickstart — Unity MCP Server (15分でE2E)

この手順では、新規クローンから Unity Editor と Rust サンプルを使って、接続（Handshake）・ヘルスチェック・ログ購読の最小E2Eを15分以内で再現します。

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
- 起動後、エディタログに `EditorIpcServer` の待受ログ（127.0.0.1:7777）が出力されることを確認します。

## 3) `MCP.IpcToken` を設定（必須）
Unity 側は `EditorUserSettings["MCP.IpcToken"]` のみを参照します（環境変数や `EditorPrefs` は無視）。

- 方法A（推奨）: `Edit > Project Settings... > MCP Bridge` で Token を設定
- 方法B: 以下の一時エディタスクリプトを実行

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

確認: 未設定/空のトークンや不一致は `UNAUTHENTICATED` で拒否されます。

## 4) Rust サンプルを実行

T01 ハンドシェイクから `project_root` は削除済みです。追加の環境変数設定は不要です。

```sh
cd server
cargo run --example test_unity_ipc
```

期待される出力（例）:
- `✓ Successfully connected`
- `✓ Handshake completed`
- `✓ Health response received`

続いて、Unity ログを tail します。

```sh
cargo run --example unity_log_tail
```

期待される動作:
- 10秒間 `events.log` のログイベントを購読し、受信件数とレベル別集計（info/warn/error）を表示。
- 終了時にサマリを表示。`error > 0` の場合は終了コード 1 で終了。

## トラブルシュート
- `UNAUTHENTICATED: Missing or empty token` → `EditorUserSettings["MCP.IpcToken"]` を設定（上記参照）。
- `FAILED_PRECONDITION: schema mismatch` → C# 側の SCHEMA_HASH が Rust と一致していません。CI/再生成手順で更新してください。
- `FAILED_PRECONDITION: project_root mismatch` → 旧版が混在しています。Rust/Unity の両方を最新版に更新してください（現行T01では `project_root` を使用しません）。
- `UNAVAILABLE: editor compiling/updating` → Unity のコンパイル/更新完了後に再試行。
- `tcp://127.0.0.1:7777` に接続不可 → Editor が起動しているか、ポート占有/Firewall を確認。

## 参考
- 例コード: `server/examples/test_unity_ipc.rs`、`server/examples/unity_log_tail.rs`
- 機能フラグ: `server/src/ipc/features.rs`（`events.log` を含む）
- 追加の背景: `docs/unity_mcp_server_architecture_direct_ipc_variant.md`

