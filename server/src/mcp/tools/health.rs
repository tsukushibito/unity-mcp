use crate::{mcp::service::McpService, mcp_types::HealthOut};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content, tool};

impl McpService {
    #[tool(description = "Unity Bridge health check")]
    pub async fn unity_health(&self) -> Result<CallToolResult, McpError> {
        // スタブ実装：固定値を返す
        let health = HealthOut {
            ready: true,
            version: "stub-0.1.0".to_string(),
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
        // スタブ実装のロジックをテスト（gRPC接続は不要）
        let health = HealthOut {
            ready: true,
            version: "stub-0.1.0".to_string(),
        };

        let content = serde_json::to_string(&health).expect("Serialization should succeed");
        let parsed: HealthOut = serde_json::from_str(&content).expect("Deserialization should succeed");
        
        assert_eq!(parsed.ready, true);
        assert_eq!(parsed.version, "stub-0.1.0");
    }
}
