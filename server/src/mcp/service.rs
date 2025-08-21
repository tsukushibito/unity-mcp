use crate::ipc::{client::IpcClient, path::IpcConfig};
use rmcp::{
    ServerHandler, ServiceExt, handler::server::tool::ToolRouter, model::*, tool_router,
    transport::stdio,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

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

#[derive(Clone)]
pub struct McpService {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    ipc: IpcClient,
    operations: Arc<Mutex<HashMap<String, OperationState>>>,
}

#[tool_router]
impl McpService {
    pub async fn new() -> anyhow::Result<Self> {
        let ipc = IpcClient::connect(IpcConfig::default()).await?;
        let operations = Arc::new(Mutex::new(HashMap::new()));

        // Spawn event processing task
        Self::spawn_event_processor(ipc.clone(), operations.clone()).await;

        Ok(Self {
            tool_router: Self::tool_router(),
            ipc,
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

    // 内部アクセサー
    pub(crate) fn ipc(&self) -> &IpcClient {
        &self.ipc
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
