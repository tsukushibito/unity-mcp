# rmcp の通知機能 使い方まとめ（MCP Rust SDK）

目的: Rust 製 MCP サーバ/クライアントで、rmcp（Rust SDK）を用いた通知の送り方・受け取り方・実装上の要点を短くまとめます。ここではこのリポジトリ（server/Cargo.toml の `rmcp` 依存）で実際に使える形を優先し、バージョン依存が強い API 名は控えめに扱います。

---

## なにが「通知」か
- MCP では JSON‑RPC の通知メッセージで非同期イベントを伝達します（例: 進捗、リソース更新、ツール一覧変更）。
- 代表的な通知（サーバ→クライアント）
  - `notifications/progress`（進捗）
  - `notifications/logging/message`（構造化ログ／任意イベント）
  - `notifications/resources/updated`（購読者向けのリソース更新）
  - `notifications/resources/list_changed`（リソース一覧の変更）
  - `notifications/tools/list_changed`（ツール一覧の変更）
  - `notifications/logging/message`
- 一部の通知は capability 宣言が必要（例: resources/tools の listChanged、resources の subscribe）。
  - 仕様: Resources、Tools、Progress に関する定義と送信例は MCP 公式仕様を参照。 [spec: resources/listChanged, subscribe]、[spec: tools/listChanged]、[spec: progress]

参考: MCP 仕様（抜粋）
- Resources: listChanged/subscribe による更新通知。
- Tools: listChanged による一覧変更通知。
- Progress: `notifications/progress`。

---

## 前提: サーバの capability 宣言
`ServerHandler::get_info` で必要な capability を返します。最低限、ツールを使うなら `tools`、構造化ログ通知を使うなら `logging` を公開します。リソース通知（subscribe/listChanged）を使う場合は `resources` も有効化します。

```rust
use rmcp::{ServerHandler, model::*};

#[derive(Clone, Default)]
pub struct MyServer;

impl ServerHandler for MyServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            server_info: Implementation { name: "my-server".into(), version: "0.1.0".into() },
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                logging: Some(Default::default()), // `notifications/logging/message` を使う場合
                // resources を使う場合のみ有効化する:
                // resources: Some(ResourcesCapability { subscribe: Some(true), list_changed: Some(true) }),
                ..Default::default()
            },
            instructions: None,
        }
    }
}
```

- 初期化後、クライアントは `notifications/initialized` を送って通常運用に移行します。初期化前は不要な通知を送らないのが無難です。 [spec: lifecycle]

---

## 通知の送り方（サーバ側）
rmcp は型付き API を提供します。サーバ起動時に `.serve(...)` で得られるサービスハンドル、またはハンドラ内で受け取れる `Peer` に対して `notify_*` メソッドを呼びます。

このリポジトリの実装方針（推奨）
- 独自イベントは `notifications/logging/message` に構造化 JSON を載せて送る（互換性が高く、クライアント側で取り回しが容易）。
- 進捗は progressToken を受けた場合に `notifications/progress` を送る。

### 1) サービスハンドルから送る（任意のタイミング）
```rust
use rmcp::{ServiceExt, transport::stdio};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = MyServer::default().serve(stdio()).await?; // 初期化完了

    // 例: ログ通知（任意の構造を data に入れられる）
    server
        .notify_logging_message(rmcp::model::LoggingMessageNotificationParam {
            level: rmcp::model::LoggingLevel::Info,
            logger: None,
            data: serde_json::json!({ "message": "warm-up done" }),
        })
        .await?;

    // デモ待機
    sleep(Duration::from_secs(60)).await;
    Ok(())
}
```

- メソッド名は概ね `notify_<種類>` という規則です。`notify_logging_message` は広く利用可能です。
- リソース関連（`notify_resource_updated` など）は SDK のバージョンにより API 名や利用条件（subscribe 必須）が異なるため、利用時に docs.rs の該当バージョンを確認してください。

### 2) リクエスト処理中に送る（進捗など）
ハンドラ内では `RequestContext`/`NotificationContext` から `peer` を取り出して送信できます。

```rust
use rmcp::{ServerHandler, model::*, service::{RequestContext, RoleServer}};

#[rmcp::tool_handler]
impl ServerHandler for MyServer {
    async fn call_tool(
        &self,
        req: CallToolRequestParam,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::Error> {
        // progressToken を要求側が _meta で付けた場合、進捗通知を返せる
        if let Some(meta) = req.meta.clone() {
            if let Some(token) = meta.progress_token {
                let _ = ctx.peer
                    .notify_progress(ProgressNotificationParam {
                        progress_token: token,
                        progress: 10.0,
                        total: Some(100.0),
                        message: Some("Starting heavy work".into()),
                    })
                    .await;
            }
        }
        // ...長い処理...
        Ok(CallToolResult::success(vec![Content::text("done")]))
    }
}
```

