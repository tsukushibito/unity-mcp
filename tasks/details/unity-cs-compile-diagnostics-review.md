# Unity C# コンパイル診断機能 — 変更レビューと改善提案

- 日付: 2025-08-30
- 対象変更: Unity レポーター追加/更新、Rust MCP ツール追加、Cargo 設定変更
- 参照: `tasks/details/unity-cs-compile-diagnostics-mvp.md`, `tasks/details/unity-cs-compile-diagnostics-checklist.md`

## サマリー
- 目的どおり「Unity の C# コンパイル結果を MCP ツールで取得」する流れが実装開始済み。
- 重大/潜在的な問題をいくつか確認。Unity 側の API 誤用と JSON 依存は修正済み（このレビューに伴うパッチ）。
- 残課題は主に「配置（asmdef/Packages 化）」「Rust 側のパス解決とテスト整合」。

## 変更点（検出）
- Unity: `bridge/Assets/Editor/McpDiagnosticsReporter.cs` 追加
- Rust:
  - `server/src/mcp/tools/diagnostics.rs` 追加（新ツール実装）
  - `server/src/mcp/tools.rs` にツール登録 `unity_get_compile_diagnostics`
  - `Cargo.toml` の `tokio` に `fs` 機能追加、`dev-dependencies` に `tempfile`

## 評価（良い点）
- ツール設計はMVP方針に沿っており、要約/フィルタ/打ち切りの入り口が用意されている。
- Unity 側で `compile-<id>.json` と `latest.json` を出力する意図があり、将来拡張に適合。
- Rust 側レスポンス型は `serde`/`schemars` 準拠で拡張に耐える。

## 問題点と修正状況

### Unity 側
- API 誤用（修正済）
  - 誤り: `CompilationPipeline.GetLogEntries()`/`LogEntry` ベースの収集。
  - 対応: `assemblyCompilationFinished(string, CompilerMessage[])` で集約 → `compilationFinished` で一括書き出しに修正。
- 依存（修正済）
  - 変更: `Newtonsoft.Json` → Unity 標準 `JsonUtility` へ置換（追加依存不要）。
  - 影響: プロパティ非対応/Dictionary非対応だが、現行スキーマはフィールドのみのため影響なし。
- 指紋（修正済）
  - 変更: `.GetHashCode()` → `SHA256` に変更し安定化。
- URI 生成（修正済）
  - 変更: `new Uri(path).AbsoluteUri` を使用しクロスプラットフォームの一貫性を確保。
- アセンブリ名判定（未修正・要対応）
  - 現状: `ExtractAssemblyName` が `"/Assets/"` を先に判定するため `Assets/Editor` も `Assembly-CSharp` になり得る。
  - 提案: `Editor` 優先判定、または `assemblyCompilationFinished` の `assemblyName` を保存して使用。
- 配置（未対応・推奨変更）
  - 現状: `Assets/Editor/` 配下。`.meta` 未コミット。
  - 推奨: `Packages/com.example.mcp-bridge/Editor/` へ移動し、`Bridge.asmdef`（Editor only）を追加。代替として `Assets/MCP/Editor` + asmdef でも可。

### Rust 側
- パス解決（未修正・要対応）
  - 現状: `get_diagnostics_path()` が `cwd/bridge/Temp/AI/latest.json` 前提。
  - 問題: `cd server && cargo run` だと `server/bridge/...` を見に行き失敗。
  - 提案: `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".." )/bridge/Temp/AI/latest.json` を既定とし、`UNITY_MCP_DIAG_PATH` 環境変数で上書き可能に。
- セキュリティチェックとテストの整合（未修正・要対応）
  - 現状: ファイル読み込みで `bridge` 配下検証あり。一方、テストは `tempfile` に外部ファイルを作成し、意図しないエラー種別に。
  - 提案: テストでは `bridge` 配下に一時ディレクトリを作る／環境変数でパス差し替え／もしくはサービスに基底パスを注入できるようリファクタ。
- ファイルサイズ制限テスト（未修正・要対応）
  - 提案: 許可ディレクトリ内に 2MB 超の JSON を生成して検証。

## 直近で適用した修正（このレビューに伴う最小差分）
- Unity: `McpDiagnosticsReporter.cs`
  - `JsonUtility` に置換
  - `assemblyCompilationFinished` での `CompilerMessage` 集約
  - `SHA256` 指紋、`AbsoluteUri` 生成

## 残アクション（推奨対応順）
1) Unity: 配置再構成
- [ ] `Assets/Editor/McpDiagnosticsReporter.cs` → `Packages/com.example.mcp-bridge/Editor/`
- [ ] `Bridge.asmdef`（Editor only）追加、`.meta` をコミット

2) Unity: アセンブリ名の正確化
- [ ] `assemblyCompilationFinished` の `assemblyName` を保持し、各 `CompilerMessage` と紐づけて `Diagnostic.assembly` に採用
- [ ] 代替: `ExtractAssemblyName` の判定順を `Editor` 優先に修正

3) Rust: パス解決の堅牢化
- [ ] `get_diagnostics_path()` を `CARGO_MANIFEST_DIR/../bridge/...` 基準へ
- [ ] `UNITY_MCP_DIAG_PATH` で上書き可能に（ドキュメント追記）

4) Rust: テスト整備
- [ ] セキュリティ検証とファイルサイズ制限テストを `bridge` 配下で実施
- [ ] 既存テストのエラー種別期待を修正（"Access denied" とサイズ超過の切り分け）

5) ドキュメント/開発者体験
- [ ] README に `latest.json` の既定パスと環境変数オーバーライドを追記
- [ ] `AGENTS.md`/`CLAUDE.md` に新ツールの説明を追加

## リスク/注意点
- `JsonUtility` は `null` フィールドを省略しがち。Rust 側は `Option<>` で受けるため許容だが、厳密比較テストでは差異に留意。
- 将来の差分検出（`changed_only`）実装時は、Unity 側の `previous.json` 読み込み/マージまたは Rust 側で指紋ベースの差分抽出が必要。
- Windows 環境ではパス区切り/URI の一貫性に注意。`AbsoluteUri` 採用で大半は吸収できる想定。

## 更新後の受け入れ基準（補足）
- Unity: コンパイル後に `bridge/Temp/AI/latest.json` が `JsonUtility` 形式で生成される
- Rust: ツール `unity.get_compile_diagnostics` が `severity`/`max_items`/`assembly` を適用し、`truncated` を返す
- 配置: Editor 専用コードが Editor 専用 asmdef に属している
- 実行パスの差異（`cd server` か否か）に関わらず診断ファイルを解決できる

