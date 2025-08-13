use std::process;
use tracing::{error, info, warn};

use server::generated::mcp::unity::v1::HealthRequest;
use server::grpc::channel::ChannelManager;
use server::grpc::config::GrpcConfig;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_tracing();

    let cfg = GrpcConfig::from_env();
    info!(addr = %cfg.addr, timeout_secs = cfg.default_timeout_secs, "Starting minimal EditorControl client");

    let manager = match ChannelManager::connect(&cfg).await {
        Ok(m) => m,
        Err(e) => {
            error!(error = %e, "Failed to connect to gRPC bridge");
            process::exit(2);
        }
    };

    // Client with interceptor that adds Authorization headers when token is configured
    let mut client = manager.editor_control_client();

    match client.health(HealthRequest {}).await {
        Ok(resp) => {
            let body = resp.into_inner();
            if body.ready {
                info!(version = %body.version, "Bridge is ready");
                process::exit(0);
            } else {
                warn!(version = %body.version, "Bridge responded but not ready");
                process::exit(4);
            }
        }
        Err(status) => {
            error!(code = ?status.code(), message = %status.message(), "Health RPC failed");
            process::exit(3);
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("valid RUST_LOG");

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
