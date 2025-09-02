use crate::mcp::service::McpService;
use rmcp::{
    ErrorData as McpError,
    model::{CallToolResult, Content},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

impl McpService {
    pub(super) async fn do_unity_prefab_create(
        &self,
        game_object_path: String,
        prefab_path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .prefab_create(game_object_path, prefab_path, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Prefab create IPC error: {}", e), None)
            })?;

        let output = PrefabCreateOutput {
            ok: response.ok,
            guid: if response.guid.is_empty() {
                None
            } else {
                Some(response.guid)
            },
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

    pub(super) async fn do_unity_prefab_update(
        &self,
        game_object_path: String,
        prefab_path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .prefab_update(game_object_path, prefab_path, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Prefab update IPC error: {}", e), None)
            })?;

        let output = PrefabUpdateOutput {
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

    pub(super) async fn do_unity_prefab_apply_overrides(
        &self,
        instance_path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;
        let response = ipc
            .prefab_apply_overrides(instance_path, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Prefab apply overrides IPC error: {}", e), None)
            })?;

        let output = PrefabApplyOverridesOutput {
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
pub struct PrefabCreateOutput {
    pub ok: bool,
    pub guid: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabUpdateOutput {
    pub ok: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabApplyOverridesOutput {
    pub ok: bool,
    pub message: Option<String>,
}
