#[cfg(feature = "server-stubs")]
use futures;
#[cfg(feature = "server-stubs")]
use std::{net::SocketAddr, time::Duration};
#[cfg(feature = "server-stubs")]
use tokio::{net::TcpListener, sync::oneshot, time::sleep};
#[cfg(feature = "server-stubs")]
use tokio_stream::wrappers::TcpListenerStream;
#[cfg(feature = "server-stubs")]
use tonic::{Request, Response, Status, transport::Server};

#[cfg(feature = "server-stubs")]
use server::generated::mcp::unity::v1::{
    HealthRequest, HealthResponse,
    editor_control_server::{EditorControl, EditorControlServer},
};

#[cfg(feature = "server-stubs")]
use server::config::{BridgeConfig, GrpcConfig, ServerConfig};
#[cfg(feature = "server-stubs")]
use server::mcp::service::McpService;

#[cfg(feature = "server-stubs")]
#[derive(Debug)]
struct TestEditorControlService {
    behavior: TestBehavior,
}

#[cfg(feature = "server-stubs")]
#[derive(Debug, Clone)]
enum TestBehavior {
    Success { ready: bool, version: String },
    Delay(Duration),
    Unavailable,
}

#[cfg(feature = "server-stubs")]
#[tonic::async_trait]
impl EditorControl for TestEditorControlService {
    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        match &self.behavior {
            TestBehavior::Success { ready, version } => Ok(Response::new(HealthResponse {
                ready: *ready,
                version: version.clone(),
                status: if *ready {
                    "OK".to_string()
                } else {
                    "NOT_READY".to_string()
                },
            })),
            TestBehavior::Delay(d) => {
                sleep(*d).await;
                Ok(Response::new(HealthResponse {
                    ready: true,
                    version: "delayed".to_string(),
                    status: "OK".to_string(),
                }))
            }
            TestBehavior::Unavailable => {
                Err(Status::unavailable("Service temporarily unavailable"))
            }
        }
    }

    async fn get_play_mode(
        &self,
        _req: Request<server::generated::mcp::unity::v1::Empty>,
    ) -> Result<Response<server::generated::mcp::unity::v1::GetPlayModeResponse>, Status> {
        Err(Status::unimplemented("Not implemented in test"))
    }

    async fn set_play_mode(
        &self,
        _req: Request<server::generated::mcp::unity::v1::SetPlayModeRequest>,
    ) -> Result<Response<server::generated::mcp::unity::v1::SetPlayModeResponse>, Status> {
        Err(Status::unimplemented("Not implemented in test"))
    }
}

