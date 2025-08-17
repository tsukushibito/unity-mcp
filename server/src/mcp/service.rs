use rmcp::{
    handler::server::tool::ToolRouter, 
    tool_router, 
    transport::stdio,
    ServerHandler,
    ServiceExt,
    model::*,
};

#[derive(Clone)]
pub struct McpService {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpService {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }
}

impl ServerHandler for McpService {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            server_info: Implementation {
                name: "unity-mcp-server".to_string(),
                version: "0.1.0".to_string(),
            },
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::default(),
            instructions: None,
        }
    }
}