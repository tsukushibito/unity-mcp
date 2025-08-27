use futures::{SinkExt, StreamExt};
use server::generated::mcp::unity::v1 as pb;
use server::ipc::{
    client::{IpcClient, IpcError},
    features::{FeatureFlag, FeatureSet},
    path::IpcConfig,
};
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
                && let Some(pb::ipc_control::Kind::Hello(hello)) = control.kind
            {
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

                // For test purposes, reject "wrong-token"
                if hello.token == "wrong-token" {
                    let reject = pb::IpcReject {
                        code: pb::ipc_reject::Code::Unauthenticated as i32,
                        message: "invalid token".to_string(),
                    };
                    let reject_control = pb::IpcControl {
                        kind: Some(pb::ipc_control::Kind::Reject(reject)),
                    };
                    let reject_bytes = codec::encode_control(&reject_control).unwrap();
                    let _ = framed.send(reject_bytes).await;
                    return;
                }

                // Schema hash validation (pure validation - no token dependency)
                if hello.schema_hash != codec::schema_hash() {
                    let reject = pb::IpcReject {
                        code: pb::ipc_reject::Code::FailedPrecondition as i32,
                        message: "Schema hash mismatch. Regenerate C# SCHEMA_HASH from server (CI).".to_string(),
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
                        && let Some(pb::ipc_request::Payload::Health(_)) = req.payload
                    {
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
                        resp_env.kind = Some(pb::ipc_envelope::Kind::Response(pb::IpcResponse {
                            correlation_id: cid,
                            payload: Some(pb::ipc_response::Payload::Health(health_resp)),
                        }));
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
        max_reconnect_attempts: Some(1), // Don't retry for test
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
        max_reconnect_attempts: Some(1), // Don't retry for test
    };

    // Should fail with authentication error
    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());

    if let Err(e) = result {
        // With the new error handling, should get Authentication error
        assert!(matches!(e, IpcError::Authentication(_)));
        let error_msg = e.to_string();
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
        max_reconnect_attempts: Some(1), // Don't retry for test
    };

    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e, IpcError::ConnectTimeout));
    }
}

#[tokio::test]
async fn test_handshake_authentication_failure() -> anyhow::Result<()> {
    let port = 18802;

    // Start mock server that will reject invalid tokens
    tokio::spawn(mock_unity_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await;

    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("wrong-token".to_string()),
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
        max_reconnect_attempts: Some(1),
    };

    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e, IpcError::Authentication(_)));
        let error_msg = e.to_string();
        assert!(error_msg.contains("invalid token") || error_msg.contains("missing token"));
    }

    Ok(())
}

#[tokio::test]
async fn test_handshake_version_incompatible() -> anyhow::Result<()> {
    let port = 18803;

    // Start mock server with version check
    tokio::spawn(mock_unity_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await;

    // TODO: Modify IpcClient to allow version override for testing
    // For now, this test validates the framework is in place

    Ok(())
}

#[tokio::test]
async fn test_connect_with_retry_success() -> anyhow::Result<()> {
    let port = 18804;

    // Start server after a delay to test retry logic
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await; // Delay server start
        let _ = mock_unity_server(port).await;
    });

    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("test-token".to_string()),
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_millis(200),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
        max_reconnect_attempts: Some(5), // Allow retries
    };

    // Should succeed after retries
    let client = IpcClient::connect_with_retry(cfg).await?;
    let health = client.health(Duration::from_secs(1)).await?;
    assert_eq!(health.status, "ok");

    Ok(())
}

#[tokio::test]
async fn test_connect_with_retry_permanent_failure() -> anyhow::Result<()> {
    let port = 18805;

    // Start server that rejects authentication
    tokio::spawn(mock_unity_server(port));
    tokio::time::sleep(Duration::from_millis(100)).await;

    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("".to_string()), // Empty token will be permanently rejected
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
        max_reconnect_attempts: Some(3),
    };

    // Should fail immediately without retries for authentication errors
    let result = IpcClient::connect_with_retry(cfg).await;
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(matches!(e, IpcError::Authentication(_)));
    }

    Ok(())
}

