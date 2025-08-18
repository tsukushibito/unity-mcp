use server::{mcp::service::McpService, observability};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    observability::init_tracing();

    let svc = McpService::new().await?;
    svc.serve_stdio().await
}
