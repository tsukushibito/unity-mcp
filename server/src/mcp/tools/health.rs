use crate::{mcp::service::McpService, mcp_types::HealthOut};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content, tool};
use std::time::Duration;

impl McpService {
    #[tool(description = "Unity Bridge health check")]
    pub async fn unity_health(&self) -> Result<CallToolResult, McpError> {
        // IPCクライアント使用
        let timeout = Duration::from_millis(1500);
        let health_response = self.ipc().health(timeout).await.map_err(|e| {
            McpError::internal_error(format!("Unity Bridge IPC error: {}", e), None)
        })?;

        // IPC HealthResponse から HealthOut に変換
        let health = HealthOut {
            ready: health_response.ready,
            version: health_response.version,
        };

        let content = serde_json::to_string(&health)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unity_health_stub_output() {
        // スタブ実装のロジックをテスト（IPC接続のみ）
        let health = HealthOut {
            ready: true,
            version: "stub-0.1.0".to_string(),
        };

        let content = serde_json::to_string(&health).expect("Serialization should succeed");
        let parsed: HealthOut =
            serde_json::from_str(&content).expect("Deserialization should succeed");

        assert!(parsed.ready);
        assert_eq!(parsed.version, "stub-0.1.0");
    }
}