/// Mock Unity server with configurable feature support
struct MockUnityServer {
    supported_features: Vec<String>,
    override_schema_hash_for_test: Option<Vec<u8>>,
}

impl MockUnityServer {
    pub fn new() -> Self {
        Self {
            supported_features: vec![
                "assets.basic".to_string(),
                "build.min".to_string(),
                "events.log".to_string(),
                "ops.progress".to_string(),
            ],
            override_schema_hash_for_test: None,
        }
    }

    pub fn with_supported_features(mut self, features: Vec<&str>) -> Self {
        self.supported_features = features.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_fake_schema_hash_for_test(mut self, fake_hash: Vec<u8>) -> Self {
        self.override_schema_hash_for_test = Some(fake_hash);
        self
    }

    pub async fn start(&self, port: u16) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        let supported_features = self.supported_features.clone();
        let override_schema_hash = self.override_schema_hash_for_test.clone();

        while let Ok((stream, _)) = listener.accept().await {
            let features_clone = supported_features.clone();
            let hash_override = override_schema_hash.clone();
            tokio::spawn(async move {
                let mut framed = framing::into_framed(stream);

                // Handle handshake
                if let Some(Ok(bytes)) = framed.next().await
                    && let Ok(control) = codec::decode_control(bytes.freeze())
                    && let Some(pb::ipc_control::Kind::Hello(hello)) = control.kind
                {
                    // Schema hash validation (responsibility separated from token)
                    let expected_schema_hash = if let Some(fake_hash) = hash_override {
                        fake_hash
                    } else {
                        codec::schema_hash().to_vec()
                    };
                    
                    if hello.schema_hash != expected_schema_hash {
                        let reject = pb::IpcReject {
                            code: pb::ipc_reject::Code::FailedPrecondition as i32,
                            message: "Schema hash mismatch. Regenerate C# SCHEMA_HASH from server (CI).".to_string(),
                        };
                        let reject_control = pb::IpcControl {
                            kind: Some(pb::ipc_control::Kind::Reject(reject)),
                        };
                        let reject_bytes = codec::encode_control(&reject_control).unwrap();
                        let _ = framed.send(reject_bytes).await;
                        return;
                    }

                    // Feature negotiation - intersection
                    let client_features: Vec<String> = hello.features;
                    let accepted_features: Vec<String> = client_features
                        .into_iter()
                        .filter(|f| features_clone.contains(f))
                        .collect();

                    let welcome = pb::IpcWelcome {
                        ipc_version: hello.ipc_version,
                        accepted_features,
                        schema_hash: hello.schema_hash,
                        server_name: "mock-unity-server".to_string(),
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
                }
            });
        }

        Ok(())
    }
}

fn test_config(port: u16) -> IpcConfig {
    IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("test-token".to_string()),
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
        max_reconnect_attempts: Some(1),
    }
}

