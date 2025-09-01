use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneOpOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenScenesOutput {
    pub scenes: Vec<String>,
    pub active_scene: Option<String>,
}

impl McpService {
    pub(super) async fn do_unity_scene_open(
        &self,
        path: String,
        additive: Option<bool>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .scenes_open(path, additive.unwrap_or(false), timeout)
            .await
            .map_err(|e| McpError::internal_error(format!("Scene open IPC error: {}", e), None))?;
        let output = SceneOpOutput { ok: response.ok };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_scene_save(
        &self,
        path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .scenes_save(path, timeout)
            .await
            .map_err(|e| McpError::internal_error(format!("Scene save IPC error: {}", e), None))?;
        let output = SceneOpOutput { ok: response.ok };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_scene_get_open(
        &self,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .scenes_get_open_scenes(timeout)
            .await
            .map_err(|e| McpError::internal_error(format!("Scene list IPC error: {}", e), None))?;
        let output = OpenScenesOutput {
            scenes: response.scenes,
            active_scene: if response.active_scene.is_empty() {
                None
            } else {
                Some(response.active_scene)
            },
        };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_scene_set_active(
        &self,
        path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .scenes_set_active_scene(path, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Scene set active IPC error: {}", e), None)
            })?;
        let output = SceneOpOutput { ok: response.ok };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}
