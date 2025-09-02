use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

impl McpService {
    pub(super) async fn do_unity_get_project_settings(
        &self,
        keys: Vec<String>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        use crate::generated::mcp::unity::v1::{
            GetProjectSettingsRequest, IpcRequest, ipc_request,
        };

        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let client = self.require_ipc().await?;

        let request = GetProjectSettingsRequest { keys };
        let ipc_request = IpcRequest {
            payload: Some(ipc_request::Payload::GetProjectSettings(request)),
        };

        let response = client.request(ipc_request, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Project settings get IPC error: {}", e), None)
        })?;

        let settings = match response.payload {
            Some(crate::generated::mcp::unity::v1::ipc_response::Payload::GetProjectSettings(
                r,
            )) => r,
            _ => {
                return Err(McpError::internal_error(
                    "Unexpected IPC response for project settings get".to_string(),
                    None,
                ));
            }
        };

        let output = GetProjectSettingsOutput {
            settings: settings.settings,
        };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_set_project_settings(
        &self,
        settings: HashMap<String, String>,
        timeout_secs: Option<u64>,
    ) -> Result<CallToolResult, McpError> {
        use crate::generated::mcp::unity::v1::{
            IpcRequest, SetProjectSettingsRequest, ipc_request,
        };

        let timeout = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));
        let client = self.require_ipc().await?;

        let request = SetProjectSettingsRequest { settings };
        let ipc_request = IpcRequest {
            payload: Some(ipc_request::Payload::SetProjectSettings(request)),
        };

        let response = client.request(ipc_request, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Project settings set IPC error: {}", e), None)
        })?;

        let resp = match response.payload {
            Some(crate::generated::mcp::unity::v1::ipc_response::Payload::SetProjectSettings(
                r,
            )) => r,
            _ => {
                return Err(McpError::internal_error(
                    "Unexpected IPC response for project settings set".to_string(),
                    None,
                ));
            }
        };

        let output = SetProjectSettingsOutput { ok: resp.ok };
        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProjectSettingsOutput {
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetProjectSettingsOutput {
    pub ok: bool,
}
