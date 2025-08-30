# Unity C# コンパイル結果を AI に渡す機能 — 作業方針と作業内容（MVP）

- 作成日: 2025-08-30
- 対象: unity-mcp（Rust MCP サーバー + Unity Editor ブリッジ）
- 目的: Unity の C# コンパイル診断（エラー/警告/情報）を構造化し、MCP ツール経由で AI クライアントが取得・活用できるようにする。

## 背景 / 問題
- 現状、Unity のコンパイル結果は Editor コンソール/ログに閉じており、AI ワークフローから直接参照・要約・修正提案に活用しにくい。
- AI ループに取り込むためには、（1）イベントフック、（2）機械可読な出力、（3）安全な取得 API が必要。

## 目標（MVP）
- Unity Editor のコンパイル完了時に診断を JSON へ出力（ローカル、安定パス）。
- Rust MCP サーバーに取得系ツール `unity.get_compile_diagnostics` を実装し、要約とフィルタ提供。
- 大量結果に対し件数制限と打ち切りフラグを返す。

## 非目標（MVP 外）
- Unity からの push ストリーミング／通知（Phase 2 で検討）。
- 強制リビルドのトリガ（`trigger_compile` は Phase 2）。
- 生ログの断片取得やアーティファクト分割 API（Phase 3）。

## 全体設計（MVP）
- フロー:
  1. Unity コンパイル → `CompilationPipeline` イベントで `CompilerMessage` を収集。
  2. LSP 風スキーマへマッピングし、`bridge/Temp/AI/latest.json` へ出力。
  3. Rust MCP ツールが JSON を読み、入力条件でフィルタ・要約して返却。
- Pull モデル: クライアントは必要時に `unity.get_compile_diagnostics` を呼び出す。
- 依存関係: gRPC/Proto の追加は不要（MVP）。

## MCP ツール定義（MVP）
- 名称: `unity.get_compile_diagnostics`
- 入力（すべて任意）:
  - `max_items:number` 既定 500
  - `severity:string` "error"|"warning"|"info"|"all"（既定 "all"）
  - `changed_only:boolean` 直近で変化/新規のみ（既定 false）
  - `assembly:string` 例 "Assembly-CSharp"
- 出力:
  - `compile_id:string`（例: epoch ベース）
  - `summary:{ errors:number, warnings:number, infos:number, assemblies:string[] }`
  - `diagnostics: Diagnostic[]`（下記）
  - `truncated:boolean`（`max_items` で打ち切り時 true）
- Diagnostic スキーマ（LSP 風）:
  - `file_uri:string`（絶対パス→URI 推奨）
  - `range:{ start:{line,character}, end:{line,character} }`
  - `severity:"error"|"warning"|"info"`
  - `message:string`
  - `code?:string`（Roslyn エラーコードが取れた場合）
  - `assembly:string`
  - `source:string`（固定 "Unity"）
  - `fingerprint:string`（`hash(file|line|message|assembly)` など）
  - `first_seen?`/`last_seen?`: ISO8601（拡張時に活用）

### リクエスト/レスポンス例
```json
{
  "name": "unity.get_compile_diagnostics",
  "arguments": {
    "severity": "error",
    "max_items": 200
  }
}
```
```json
{
  "compile_id": "1724985600",
  "summary": {"errors": 3, "warnings": 12, "infos": 0, "assemblies": ["Assembly-CSharp"]},
  "diagnostics": [
    {
      "file_uri": "file:///workspaces/unity-mcp/bridge/Assets/Scripts/Foo.cs",
      "range": {"start": {"line": 14, "character": 9}, "end": {"line": 14, "character": 9}},
      "severity": "error",
      "message": "The name 'bar' does not exist in the current context",
      "code": "CS0103",
      "assembly": "Assembly-CSharp",
      "source": "Unity",
      "fingerprint": "abc123"
    }
  ],
  "truncated": false
}
```

