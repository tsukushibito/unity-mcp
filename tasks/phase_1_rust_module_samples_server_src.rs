// NOTE: These are sample skeletons for Phase 1. They show structure, imports, and
// responsibilities. Some items are placeholders where your actual rmcp/grpc types differ.
// Required crates (add via `cargo add`):
//   rmcp@^0.5, serde, serde_json, tokio, tokio-stream, futures (optional), chrono (optional)
// And your existing tonic/prost + ChannelManager/GrpcConfig modules.

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/mcp.rs  (module root; declares submodules in mcp/)
// ──────────────────────────────────────────────────────────────────────────────
pub mod server;
pub mod context;
pub mod error;

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/mcp/context.rs  (shared dependency container)
// ──────────────────────────────────────────────────────────────────────────────
use std::sync::Arc;
use crate::grpc::{channel::ChannelManager, config::GrpcConfig};
use crate::relay::logs::LogsRelayHandle;

#[derive(Clone)]
pub struct AppContext {
    cm: ChannelManager,
    cfg: GrpcConfig,
    logs: LogsRelayHandle,
}

impl AppContext {
    pub fn new(cm: ChannelManager, cfg: GrpcConfig, logs: LogsRelayHandle) -> Arc<Self> {
        Arc::new(Self { cm, cfg, logs })
    }

    pub fn config(&self) -> &GrpcConfig { &self.cfg }

    pub fn channel_manager(&self) -> &ChannelManager { &self.cm }

    /// Client with default endpoint policy
    pub fn editor_client(&self) -> crate::grpc::clients::EditorControlClient {
        self.cm.editor_control_client()
    }

    /// Client with per-call timeout override
    pub fn editor_client_with_timeout(
        &self,
        dur: std::time::Duration,
    ) -> crate::grpc::clients::EditorControlClient {
        self.cm.editor_control_client_with_timeout(dur)
    }

    /// Access the logs relay handle
    pub fn logs_relay(&self) -> &LogsRelayHandle { &self.logs }
}

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/mcp/error.rs  (gRPC Status → MCP error mapping)
// ──────────────────────────────────────────────────────────────────────────────
use serde::Serialize;

#[derive(thiserror::Error, Debug, Serialize)]
#[serde(tag = "code", content = "details")]
pub enum McpError {
    #[error("service unavailable: {0}")]
    service_unavailable(String),
    #[error("timeout: {0}")]
    timeout_error(String),
    #[error("internal error: {0}")]
    internal_error(String),
}

impl From<tonic::Status> for McpError {
    fn from(s: tonic::Status) -> Self {
        use tonic::Code;
        match s.code() {
            Code::Unavailable => McpError::service_unavailable(s.message().to_string()),
            Code::DeadlineExceeded => McpError::timeout_error(s.message().to_string()),
            _ => McpError::internal_error(format!("{}: {}", s.code() as i32, s.message())),
        }
    }
}

// Helper to convert anyhow/other errors to McpError uniformly
impl From<anyhow::Error> for McpError {
    fn from(e: anyhow::Error) -> Self { McpError::internal_error(e.to_string()) }
}

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/mcp/server.rs  (MCP stdio bootstrap)
// ──────────────────────────────────────────────────────────────────────────────
use std::sync::Arc;
use crate::{mcp::context::AppContext, tools, tools::editor, tools::events};

// TODO: Replace these with the actual rmcp server imports for your runtime.
// The below is pseudo-API showing intent.
mod rmcp_api {
    use super::*;
    use futures::Stream;
    use serde_json::Value;

    pub struct ServerBuilder;
    impl ServerBuilder {
        pub fn stdio() -> Self { ServerBuilder }
        pub fn build(self) -> Server { Server }
    }

    pub struct Server;
    impl Server {
        pub fn register_tool<F>(&mut self, name: &str, f: F)
        where F: Fn(Arc<AppContext>, serde_json::Value) -> Result<Value, crate::mcp::error::McpError> + Send + Sync + 'static {}

        pub fn register_stream<S, F>(&mut self, name: &str, f: F)
        where S: Stream<Item = serde_json::Value> + Send + 'static,
              F: Fn(Arc<AppContext>, serde_json::Value) -> S + Send + Sync + 'static {}

        pub async fn serve(self) -> anyhow::Result<()> { Ok(()) }
    }
}

