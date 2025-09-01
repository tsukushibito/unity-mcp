use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

impl McpService {
    pub(super) async fn do_unity_component_add(
        &self,
        game_object: String,
        component: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .component_add(game_object, component, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Component add IPC error: {}", e), None)
            })?;

        let output = ComponentAddOutput {
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

    pub(super) async fn do_unity_get_components(
        &self,
        game_object: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc.component_get(game_object, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Component get IPC error: {}", e), None)
        })?;

        let output = GetComponentsOutput {
            components: response.components,
        };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_component_remove(
        &self,
        game_object: String,
        component: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .component_remove(game_object, component, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Component remove IPC error: {}", e), None)
            })?;

        let output = ComponentRemoveOutput {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentAddOutput {
    pub ok: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetComponentsOutput {
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRemoveOutput {
    pub ok: bool,
    pub message: Option<String>,
}
