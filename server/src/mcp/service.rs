use crate::ipc::{client::IpcClient, path::IpcConfig};
use rmcp::{
    ServerHandler, ServiceExt, handler::server::tool::ToolRouter, model::*, tool_router,
    transport::stdio,
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct OperationState {
    pub op_id: String,
    pub kind: String,
    pub progress: i32,
    pub code: i32,
    pub message: String,
    pub payload_json: String,
    pub last_updated: std::time::Instant,
}

#[derive(Clone, Debug, Default)]
pub struct BridgeState {
    pub connected: bool,
    pub attempt: u32,
    pub last_error: Option<String>,
    pub next_retry_ms: Option<u64>,
    pub endpoint: String,
}

#[derive(Clone)]
pub struct McpService {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    ipc: Arc<RwLock<Option<IpcClient>>>,
    bridge_state: Arc<RwLock<BridgeState>>,
    operations: Arc<Mutex<HashMap<String, OperationState>>>,
}

#[tool_router]
impl McpService {
    pub async fn new() -> anyhow::Result<Self> {
        let operations = Arc::new(Mutex::new(HashMap::new()));
        let ipc_cell: Arc<RwLock<Option<IpcClient>>> = Arc::new(RwLock::new(None));
        let bridge_state = Arc::new(RwLock::new(BridgeState::default()));

        // 接続スーパーバイザを起動（初回未接続でもMCPは起動継続）
        Self::spawn_bridge_connector(ipc_cell.clone(), bridge_state.clone(), operations.clone())
            .await;

        Ok(Self {
            tool_router: Self::tool_router(),
            ipc: ipc_cell,
            bridge_state,
            operations,
        })
    }

    async fn spawn_event_processor(
        ipc: IpcClient,
        operations: Arc<Mutex<HashMap<String, OperationState>>>,
    ) {
        use std::collections::HashMap;
        use std::time::{Duration, Instant};
        use tokio_stream::{StreamExt, wrappers::BroadcastStream};

        let ipc_clone = ipc.clone();

        tokio::spawn(async move {
            tracing::info!("Starting Unity event processor");

            let mut rx = ipc_clone.events();
            let mut stream = BroadcastStream::new(rx);
            let mut last_info_log: HashMap<String, Instant> = HashMap::new();
            const INFO_THROTTLE_INTERVAL: Duration = Duration::from_millis(500);

            while let Some(result) = stream.next().await {
                match result {
                    Ok(event) => match event.payload {
                        Some(crate::generated::mcp::unity::v1::ipc_event::Payload::Log(log)) => {
                            Self::process_log_event(
                                log,
                                &mut last_info_log,
                                INFO_THROTTLE_INTERVAL,
                            )
                            .await;
                        }
                        Some(crate::generated::mcp::unity::v1::ipc_event::Payload::Op(op)) => {
                            Self::process_operation_event(op, operations.clone()).await;
                        }
                        None => {
                            tracing::warn!("Received empty event payload");
                        }
                    },
                    Err(e) => {
                        tracing::error!("Event stream error: {}", e);
                        // On broadcast lag, create new receiver
                        rx = ipc_clone.events();
                        stream = BroadcastStream::new(rx);
                    }
                }
            }

            tracing::warn!("Unity event processor stopped");
        });
    }

    async fn spawn_bridge_connector(
        ipc_cell: Arc<RwLock<Option<IpcClient>>>,
        bridge_state: Arc<RwLock<BridgeState>>,
        operations: Arc<Mutex<HashMap<String, OperationState>>>,
    ) {
        tokio::spawn(async move {
            let mut backoff_ms: u64 = 200;
            const MAX_BACKOFF_MS: u64 = 5_000;
            let mut attempt: u32 = 0;

            loop {
                // 現在の設定から接続先を解決
                let cfg = IpcConfig::default();
                let endpoint_resolved = cfg
                    .endpoint
                    .as_deref()
                    .map(super::super::ipc::path::parse_endpoint)
                    .unwrap_or_else(super::super::ipc::path::default_endpoint);
                let endpoint_str = match endpoint_resolved {
                    #[cfg(unix)]
                    super::super::ipc::path::Endpoint::Unix(ref p) => {
                        format!("unix://{}", p.display())
                    }
                    #[cfg(windows)]
                    super::super::ipc::path::Endpoint::Pipe(ref name) => {
                        format!("pipe://{}", name)
                    }
                    super::super::ipc::path::Endpoint::Tcp(ref addr) => {
                        format!("tcp://{}", addr)
                    }
                };
                attempt = attempt.saturating_add(1);
                {
                    let mut s = bridge_state.write().await;
                    s.connected = false;
                    s.attempt = attempt;
                    s.next_retry_ms = None;
                    s.endpoint = endpoint_str.clone();
                }

                match IpcClient::connect(cfg).await {
                    Ok(ipc) => {
                        {
                            let mut s = bridge_state.write().await;
                            s.connected = true;
                            s.last_error = None;
                            s.next_retry_ms = None;
                            s.endpoint = endpoint_str.clone();
                        }
                        {
                            let mut guard = ipc_cell.write().await;
                            *guard = Some(ipc.clone());
                        }

                        // Unityイベント処理を起動
                        Self::spawn_event_processor(ipc.clone(), operations.clone()).await;

                        // IpcClient内部のリコネクト監視に委譲。ここでは待機。
                        tracing::info!("Unity Bridge connected. MCP tools are fully available.");
                        return; // 初回接続後は終了（内部で切断検出→自動再接続）
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        {
                            let mut s = bridge_state.write().await;
                            s.last_error = Some(msg.clone());
                        }
                        tracing::warn!(
                            attempt,
                            backoff_ms,
                            "Unity Bridge connect failed (attempt {attempt}): {msg}. Retrying in {backoff_ms}ms"
                        );

                        // 次回リトライ予定を公開
                        {
                            let mut s = bridge_state.write().await;
                            s.next_retry_ms = Some(backoff_ms);
                        }

                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        let jitter = rand::random::<u64>() % (backoff_ms / 4 + 1);
                        backoff_ms =
                            std::cmp::min(backoff_ms.saturating_mul(2), MAX_BACKOFF_MS) + jitter;
                    }
                }
            }
        });
    }

    /// Process log events with throttling
    async fn process_log_event(
        log: crate::generated::mcp::unity::v1::LogEvent,
        last_info_log: &mut std::collections::HashMap<String, std::time::Instant>,
        throttle_interval: std::time::Duration,
    ) {
        use crate::generated::mcp::unity::v1::log_event::Level;

        let now = std::time::Instant::now();
        let should_log = match log.level() {
            Level::Error => {
                tracing::error!(
                    target = "unity",
                    category = %log.category,
                    message = %log.message,
                    stack_trace = %log.stack_trace,
                    "Unity error"
                );
                true
            }
            Level::Warn => {
                tracing::warn!(
                    target = "unity",
                    category = %log.category,
                    message = %log.message,
                    "Unity warning"
                );
                true
            }
            Level::Info | Level::Debug | Level::Trace => {
                // Throttle INFO/DEBUG/TRACE logs by category
                let key = format!("{}:{}", log.category, log.level() as i32);
                let should_emit = last_info_log
                    .get(&key)
                    .map(|last| now.duration_since(*last) > throttle_interval)
                    .unwrap_or(true);

                if should_emit {
                    last_info_log.insert(key, now);
                    match log.level() {
                        Level::Info => {
                            tracing::info!(
                                target = "unity",
                                category = %log.category,
                                message = %log.message,
                                "Unity info"
                            );
                        }
                        Level::Debug => {
                            tracing::debug!(
                                target = "unity",
                                category = %log.category,
                                message = %log.message,
                                "Unity debug"
                            );
                        }
                        Level::Trace => {
                            tracing::trace!(
                                target = "unity",
                                category = %log.category,
                                message = %log.message,
                                "Unity trace"
                            );
                        }
                        _ => {}
                    }
                    true
                } else {
                    false
                }
            }
        };

        if !should_log {
            // Optionally count dropped messages
            tracing::trace!(
                target = "unity",
                "Throttled log message from category: {}",
                log.category
            );
        }
    }

    async fn process_operation_event(
        op: crate::generated::mcp::unity::v1::OperationEvent,
        operations: Arc<Mutex<HashMap<String, OperationState>>>,
    ) {
        use crate::generated::mcp::unity::v1::operation_event::Kind;

        let op_state = OperationState {
            op_id: op.op_id.clone(),
            kind: format!("{:?}", op.kind()),
            progress: op.progress,
            code: op.code,
            message: op.message.clone(),
            payload_json: op.payload_json.clone(),
            last_updated: std::time::Instant::now(),
        };

        // Update operation state
        {
            let mut ops = operations.lock().await;
            ops.insert(op.op_id.clone(), op_state);

            // Clean up completed operations older than 5 minutes
            let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(300);
            ops.retain(|_, state| !(state.kind == "Complete" && state.last_updated < cutoff));
        }

        match op.kind() {
            Kind::Start => {
                tracing::info!(
                    target = "unity.operation",
                    op_id = %op.op_id,
                    message = %op.message,
                    "Operation started"
                );
            }
            Kind::Progress => {
                tracing::debug!(
                    target = "unity.operation",
                    op_id = %op.op_id,
                    progress = %op.progress,
                    message = %op.message,
                    "Operation progress"
                );
            }
            Kind::Complete => {
                if op.code == 0 {
                    tracing::info!(
                        target = "unity.operation",
                        op_id = %op.op_id,
                        code = %op.code,
                        message = %op.message,
                        payload = %op.payload_json,
                        "Operation completed successfully"
                    );
                } else {
                    tracing::warn!(
                        target = "unity.operation",
                        op_id = %op.op_id,
                        code = %op.code,
                        message = %op.message,
                        payload = %op.payload_json,
                        "Operation completed with error"
                    );
                }
            }
        }
    }

    /// Get all active operations
    pub async fn get_operations(&self) -> HashMap<String, OperationState> {
        let ops = self.operations.lock().await;
        ops.clone()
    }

    /// Get specific operation by ID
    pub async fn get_operation(&self, op_id: &str) -> Option<OperationState> {
        let ops = self.operations.lock().await;
        ops.get(op_id).cloned()
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    // 接続必須の内部アクセサー（未接続時はMCPエラー相当の説明文を返すためにResult化）
    pub async fn require_ipc(&self) -> Result<IpcClient, rmcp::ErrorData> {
        if let Some(c) = self.ipc.read().await.clone() {
            Ok(c)
        } else {
            let s = self.bridge_state.read().await.clone();
            let mut msg =
                String::from("Unity Bridge not connected yet. Waiting for Unity Editor to start.");
            if let Some(err) = &s.last_error {
                msg.push_str(&format!(" last_error={}", err));
            }
            if let Some(ms) = s.next_retry_ms {
                msg.push_str(&format!(" next_retry_ms={}", ms));
            }
            Err(rmcp::ErrorData::internal_error(msg, None))
        }
    }

    pub async fn get_bridge_state(&self) -> BridgeState {
        self.bridge_state.read().await.clone()
    }
}

impl ServerHandler for McpService {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            server_info: Implementation {
                name: "unity-mcp-server".to_string(),
                version: "0.1.0".to_string(),
            },
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::default(),
            instructions: None,
        }
    }
}