- 進捗通知は「要求に progressToken が付いている場合」に送るのがルールです。
- 頻度や閾値（スロットリング）は実装側で制御します。

---

## 通知の受け取り方（クライアント側）
rmcp クライアントでは `ClientHandler` を実装すると、サーバからの通知を型安全に受け取れます。

```rust
use rmcp::{ClientHandler, ClientInfo, model::*, service::{Peer, RoleClient}};
use async_trait::async_trait;

#[derive(Clone, Debug)]
struct MyClient { info: ClientInfo, peer: Option<Peer<RoleClient>> }

impl MyClient { fn new() -> Self { Self { info: ClientInfo { name: "my-client".into(), version: "0.1.0".into(), supported_protocol_versions: None }, peer: None } } }

#[async_trait]
impl ClientHandler for MyClient {
    fn get_info(&self) -> ClientInfo { self.info.clone() }
    fn get_peer(&self) -> Option<Peer<RoleClient>> { self.peer.clone() }
    fn set_peer(&mut self, p: Peer<RoleClient>) { self.peer = Some(p) }

    async fn on_progress(&self, p: ProgressNotificationParam) -> Result<(), rmcp::Error> {
        println!("progress: token={:?} {} / {:?} msg={:?}", p.progress_token, p.progress, p.total, p.message);
        Ok(())
    }

    async fn on_resource_updated(&self, n: ResourceUpdatedNotificationParam) -> Result<(), rmcp::Error> {
        println!("resource updated: {}", n.uri);
        Ok(())
    }
}
```

- 必要に応じて `on_resource_list_changed`、`on_tool_list_changed`、`on_logging_message` 等も実装します。

---

## よくある落とし穴
- 初期化フェーズ前に（`initialized` 前）に仕様外の通知を送らない。
- `resources/updated` は購読者（`resources/subscribe` 済みクライアント）がいないと意味がない。
- 進捗通知は progressToken を伴う要求に対してのみ送る（自由送信は不可）。 [spec: progress]
- 通知はベストエフォート。取りこぼしに備え、必要ならファイル等のフォールバック読取も併用する。

---

## バージョン差異の注意
- 本リポジトリは `server/Cargo.toml` で `rmcp` を利用しています。メソッド名や Capability 構造体のフィールド（例: resources の `subscribe`/`list_changed`）はバージョンで変わることがあります。
- 使う前に必ず「利用している rmcp のバージョンの docs.rs」を確認してください。IDE の補完も活用すると安全です。

---

## 追加メモ（カスタム用途）
- rmcp の公開 API は MCP 仕様で定義済みの通知を型付きで提供します（例: `notify_progress`, `notify_logging_message`, `notify_cancelled` など）。
- アプリ固有の状態通知が必要な場合は、まず `notifications/logging/message` の `data` に構造化 JSON を載せる運用が簡単です（クライアント側でスキーマを共有して解釈）。将来的に仕様拡張・SDK 提案で型付き通知を追加する案も検討できます。

---

## 参考リンク
- rmcp（Rust SDK）: `notify_*` で通知を送る例、`Peer` からの送信、`ClientHandler` での受信（docs.rs の自分の利用バージョンを確認してください）。
- MCP 仕様（Resources/Tools/Progress/Lifecycle など）: modelcontextprotocol.io の仕様セクション。

補足: 代表的なメソッド名と型は、`rmcp::model` の `*Notification*` 型（例: `ProgressNotificationParam`, `LoggingMessageNotificationParam`）と、サービス／`Peer` の `notify_*` 群として公開されています。IDE の補完／docs.rs を併用してください。

---

## スニペット集（抜粋）
- Progress 通知（要求に progressToken がある場合）
```rust
ctx.peer.notify_progress(rmcp::model::ProgressNotificationParam {
    progress_token: token,
    progress: 50.0,
    total: Some(100.0),
    message: Some("Halfway".into()),
}).await?;
```
- Logging 通知（独自イベント）
```rust
server.notify_logging_message(rmcp::model::LoggingMessageNotificationParam {
    level: rmcp::model::LoggingLevel::Info,
    logger: Some("tests".into()),
    data: serde_json::json!({"event": "unity.tests.finished", "runId": run_id}),
}).await?;
```
