# フェーズA 実装レビュー（2025-08-27）

本書は、フェーズA（ハンドシェイク不変条件）の実装差分レビュー結果を整理したものです。CI、C#生成、Unity側実装、テスト、リスク/改善提案を含みます。

## 変更サマリ（確認済み）
- CI: `.github/workflows/ci.yml` に `Schema Hash Parity Check` ジョブを追加。
  - Rust再生成→生成物ドリフト検出→`schema.pb`のSHA-256算出→C# `SCHEMA_HASH_HEX`抽出→比較→C#生成物ドリフト検出。
- 生成スクリプト: `bridge/Tools/generate-csharp.sh`
  - Rustの `server/src/generated/schema_hash.rs` から [u8;32] を抽出し、HEX/bytes化して `SchemaHash.cs` を生成。
  - 出力: `bridge/Packages/com.example.mcp-bridge/Editor/Generated/SchemaHash.cs`
  - 名前空間: `Mcp.Unity.V1.Generated`、型: `internal static class Schema`
- Unity側ハンドシェイク: `EditorIpcServer.cs`
  - `ValidateSchemaHash` を導入。不一致/欠落は `FAILED_PRECONDITION` でReject。メッセージはDoD準拠。
  - `Welcome.SchemaHash` を `Generated.Schema.SchemaHashBytes` に切替。
  - トークン方針をEditorUserSettingsのみ＋No Dev Modeに変更。ガイダンス文面を1行で提示。
- テスト
  - Unity: `HandshakeTests.cs` にスキーマハッシュの基本性質テストを追加。
  - Rust: `test_schema_hash_mismatch_rejection` を追加（ミスマッチ拒否を検証）。

## 要修正（優先度: 高）
- Unity APIのスレッドセーフティ（EditorUserSettingsの取得）
  - 現状: `ValidateToken` 内で `EditorUserSettings.GetConfigValue("MCP.IpcToken")` をメインスレッド外で呼び出す可能性あり（`RunOnMainAsync` 前段）。
  - リスク: Unity Editor APIはメインスレッド限定のため例外/未定義動作の恐れ。
  - 対応案（いずれか）:
    1) メインスレッド内で期待トークンを取得し、値のみを非同期へ引き渡して `ValidateToken(expectedToken, clientToken)` で比較（`ValidateToken` はUnity API非依存化）。
    2) 検証前に `expectedToken = await EditorDispatcher.RunOnMainAsync(() => EditorUserSettings.GetConfigValue("MCP.IpcToken"))` で取得。

- Rust統合テストのトリガ条件の責務分離
  - 現状: スキーマミスマッチを `hello.token == "wrong-schema-hash"` で分岐させている。
  - リスク: トークンとスキーマの責務が混線し、将来の可読性/保守性に影響。
  - 対応案:
    - 送信直前のHelloの `schema_hash` を偽値に置換するテストヘルパを用意（バイト列書き換えなど）。
    - もしくは `IpcClient` にテスト専用オプション（例: `override_schema_hash_for_test`）を追加（本番ビルドでは無効）。

## 改善提案（任意）
- CIジョブ名の統一: ドキュメントと合わせて `Proto & Schema Parity Check` にすると参照性が向上。
- 生成スクリプトの頑健性:
  - `schema_hash.rs` の配列抽出は `grep -o '\\[[0-9, ]*\\]'` に依存。将来の整形変更に備え、`SCHEMA_HASH` 定義行のスコープを限定して抽出、または `server/src/generated/schema.pb` の SHA-256 を直接HEX化して利用すると堅牢。
- Unityテストの追加:
  - `ValidateSchemaHash` の分岐（empty / length mismatch / byte mismatch）を実際に通すユニットテストを追加し、回帰耐性を強化。

## CI 実装のポイント（確認用）
- 比較元:
  - Rust: `sha256sum server/src/generated/schema.pb` → HEX（小文字）
  - C#: `bridge/Packages/com.example.mcp-bridge/Editor/Generated/SchemaHash.cs` の `SCHEMA_HASH_HEX`
- 検証順序:
  1) Rust proto再生成 → `git diff --exit-code server/src/generated/`
  2) Rust HEX算出
  3) C# HEX抽出
  4) 比較 → 不一致で失敗（再生成手順を案内）
- 追加先: `.github/workflows/ci.yml` に新規ジョブとして追加（ubuntu-latest）。既存 `build-test` に `if: matrix.os == 'ubuntu-latest'` で組み込む代替も可。

## エラーメッセージ（現行）
- トークン未設定/空: `UNAUTHENTICATED` / `Missing or empty token. Set EditorUserSettings: MCP.IpcToken`
- トークン不一致: `UNAUTHENTICATED` / `Invalid token. Check EditorUserSettings: MCP.IpcToken`
- スキーマ不一致: `FAILED_PRECONDITION` / `Schema hash mismatch. Regenerate C# SCHEMA_HASH from server (CI).`

## 次アクション（提案）
1) メインスレッド内でのトークン取得へリファクタ（または事前取得）。
2) Rust統合テストからトークン依存の分岐を除去し、`schema_hash` 書き換え方式に変更。
3) （任意）CIジョブ名の表示調整とスクリプト抽出処理の堅牢化。
4) Unity側ユニットテストを1件追加し、`ValidateSchemaHash` の各分岐を網羅。

参照
- `tasks/details/phase_A_handshake.md`
- `tasks/details/phase_B_ci_ssot.md`
- `.github/workflows/ci.yml`
- `bridge/Tools/generate-csharp.sh`
- `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/EditorIpcServer.cs`
- `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/Tests/HandshakeTests.cs`
- `server/tests/ipc_integration.rs`
