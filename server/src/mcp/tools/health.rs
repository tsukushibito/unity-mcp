#[cfg(feature = "transport-grpc")]
use crate::generated::mcp::unity::v1::HealthRequest;
use crate::{mcp::service::McpService, mcp_types::HealthOut};
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content, tool};
#[cfg(feature = "transport-grpc")]
use tonic::{Code, Status};

impl McpService {
    #[tool(description = "Unity Bridge health check")]
    pub async fn unity_health(&self) -> Result<CallToolResult, McpError> {
        #[cfg(feature = "transport-grpc")]
        {
            // gRPCクライアント取得
            let mut client = self.channel_manager().editor_control_client();

            // リクエスト作成
            let request = HealthRequest {};

            // タイムアウト設定
            let timeout = self.config().health_timeout();

            // gRPC呼び出し（タイムアウト付き）
            let response = tokio::time::timeout(timeout, client.health(request))
                .await
                .map_err(|_| McpError::internal_error("Unity Bridge deadline exceeded", None))?
                .map_err(to_tool_error)?;

            let health_response = response.into_inner();

            // gRPC HealthResponse から HealthOut に変換
            let health = HealthOut {
                ready: health_response.ready,
                version: health_response.version,
            };

            let content = serde_json::to_string(&health)
                .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

            Ok(CallToolResult::success(vec![Content::text(content)]))
        }
        
        #[cfg(not(feature = "transport-grpc"))]
        {
            // Temporary stub: return a clear error until IPC implementation lands
            Err(McpError::internal_error(
                "Unity Bridge health check not available - gRPC transport disabled. IPC implementation coming in next step.",
                None
            ))
        }
    }
}

// gRPC Status -> MCP ToolError マッピング
#[cfg(feature = "transport-grpc")]
fn to_tool_error(status: Status) -> McpError {
    match status.code() {
        Code::Unavailable => McpError::internal_error("Unity Bridge unavailable", None),
        Code::DeadlineExceeded => McpError::internal_error("Unity Bridge deadline exceeded", None),
        Code::Unauthenticated => McpError::internal_error("Unauthenticated to Unity Bridge", None),
        Code::PermissionDenied => {
            McpError::internal_error("Permission denied by Unity Bridge", None)
        }
        _ => McpError::internal_error(format!("Unity Bridge error: {}", status.message()), None),
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
        let parsed: HealthOut =
            serde_json::from_str(&content).expect("Deserialization should succeed");

        assert!(parsed.ready);
        assert_eq!(parsed.version, "stub-0.1.0");
    }
}