#[tokio::test]
async fn test_feature_negotiation_intersection() -> anyhow::Result<()> {
    let port = 18901;

    // Start mock server with limited feature support
    let server = MockUnityServer::new().with_supported_features(vec!["assets.basic", "events.log"]);
    tokio::spawn(async move {
        let _ = server.start(port).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = IpcClient::connect(test_config(port)).await?;

    let features = client.get_negotiated_features().await;
    let feature_strings = features.to_strings();

    // Should contain features that both client and server support
    assert!(feature_strings.contains(&"assets.basic".to_string()));
    assert!(feature_strings.contains(&"events.log".to_string()));

    // Should NOT contain features not supported by mock server
    assert!(!feature_strings.contains(&"build.min".to_string()));
    assert!(!feature_strings.contains(&"ops.progress".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_feature_dependent_operation_rejection() -> anyhow::Result<()> {
    let port = 18902;

    // Start mock server with NO assets.basic support
    let server = MockUnityServer::new().with_supported_features(vec!["events.log"]);
    tokio::spawn(async move {
        let _ = server.start(port).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = IpcClient::connect(test_config(port)).await?;

    // Try to call assets_import - should fail due to missing feature
    let result = client
        .assets_import(vec![], false, false, Duration::from_secs(1))
        .await;
    assert!(matches!(result, Err(IpcError::UnsupportedFeature(_))));

    if let Err(IpcError::UnsupportedFeature(msg)) = result {
        assert!(msg.contains("assets.basic"));
    }

    Ok(())
}

#[tokio::test]
async fn test_unknown_features_filtered_during_negotiation() -> anyhow::Result<()> {
    let client_features = vec!["assets.basic".to_string(), "unknown.feature".to_string()];
    let feature_set = FeatureSet::from_strings(&client_features);

    // Unknown features should be filtered out during negotiation
    let result_strings = feature_set.to_strings();
    assert!(!result_strings.contains(&"unknown.feature".to_string()));
    assert!(feature_set.contains(&FeatureFlag::AssetsBasic));

    Ok(())
}

#[tokio::test]
async fn test_feature_string_normalization() -> anyhow::Result<()> {
    let normalized = FeatureFlag::normalize_string(" Assets.Basic ");
    assert_eq!(normalized, "assets.basic");

    let feature = FeatureFlag::from_string(&normalized);
    assert_eq!(feature, FeatureFlag::AssetsBasic);

    Ok(())
}

#[tokio::test]
async fn test_feature_set_intersection() -> anyhow::Result<()> {
    let mut client_features = FeatureSet::new();
    client_features.insert(FeatureFlag::AssetsBasic);
    client_features.insert(FeatureFlag::BuildMin);
    client_features.insert(FeatureFlag::EventsLog);

    let mut server_features = FeatureSet::new();
    server_features.insert(FeatureFlag::AssetsBasic);
    server_features.insert(FeatureFlag::EventsLog);
    server_features.insert(FeatureFlag::OpsProgress);

    let negotiated = client_features.intersect(&server_features);
    assert!(negotiated.contains(&FeatureFlag::AssetsBasic));
    assert!(negotiated.contains(&FeatureFlag::EventsLog));
    assert!(!negotiated.contains(&FeatureFlag::BuildMin));
    assert!(!negotiated.contains(&FeatureFlag::OpsProgress));

    Ok(())
}

#[tokio::test]
async fn test_supported_by_client() -> anyhow::Result<()> {
    let client_features = FeatureSet::supported_by_client();
    assert!(client_features.contains(&FeatureFlag::AssetsBasic));
    assert!(client_features.contains(&FeatureFlag::BuildMin));
    assert!(client_features.contains(&FeatureFlag::EventsLog));
    assert!(client_features.contains(&FeatureFlag::OpsProgress));
    assert!(!client_features.contains(&FeatureFlag::AssetsAdvanced));

    Ok(())
}

#[tokio::test]
async fn test_schema_hash_mismatch_rejection() -> anyhow::Result<()> {
    let port = 18806;

    // Create fake schema hash - different from expected
    let fake_hash = vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 
                         0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
                         0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                         0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];

    // Start mock server with fake schema hash for test
    let server = MockUnityServer::new().with_fake_schema_hash_for_test(fake_hash);
    tokio::spawn(async move {
        let _ = server.start(port).await;
    });
    tokio::time::sleep(Duration::from_millis(100)).await;

    let cfg = IpcConfig {
        endpoint: Some(format!("tcp://127.0.0.1:{}", port)),
        token: Some("test-token".to_string()), // Valid token - schema hash is the issue
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(5),
        handshake_timeout: Duration::from_secs(2),
        total_handshake_timeout: Duration::from_secs(8),
        call_timeout: Duration::from_secs(5),
        max_reconnect_attempts: Some(1),
    };

    let result = IpcClient::connect(cfg).await;
    assert!(result.is_err());

    if let Err(e) = result {
        // Should get a FAILED_PRECONDITION error for schema mismatch
        let error_msg = e.to_string();
        assert!(error_msg.contains("Schema hash mismatch") || error_msg.contains("FAILED_PRECONDITION"));
    }

    Ok(())
}
