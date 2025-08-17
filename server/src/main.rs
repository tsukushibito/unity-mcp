mod config;
mod observability;
mod mcp;
mod mcp_types;

use crate::mcp::service::McpService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    observability::init_tracing();
    
    let svc = McpService::new();
    svc.serve_stdio().await
}