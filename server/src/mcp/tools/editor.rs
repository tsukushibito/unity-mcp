use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

impl McpService {
    pub(super) async fn do_unity_execute_menu_item(
        &self,
        path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc.execute_menu_item(path, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Menu execution IPC error: {}", e), None)
        })?;

        let output = ExecuteMenuItemOutput {
            ok: response.ok,
            message: if response.message.is_empty() {
                None
            } else {
                Some(response.message)
            },
        };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_focus_window(
        &self,
        window_type: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc.focus_window(window_type, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Window focus IPC error: {}", e), None)
        })?;

        let output = FocusWindowOutput { ok: response.ok };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteMenuItemOutput {
    pub ok: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusWindowOutput {
    pub ok: bool,
}
