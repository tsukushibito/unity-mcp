use server::ipc::{client::IpcClient, path::IpcConfig};
use server::generated::mcp::unity::v1 as pb;
use tokio::{
    net::TcpListener,
    time::Duration,
};
use futures::{SinkExt, StreamExt};
use server::ipc::{codec, framing};

/// Simple TCP echo server for testing IPC handshake
async fn echo_server(port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let mut framed = framing::into_framed(stream);
            
            // Handle handshake
            if let Some(Ok(bytes)) = framed.next().await {
                let env = codec::decode_envelope(bytes.freeze()).unwrap();
                if let Some(pb::ipc_envelope::Kind::Request(req)) = env.kind {
                    if let Some(pb::ipc_request::Payload::Hello(_hello)) = req.payload {
                        // Send welcome response
                        let welcome = pb::IpcWelcome {
                            ok: true,
                            error: String::new(),
                        };
                        let cid = env.correlation_id.clone();
                        let mut resp_env = pb::IpcEnvelope {
                            correlation_id: cid.clone(),
                            kind: None,
                        };
                        resp_env.kind = Some(pb::ipc_envelope::Kind::Response(pb::IpcResponse {
                            correlation_id: cid,
                            payload: Some(pb::ipc_response::Payload::Welcome(welcome)),
                        }));
                        let welcome_bytes = codec::encode_envelope(&resp_env).unwrap();
                        let _ = framed.send(welcome_bytes).await;
                        
                        // Echo loop for other messages
                        while let Some(Ok(bytes)) = framed.next().await {
                            let env = codec::decode_envelope(bytes.freeze()).unwrap();
                            if let Some(pb::ipc_envelope::Kind::Request(req)) = env.kind {
                                if let Some(pb::ipc_request::Payload::Health(_)) = req.payload {
                                    // Respond to health check
                                    let health_resp = pb::HealthResponse {
                                        ready: true,
                                        version: "test-echo-server".to_string(),
                                        status: "ok".to_string(),
                                    };
                                    let cid = env.correlation_id.clone();
                                    let mut resp_env = pb::IpcEnvelope {
                                        correlation_id: cid.clone(),
                                        kind: None,
                                    };
                                    resp_env.kind = Some(pb::ipc_envelope::Kind::Response(pb::IpcResponse {
                                        correlation_id: cid,
                                        payload: Some(pb::ipc_response::Payload::Health(health_resp)),
                                    }));
                                    let resp_bytes = codec::encode_envelope(&resp_env).unwrap();
                                    let _ = framed.send(resp_bytes).await;
                                }
                            }
                        }
                        return;
                    }
                }
            }
        });
    }
    
    Ok(())
}

#[tokio::test]
async fn test_ipc_handshake() -> anyhow::Result<()> {
    let port = 18800; // Use a specific port for testing
    
    // Start echo server
    tokio::spawn(echo_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await; // Let server start
    
    // Configure client to connect to our test server
    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("test-token".to_string()),
        connect_timeout: Duration::from_secs(5),
        call_timeout: Duration::from_secs(5),
    };
    
    // Test connection
    let client = IpcClient::connect(cfg).await?;
    
    // Test events channel
    let _events = client.events();
    
    // Verify client was created successfully  
    // Note: client doesn't implement Debug, so we just verify it exists
    let _ = &client;
    
    Ok(())
}

#[tokio::test]
async fn test_health_request_response() -> anyhow::Result<()> {
    let port = 18801; // Use different port to avoid conflicts
    
    // Start echo server
    tokio::spawn(echo_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await; // Let server start
    
    // Configure client
    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("test-token".to_string()),
        connect_timeout: Duration::from_secs(5),
        call_timeout: Duration::from_secs(5),
    };
    
    // Connect and test health
    let client = IpcClient::connect(cfg).await?;
    let health_resp = client.health(Duration::from_secs(5)).await?;
    
    assert_eq!(health_resp.status, "ok");
    
    Ok(())
}

#[tokio::test]
async fn test_connection_timeout() {
    let cfg = IpcConfig {
        endpoint: Some("tcp://127.0.0.1:99999".to_string()), // Non-existent endpoint
        token: None,
        connect_timeout: Duration::from_millis(100),
        call_timeout: Duration::from_secs(1),
    };
    
    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());
}