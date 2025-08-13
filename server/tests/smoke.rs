use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, sync::oneshot, time::timeout};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};

use server::generated::mcp::unity::v1::{
    Empty, GetPlayModeResponse, HealthRequest, HealthResponse, SetPlayModeRequest,
    SetPlayModeResponse,
    editor_control_server::{EditorControl, EditorControlServer},
};

#[derive(Debug, Default)]
struct TestEditorControlService;

#[tonic::async_trait]
impl EditorControl for TestEditorControlService {
    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            version: "test".to_string(),
            ready: true,
            status: "OK".to_string(),
        }))
    }

    async fn get_play_mode(
        &self,
        _req: Request<Empty>,
    ) -> Result<Response<GetPlayModeResponse>, Status> {
        Ok(Response::new(GetPlayModeResponse { is_playing: false }))
    }

    async fn set_play_mode(
        &self,
        _req: Request<SetPlayModeRequest>,
    ) -> Result<Response<SetPlayModeResponse>, Status> {
        Ok(Response::new(SetPlayModeResponse { applied: true }))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn channel_manager_roundtrip_health() -> anyhow::Result<()> {
    // 1) Bind a local port and capture it before serving
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr: SocketAddr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);

    // 2) Spawn the in-process gRPC server with clean shutdown
    let (tx, rx) = oneshot::channel::<()>();
    let svc = TestEditorControlService::default();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(EditorControlServer::new(svc))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = rx.await; // wait for shutdown signal
            })
            .await
    });

    // 3) Connect ChannelManager to the test server
    let cfg = server::grpc::config::GrpcConfig {
        addr: format!("http://{}", addr),
        token: None,
        default_timeout_secs: 5,
    };
    let cm = server::grpc::channel::ChannelManager::connect(&cfg).await?;
    let mut client = cm.editor_control_client();

    // 4) Round-trip: call Health and assert L0 schema
    let resp = timeout(Duration::from_secs(5), client.health(HealthRequest {})).await??;
    let HealthResponse { version, ready, .. } = resp.into_inner();
    assert!(ready, "bridge should report ready");
    assert!(!version.is_empty(), "version should be non-empty");

    // 5) Cleanup
    let _ = tx.send(());
    server.await??;
    Ok(())
}
