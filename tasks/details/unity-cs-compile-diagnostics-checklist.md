# Unity C# コンパイル診断 — 作業チェックリスト

- 作成日: 2025-08-30
- 対象: unity-mcp（Rust MCP サーバー + Unity Editor ブリッジ）
- 参考: `tasks/details/unity-cs-compile-diagnostics-mvp.md`

## 全体
- [x] 設計ドキュメント（MVP）を追加（本機能の目的/範囲/受け入れ基準）
- [ ] 実装ブランチ作成（例: `feat/unity-diagnostics-mvp`）
- [ ] 実装完了後、PR 作成（Conventional Commits / 説明・スクショ）

## Unity 側（bridge/）
- [ ] `Assets/Editor/McpDiagnosticsReporter.cs` 追加
- [ ] `CompilationPipeline.compilationStarted/Finished` を購読
- [ ] `assemblyCompilationFinished` で `CompilerMessage[]` を収集
- [ ] LSP 風スキーマへ変換（file_uri/range/severity/message/code/assembly/source/fingerprint）
- [ ] JSON 出力先を `bridge/Temp/AI/latest.json` に固定
- [ ] `compile-<id>.json`（ID 付き）も保存（将来の分割出力に備える）
- [ ] パスの URI 化（`Application.dataPath` を基準に `file://`）
- [ ] 列情報欠落時は `character:0` を設定
- [ ] 大量メッセージのハンドリング（暫定: 全件出力。分割は Phase 3）
- [ ] ログ/例外処理（`Debug.LogError`）

## Rust MCP サーバー側（server/）
- [ ] `server/src/mcp/tools/diagnostics.rs` を追加
- [ ] `McpService` に `do_unity_get_compile_diagnostics(...)` 実装
- [ ] 入力: `max_items`/`severity`/`changed_only`/`assembly` を受理
- [ ] 出力: `compile_id`/`summary`/`diagnostics[]`/`truncated` を返却
- [ ] `bridge/Temp/AI/latest.json` を読み込み（存在チェック/エラーハンドリング）
- [ ] フィルタ（severity/assembly）、上限適用、要約集計
- [ ] セキュリティ: `bridge/` 配下以外のパスをマスク/拒否
- [ ] レスポンスサイズ上限（~2MB）を超える場合は打ち切り + `truncated=true`
- [ ] `tracing` ログで詳細を記録、ユーザ向けエラーメッセージは簡潔に

## MCP ツール登録/スキーマ
- [ ] `unity.get_compile_diagnostics` をツール登録
- [ ] ツールの引数/戻り値スキーマを宣言（`rmcp` の型に準拠）
- [ ] 既存ツール一覧・ヘルスチェックに影響がないことを確認

## テスト
- [ ] サンプル JSON（小/中/大）を `server/tests/data/` に追加
- [ ] 正常系テスト: フィルタ/要約/上限の整合性
- [ ] エラー系テスト: ファイル未存在/不正 JSON
- [ ] 文字数・サイズ境界の打ち切り挙動
- [ ] `cargo test` がグリーン

## 品質（静的解析/スタイル）
- [ ] `cd server && cargo fmt --all` がパス
- [ ] `cd server && cargo clippy --all-targets -- -D warnings` がパス
- [ ] C# 側は EditorConfig/Unity 既定スタイルに準拠

## 手動確認（Unity Editor）
- [ ] 意図的にエラー/警告を発生させ `Temp/AI/latest.json` 生成を確認
- [ ] 実際のファイル/行が正しい URI/range で出力される
- [ ] MCP クライアントからの取得でフィルタ/要約が反映
- [ ] `max_items` 指定時に `truncated` が true になるシナリオ

## ドキュメント/開発者体験
- [ ] README に「使い方（Unity 側 JSON 出力と MCP ツール呼び出し）」を追記
- [ ] `docs/rust-coding-guidelines.md` にエラー処理方針の補足が必要か確認
- [ ] `AGENTS.md`/`CLAUDE.md` に新ツールの説明を追記

## 受け入れ基準（MVP Done）
- [ ] Unity 変更→コンパイル→`bridge/Temp/AI/latest.json` が更新
- [ ] `unity.get_compile_diagnostics` が存在し、`severity`/`max_items`/`assembly` が機能
- [ ] `summary` が Unity コンソールと一致し、`truncated` が仕様通り
- [ ] `fmt`/`clippy`/`test` が通過

## フェーズ 2（バックログ）
- [ ] MCP: `unity.trigger_compile` 追加
- [ ] gRPC: Unity→Rust `ReportCompilation`（push）実装
- [ ] MCP 通知 `compile.updated` を発火
- [ ] `notify` による `bridge/Temp/AI/` 監視（キャッシュ更新）

## フェーズ 3（バックログ）
- [ ] `unity.get_compile_artifact`（大規模アーティファクト分割取得）
- [ ] 指紋（`fingerprint`）による差分抽出（新規/解消のみ）
- [ ] サイズ/パフォーマンス最適化（ページング、ストリーミング）

---

### 参考コマンド
- `cd server && cargo build`
- `cd server && cargo run`
- `cd server && cargo test`
- `cd server && cargo fmt --all`
- `cd server && cargo clippy --all-targets -- -D warnings`

