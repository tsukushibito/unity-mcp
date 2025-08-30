# MCP ツール一覧が空になる問題の修正作業書

## 背景 / 症状
- MCP Inspector で `tools/list` を呼ぶと `{"tools": []}` になる。
- サーバーは `tools` capability を公開しているが、実ツールが 0 件として扱われている。

## 根本原因
- `#[tool_router]` が付与された `impl McpService` ブロック（`src/mcp/service.rs`）内に `#[tool]` 関数が存在しない。
- 実際の `#[tool]` 関数は `src/mcp/tools/*.rs` の別 `impl McpService` ブロックに定義されている。
- rmcp の `#[tool_router]` は「同一の impl ブロック内の `#[tool]` メソッド」を収集してルーターを生成するため、現在は登録 0 件となり `tools/list` が空になる。

## 修正方針（推奨）
1) `#[tool]` アノテーションが付くメソッド群を、単一の `#[tool_router] impl McpService` ブロック内に集約する。
2) 既存のロジックはモジュールに残し、`#[tool]` メソッドは薄いラッパとして内部ヘルパーを呼び出す（＝関数名の衝突を避け、見通しを維持）。

備考: Unity Bridge への接続は実行時に `require_ipc()` で判定する設計を維持する。`tools/list` は接続の有無に関わらず非空になるのが正。

## 変更スコープ
- コード移動/名称変更のみ。外部 API（ツール名・引数・戻り値の JSON 形状）は不変。
- テスト・ドキュメントの最小更新を含む。

## 具体的手順
1. ツール洗い出し
   - `src/mcp/tools/health.rs` の `unity_health`
   - `src/mcp/tools/status.rs` の `unity_bridge_status`
   - `src/mcp/tools/assets.rs` の `unity_assets_*` 一式（import/move/delete/refresh/guid_to_path/path_to_guid）

2. 内部ヘルパー化（衝突回避）
   - 各ツール実装を `impl McpService` から外し、同モジュール内のプライベート関数（例: `pub(super) async fn do_unity_health(..)`）へ退避、または既存メソッド名を `do_*` にリネームし `#[tool]` は外す。
   - シグネチャ（引数/戻り値）はそのまま流用可能な形で保持。

3. ルーター集中定義
   - `src/mcp/tools.rs` に「単一の」`#[tool_router] impl McpService` を新設し、そこに全 `#[tool]` メソッドを定義。
   - 各 `#[tool]` メソッドは前項のヘルパーを呼び出すだけの薄いラッパにする。
   - 例: `#[tool] pub async fn unity_health(&self) -> Result<CallToolResult, McpError> { tools::health::do_unity_health(self).await }`

4. 既存参照の維持
   - `src/mcp/service.rs` の `McpService::new()` は現状どおり `tool_router: Self::tool_router(),` を使用（`#[tool_router]` を付けた impl によって生成される関連関数）。

5. ビルドと整形
   - `cd server && cargo fmt --all`
   - `cd server && cargo clippy --all-targets -- -D warnings`
   - `cd server && cargo build`

6. 動作検証
   - `RUST_LOG=server=debug cargo run` で起動。
   - MCP Inspector から接続し、`tools/list` が非空になることを確認。
   - `tools/call` で `unity_bridge_status` が接続の有無に関わらず成功し、`unity_health` は未接続時に適切なエラーを返すことを確認。

7. 回帰抑止
   - 簡易テスト（統合テスト）を追加: `server/tests/tools_list_non_empty.rs` を新設し、`tools/list` レスポンスにツール名が含まれることを確認（可能なら `rmcp` のテストユーティリティを利用）。

## ロールバック・リスク
- 主なリスクはリネーム漏れと重複シンボル。手順 2→3 を一括対応すれば解消可能。
- ロジック自体は未変更のため、Unity Bridge 連携の挙動リスクは低い。

## 完了条件（Definition of Done）
- MCP Inspector で `tools/list` が既定のツール（`unity_bridge_status`、`unity_health`、`unity_assets_*`）を返す。
- `cargo build` / `cargo clippy -D warnings` / `cargo test` が全て成功。
- 簡易テストでツール登録の非空を検証。

