use rmcp::{tool, model::CallToolResult, model::Content, ErrorData as McpError};
use crate::mcp_types::HealthOut;
use crate::mcp::service::McpService;

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
    async fn test_unity_health_stub() {
        let svc = McpService::new();
        let result = svc.unity_health().await;
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        // エラーフラグがないかfalseであることを確認
        assert!(tool_result.is_error.is_none() || !tool_result.is_error.unwrap());
        // コンテンツが存在することを確認
        assert!(tool_result.content.is_some());
    }
}