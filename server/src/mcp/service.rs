use crate::config::ServerConfig;
use crate::ipc::{client::IpcClient, path::IpcConfig};
use rmcp::{
    ServerHandler, ServiceExt, handler::server::tool::ToolRouter, model::*, tool_router,
    transport::stdio,
};

#[derive(Clone)]
pub struct McpService {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    ipc: IpcClient,
    config: ServerConfig,
}

#[tool_router]
impl McpService {
    pub async fn new() -> anyhow::Result<Self> {
        let config = ServerConfig::load();
        let ipc = IpcClient::connect(IpcConfig::default()).await?;
        Ok(Self {
            tool_router: Self::tool_router(),
            ipc,
            config,
        })
    }

    /// Create McpService with explicit configuration for testing
    pub async fn with_config(config: ServerConfig) -> anyhow::Result<Self> {
        let ipc = IpcClient::connect(IpcConfig::default()).await?;
        Ok(Self {
            tool_router: Self::tool_router(),
            ipc,
            config,
        })
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let service = self.serve(stdio()).await?;
        service.waiting().await?;
        Ok(())
    }

    // 内部アクセサー
    pub(crate) fn ipc(&self) -> &IpcClient {
        &self.ipc
    }

    pub(crate) fn config(&self) -> &ServerConfig {
        &self.config
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