## Unity 側実装（bridge/）
- 追加ファイル: `Assets/Editor/McpDiagnosticsReporter.cs`
- 主要処理:
  - `CompilationPipeline.compilationStarted/compilationFinished` を購読し開始/終了時刻を記録。
  - `assemblyCompilationFinished` で `CompilerMessage[]` を収集して正規化。
  - JSON 出力先: `bridge/Temp/AI/latest.json`（ID 付きは `compile-<id>.json`）。
  - パスは `Application.dataPath` 基準で `file://` URI を生成。列情報が無い場合は `character:0`。
- 留意事項:
  - Play Mode 中の auto-recompile 状況に応じてガードまたはその旨を `summary` に記録。
  - 大量メッセージ時は分割出力の余地を残す（Phase 3）。

## Rust サーバー実装（server/）
- 追加/変更箇所:
  - `server/src/mcp/tools/` に新規 `diagnostics.rs`（または既存モジュールへ追加）。
  - `McpService` にハンドラ `do_unity_get_compile_diagnostics(...)` を追加。
  - 型: `anyhow::Result` と `?` を使用。`serde` で JSON をパース。
- 動作:
  - `bridge/Temp/AI/latest.json` 読み込み→入力条件でフィルタ→`CallToolResult` で返却。
  - `diagnostics.len() > max_items` で `truncated=true`。
- エラー応答:
  - ファイル未生成: ユーザ向けに「Unity で一度スクリプト保存/再コンパイル」を案内。
  - JSON 不正: `internal_error` で詳細を `tracing` に記録。

## セキュリティ/プライバシー
- パス制御: `bridge/` 配下のみ返却（ワークスペース外はマスク）。
- サイズ上限: レスポンス最大 ~2MB（超過時は打ち切り + 将来のアーティファクト API 案内）。
- 機密: メッセージ内に環境ユーザー名等が含まれる場合は省略/ハッシュ化オプション（将来）。

## テスト計画
- ユニットテスト（Rust）:
  - 正常系: JSON サンプルを読み込み、フィルタ/要約の整合性を検証。
  - 打ち切り: `max_items` 指定で `truncated` が期待通りになること。
  - 未存在: `latest.json` 不在時にユーザ向け説明を含むエラーを返すこと。
- 手動確認（Unity）:
  - 意図的にエラー/警告を発生させ、`Temp/AI/latest.json` の内容を確認。
  - MCP クライアントからツール呼び出し→件数/フィルタ/要約が反映されること。

## 受け入れ基準（MVP Done）
- Unity でコード変更→コンパイル→`bridge/Temp/AI/latest.json` が生成・更新される。
- MCP ツール `unity.get_compile_diagnostics` が存在し、`severity`/`max_items`/`assembly` でフィルタ可能。
- `summary` の件数が Unity コンソールと一致し、`truncated` が仕様通りに動作。
- `cargo fmt`/`clippy` が通過し、ドキュメント（本ファイル）がリポジトリに追加されている。

## 実装ステップ（MVP）
1. Unity Editor スクリプト雛形追加（イベント購読・JSON 出力）。
2. Rust MCP ツール骨組み追加（ハンドラ + JSON 読み込み）。
3. ツール登録/スキーマ宣言を既存 MCP 登録箇所に組み込み。
4. サンプル JSON とユニットテストを追加（server/tests/ もしくはユニット）。
5. `cargo fmt`/`clippy`/`cargo test` 実行。

## 将来拡張（Phase 2/3 概要）
- Phase 2: `unity.trigger_compile`、Unity→Rust の `ReportCompilation` push、MCP 通知 `compile.updated`。
- Phase 3: アーティファクト分割取得 `unity.get_compile_artifact`、差分抽出（`fingerprint` ベース）、「新規/解消のみ」取得。

## 変更ファイル/追加予定
- bridge: `Assets/Editor/McpDiagnosticsReporter.cs`
- server: `src/mcp/tools/diagnostics.rs`（新規）、`src/mcp/service.rs`（登録）
- tasks: 本ドキュメント

