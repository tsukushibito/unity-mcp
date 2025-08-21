use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content, tool};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

impl McpService {
    #[tool(description = "Import Unity assets via Direct IPC")]
    pub async fn unity_assets_import(
        &self,
        paths: Vec<String>,
        recursive: Option<bool>,
        auto_refresh: Option<bool>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let response = self
            .ipc()
            .assets_import(
                paths,
                recursive.unwrap_or(false),
                auto_refresh.unwrap_or(true),
                timeout,
            )
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Assets import IPC error: {}", e), None)
            })?;

        let results: Vec<ImportResult> = response
            .results
            .into_iter()
            .map(|r| ImportResult {
                path: r.path,
                guid: if r.guid.is_empty() {
                    None
                } else {
                    Some(r.guid)
                },
                ok: r.ok,
                message: if r.message.is_empty() {
                    None
                } else {
                    Some(r.message)
                },
            })
            .collect();

        let output = ImportAssetsOutput { results };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Move Unity asset via Direct IPC")]
    pub async fn unity_assets_move(
        &self,
        from_path: String,
        to_path: String,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let response = self
            .ipc()
            .assets_move(from_path, to_path, timeout)
            .await
            .map_err(|e| McpError::internal_error(format!("Assets move IPC error: {}", e), None))?;

        let output = MoveAssetOutput {
            ok: response.ok,
            message: if response.message.is_empty() {
                None
            } else {
                Some(response.message)
            },
            new_guid: if response.new_guid.is_empty() {
                None
            } else {
                Some(response.new_guid)
            },
        };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Delete Unity assets via Direct IPC")]
    pub async fn unity_assets_delete(
        &self,
        paths: Vec<String>,
        soft: Option<bool>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let response = self
            .ipc()
            .assets_delete(paths, soft.unwrap_or(true), timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Assets delete IPC error: {}", e), None)
            })?;

        let output = DeleteAssetsOutput {
            deleted: response.deleted,
            failed: response.failed,
        };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Refresh Unity AssetDatabase via Direct IPC")]
    pub async fn unity_assets_refresh(
        &self,
        force: Option<bool>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let response = self
            .ipc()
            .assets_refresh(force.unwrap_or(false), timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Assets refresh IPC error: {}", e), None)
            })?;

        let output = RefreshAssetsOutput { ok: response.ok };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Convert Unity asset GUIDs to paths via Direct IPC")]
    pub async fn unity_assets_guid_to_path(
        &self,
        guids: Vec<String>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let response = self
            .ipc()
            .assets_guid_to_path(guids, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Assets GUID to path IPC error: {}", e), None)
            })?;

        let output = GuidToPathOutput {
            mapping: response.map,
        };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    #[tool(description = "Convert Unity asset paths to GUIDs via Direct IPC")]
    pub async fn unity_assets_path_to_guid(
        &self,
        paths: Vec<String>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let response = self
            .ipc()
            .assets_path_to_guid(paths, timeout)
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Assets path to GUID IPC error: {}", e), None)
            })?;

        let output = PathToGuidOutput {
            mapping: response.map,
        };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAssetsOutput {
    pub results: Vec<ImportResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub path: String,
    pub guid: Option<String>,
    pub ok: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveAssetOutput {
    pub ok: bool,
    pub message: Option<String>,
    pub new_guid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAssetsOutput {
    pub deleted: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshAssetsOutput {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidToPathOutput {
    pub mapping: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathToGuidOutput {
    pub mapping: std::collections::HashMap<String, String>,
}
