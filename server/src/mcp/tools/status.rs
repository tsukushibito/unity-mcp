use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content, tool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatusOut {
    pub connected: bool,
    pub attempt: u32,
    pub last_error: Option<String>,
    pub next_retry_ms: Option<u64>,
    pub negotiated_features: Option<Vec<String>>, // 接続済みなら公開
    pub endpoint: String,
}

impl McpService {
    #[tool(description = "Unity Bridge connection status (always available)")]
    pub async fn unity_bridge_status(&self) -> Result<CallToolResult, McpError> {
        let mut out = {
            let s = self.get_bridge_state().await;
            BridgeStatusOut {
                connected: s.connected,
                attempt: s.attempt,
                last_error: s.last_error,
                next_retry_ms: s.next_retry_ms,
                negotiated_features: None,
                endpoint: s.endpoint,
            }
        };

        // 接続済みなら交渉済み機能も返す（情報価値向上）
        if out.connected
            && let Ok(ipc) = self.require_ipc().await
        {
            let features = ipc.get_negotiated_features().await.to_strings();
            out.negotiated_features = Some(features);
        }

        let content = serde_json::to_string(&out)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}
