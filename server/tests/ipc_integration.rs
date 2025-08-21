use futures::{SinkExt, StreamExt};
use server::generated::mcp::unity::v1 as pb;
use server::ipc::{client::IpcClient, path::IpcConfig};
use server::ipc::{codec, framing};
use tokio::{net::TcpListener, time::Duration};

/// T01-compliant mock Unity server for testing IPC handshake
async fn mock_unity_server(port: u16) -> anyhow::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(async move {
            let mut framed = framing::into_framed(stream);

            // T01: Handle handshake with IpcControl
            if let Some(Ok(bytes)) = framed.next().await
                && let Ok(control) = codec::decode_control(bytes.freeze())
                && let Some(pb::ipc_control::Kind::Hello(hello)) = control.kind {
                // Basic validation
                if hello.token.is_empty() {
                    // Send reject
                    let reject = pb::IpcReject {
                        code: pb::ipc_reject::Code::Unauthenticated as i32,
                        message: "missing token".to_string(),
                    };
                    let reject_control = pb::IpcControl {
                        kind: Some(pb::ipc_control::Kind::Reject(reject)),
                    };
                    let reject_bytes = codec::encode_control(&reject_control).unwrap();
                    let _ = framed.send(reject_bytes).await;
                    return;
                }

                // Send welcome response
                let welcome = pb::IpcWelcome {
                    ipc_version: hello.ipc_version,
                    accepted_features: hello.features,
                    schema_hash: hello.schema_hash,
                    server_name: "test-unity-server".to_string(),
                    server_version: "0.1.0".to_string(),
                    editor_version: "Unity 6000.0.test".to_string(),
                    session_id: "test-session-123".to_string(),
                    meta: std::collections::HashMap::from([(
                        "platform".to_string(),
                        "test".to_string(),
                    )]),
                };
                let welcome_control = pb::IpcControl {
                    kind: Some(pb::ipc_control::Kind::Welcome(welcome)),
                };
                let welcome_bytes = codec::encode_control(&welcome_control).unwrap();
                let _ = framed.send(welcome_bytes).await;

                // Handle regular envelope messages
                while let Some(Ok(bytes)) = framed.next().await {
                    if let Ok(env) = codec::decode_envelope(bytes.freeze())
                        && let Some(pb::ipc_envelope::Kind::Request(req)) = env.kind
                        && let Some(pb::ipc_request::Payload::Health(_)) = req.payload {
                        // Respond to health check
                        let health_resp = pb::HealthResponse {
                            ready: true,
                            version: "test-unity-server".to_string(),
                            status: "ok".to_string(),
                        };
                        let cid = env.correlation_id.clone();
                        let mut resp_env = pb::IpcEnvelope {
                            correlation_id: cid.clone(),
                            kind: None,
                        };
                        resp_env.kind = Some(pb::ipc_envelope::Kind::Response(
                            pb::IpcResponse {
                                correlation_id: cid,
                                payload: Some(pb::ipc_response::Payload::Health(
                                    health_resp,
                                )),
                            },
                        ));
                        let resp_bytes = codec::encode_envelope(&resp_env).unwrap();
                        let _ = framed.send(resp_bytes).await;
                    }
                }
            }
        });
    }

    Ok(())
}

#[tokio::test]
async fn test_t01_basic_handshake_success() -> anyhow::Result<()> {
    let port = 18800; // Use a specific port for testing

    // Start T01-compliant mock server
    tokio::spawn(mock_unity_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await; // Let server start

    // Configure client to connect to our test server
    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("test-token".to_string()),
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
    };

    // Test T01 handshake
    let client = IpcClient::connect(cfg).await?;

    // Verify handshake completed by testing health
    let health = client.health(Duration::from_secs(1)).await?;
    assert_eq!(health.status, "ok");
    assert!(health.ready);

    Ok(())
}

#[tokio::test]
async fn test_t01_basic_token_rejection() -> anyhow::Result<()> {
    let port = 18801; // Use different port to avoid conflicts

    // Start T01-compliant mock server
    tokio::spawn(mock_unity_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await; // Let server start

    // Configure client with empty token (should be rejected)
    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("".to_string()), // Empty token should trigger rejection
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
    };

    // Should fail with handshake error
    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(error_msg.contains("handshake failed"));
        assert!(error_msg.contains("missing token"));
    }

    Ok(())
}

#[tokio::test]
async fn test_connection_timeout() {
    let cfg = IpcConfig {
        endpoint: Some("tcp://127.0.0.1:99999".to_string()), // Non-existent endpoint
        token: None,
        project_root: None,
        connect_timeout: Duration::from_millis(100),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(3),
        call_timeout: Duration::from_secs(1),
    };

    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());
}