pub async fn run_stdio_server(ctx: Arc<AppContext>) -> anyhow::Result<()> {
    use rmcp_api::*;

    let mut server = ServerBuilder::stdio().build();

    // unity.editor.health (unary)
    server.register_tool("unity.editor.health", move |ctx, _params| {
        editor::handle_health(ctx)
    });

    // unity.events.subscribe_logs (stream)
    server.register_stream("unity.events.subscribe_logs", move |ctx, _params| {
        events::subscribe_logs(ctx)
    });

    server.serve().await
}

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/tools.rs  (module root; declares submodules in tools/)
// ──────────────────────────────────────────────────────────────────────────────
pub mod editor;
pub mod events;

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/tools/editor.rs  (Tool: unity.editor.health)
// ──────────────────────────────────────────────────────────────────────────────
use std::sync::Arc;
use serde_json::{json, Value};
use crate::mcp::{context::AppContext, error::McpError};
use std::time::{Duration, Instant};

pub fn handle_health(ctx: Arc<AppContext>) -> Result<Value, McpError> {
    // Per-call timeout: 5s
    let dur = Duration::from_secs(5);

    // Use a blocking-on-async bridge only if your rmcp tool signature is sync.
    // Prefer async handlers in your actual rmcp runtime if supported.
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async move {
            let t0 = Instant::now();
            let mut cli = ctx.editor_client_with_timeout(dur);

            // TODO: import actual request/response types from your generated stubs
            // e.g., use crate::generated::editor_control::{HealthRequest, HealthResponse};
            let rsp = cli.health(Default::default()).await.map_err(McpError::from)?;

            let latency_ms = t0.elapsed().as_millis() as u64;
            let cfg = ctx.config();

            Ok::<Value, McpError>(json!({
                "ready": rsp.ready,
                "version": rsp.version,
                "status": rsp.status,
                "bridge_addr": cfg.addr,
                "observed_at": chrono::Utc::now().timestamp(),
                "latency_ms": latency_ms
            }))
        })
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/tools/events.rs  (Tool: unity.events.subscribe_logs)
// ──────────────────────────────────────────────────────────────────────────────
use std::sync::Arc;
use serde_json::Value;
use futures::{Stream, StreamExt};
use tokio_stream::wrappers::BroadcastStream;
use crate::mcp::context::AppContext;

/// Returns a JSON stream of log events: { kind, message, ts, level?, source?, op_id?, seq? }
pub fn subscribe_logs(ctx: Arc<AppContext>) -> impl Stream<Item = Value> {
    let rx = ctx.logs_relay().subscribe();
    BroadcastStream::new(rx)
        .filter_map(|item| async move { item.ok() })
        .map(|ev| serde_json::to_value(ev).expect("serialize LogEvent"))
}

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/relay.rs  (module root; declares submodules in relay/)
// ──────────────────────────────────────────────────────────────────────────────
pub mod logs;

// ──────────────────────────────────────────────────────────────────────────────
// File: server/src/relay/logs.rs  (Heartbeat producer + broadcast relay)
// ──────────────────────────────────────────────────────────────────────────────
use std::time::Duration;
use tokio::sync::broadcast;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
    pub kind: String,          // e.g., "heartbeat", "info", "warn", "error"
    pub message: String,       // human-readable message
    pub ts: i64,               // epoch seconds
    #[serde(skip_serializing_if = "Option::is_none")] pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub op_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub seq: Option<u64>,
}

impl LogEvent {
    pub fn heartbeat(msg: impl Into<String>, seq: u64) -> Self {
        Self {
            kind: "heartbeat".into(),
            message: msg.into(),
            ts: chrono::Utc::now().timestamp(),
            level: None,
            source: Some("server".into()),
            op_id: None,
            seq: Some(seq),
        }
    }
}

#[derive(Clone)]
pub struct LogsRelayHandle {
    tx: broadcast::Sender<LogEvent>,
}

impl LogsRelayHandle {
    pub fn subscribe(&self) -> broadcast::Receiver<LogEvent> { self.tx.subscribe() }
}

/// Spawns a background task that emits heartbeat events every 2s.
/// Bounded channel; oldest items are dropped on overflow.
pub fn spawn_heartbeat() -> LogsRelayHandle {
    let (tx, _rx) = broadcast::channel(128);
    let handle = LogsRelayHandle { tx: tx.clone() };

    tokio::spawn(async move {
        let mut seq: u64 = 0;
        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        loop {
            ticker.tick().await;
            seq += 1;
            let _ = tx.send(LogEvent::heartbeat("bridge: idle", seq));
        }
    });

    handle
}