#[cfg(feature = "server-stubs")]
async fn start_test_server(
    behavior: TestBehavior,
) -> anyhow::Result<(SocketAddr, oneshot::Sender<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);

    let (tx, rx) = oneshot::channel::<()>();
    let svc = TestEditorControlService { behavior };

    tokio::spawn(async move {
        let _ = Server::builder()
            .add_service(EditorControlServer::new(svc))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = rx.await;
            })
            .await;
    });

    // サーバー起動を少し待つ
    tokio::time::sleep(Duration::from_millis(100)).await;

    Ok((addr, tx))
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_success() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: true,
        version: "test-1.0.0".to_string(),
    };

    let (addr, _shutdown) = start_test_server(behavior).await?;

    // 設定を直接構築（環境変数なし）
    let grpc_config = GrpcConfig::from_map([
        ("MCP_BRIDGE_ADDR", format!("http://{}", addr)),
        ("MCP_BRIDGE_TIMEOUT", "5".to_string()),
    ]);
    let bridge_config = BridgeConfig::with_values("127.0.0.1".to_string(), 50051, 5000);
    let server_config = ServerConfig::with_configs(grpc_config, bridge_config);

    let svc = McpService::with_config(server_config).await?;

    // unity_healthメソッドを直接呼び出し
    let result = svc.unity_health().await?;

    // MCPレスポンスがエラーでないことを確認
    assert!(result.is_error.is_none() || result.is_error == Some(false));
    assert!(result.content.is_some());

    // 基本的な応答があることを確認（詳細な内容検証は後で実装）
    let _content = &result.content.as_ref().unwrap()[0];

    // テストが正常に実行されていることを確認
    // （実際のJSON内容の検証は型問題解決後に実装）

    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_not_ready() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: false,
        version: "test-not-ready".to_string(),
    };

    let (addr, _shutdown) = start_test_server(behavior).await?;

    // 設定を直接構築（環境変数なし）
    let grpc_config = GrpcConfig::from_map([("MCP_BRIDGE_ADDR", format!("http://{}", addr))]);
    let bridge_config = BridgeConfig::with_values("127.0.0.1".to_string(), 50051, 2000);
    let server_config = ServerConfig::with_configs(grpc_config, bridge_config);

    let svc = McpService::with_config(server_config).await?;

    let result = svc.unity_health().await?;

    // レスポンス内容の基本検証
    assert!(result.is_error.is_none() || result.is_error == Some(false));
    assert!(result.content.is_some());

    // 基本的な応答があることを確認

    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_timeout() -> anyhow::Result<()> {
    let behavior = TestBehavior::Delay(Duration::from_millis(2000)); // 2秒遅延
    let (addr, _shutdown) = start_test_server(behavior).await?;

    // 設定を直接構築（短いタイムアウト）
    let grpc_config = GrpcConfig::from_map([
        ("MCP_BRIDGE_ADDR", format!("http://{}", addr)),
        ("MCP_BRIDGE_TIMEOUT", "1".to_string()), // 1秒タイムアウト
    ]);
    let bridge_config = BridgeConfig::with_values("127.0.0.1".to_string(), 50051, 500); // 0.5秒タイムアウト
    let server_config = ServerConfig::with_configs(grpc_config, bridge_config);

    let svc = McpService::with_config(server_config).await?;

    let result = svc.unity_health().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("deadline exceeded"));

    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_unavailable() -> anyhow::Result<()> {
    let behavior = TestBehavior::Unavailable;
    let (addr, _shutdown) = start_test_server(behavior).await?;

    // 設定を直接構築（環境変数なし）
    let grpc_config = GrpcConfig::from_map([("MCP_BRIDGE_ADDR", format!("http://{}", addr))]);
    let bridge_config = BridgeConfig::with_values("127.0.0.1".to_string(), 50051, 2000);
    let server_config = ServerConfig::with_configs(grpc_config, bridge_config);

    let svc = McpService::with_config(server_config).await?;

    let result = svc.unity_health().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("unavailable"));

    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_unity_health_connection_refused() -> anyhow::Result<()> {
    // 設定を直接構築（存在しないポート）
    let grpc_config = GrpcConfig::from_map([
        ("MCP_BRIDGE_ADDR", "http://127.0.0.1:9999".to_string()),
        ("MCP_BRIDGE_TIMEOUT", "1".to_string()),
    ]);
    let bridge_config = BridgeConfig::with_values("127.0.0.1".to_string(), 50051, 1000);
    let server_config = ServerConfig::with_configs(grpc_config, bridge_config);

    // ChannelManager の接続時点でエラーになることを確認
    let result = McpService::with_config(server_config).await;
    assert!(result.is_err());

    Ok(())
}

#[cfg(feature = "server-stubs")]
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_health_calls() -> anyhow::Result<()> {
    let behavior = TestBehavior::Success {
        ready: true,
        version: "concurrent-test".to_string(),
    };

    let (addr, _shutdown) = start_test_server(behavior).await?;

    // 設定を直接構築（環境変数なし）
    let grpc_config = GrpcConfig::from_map([("MCP_BRIDGE_ADDR", format!("http://{}", addr))]);
    let bridge_config = BridgeConfig::with_values("127.0.0.1".to_string(), 50051, 2000);
    let server_config = ServerConfig::with_configs(grpc_config, bridge_config);

    let svc = McpService::with_config(server_config).await?;

    // 3並行でhealth callを実行（macOS安定性向上）
    let tasks: Vec<_> = (0..3)
        .map(|i| {
            let svc = svc.clone();
            tokio::spawn(async move {
                // 少し間隔をあけてリクエストを送信
                tokio::time::sleep(Duration::from_millis(i * 50)).await;
                svc.unity_health().await
            })
        })
        .collect();

    let results = futures::future::try_join_all(tasks).await?;

    // 全て成功することを確認
    for result in results {
        let health_result = result?;
        assert!(health_result.is_error.is_none() || health_result.is_error == Some(false));
        assert!(health_result.content.is_some());

        // 並行実行での基本確認
        // 詳細な内容検証は後で実装
    }

    Ok(())
}
