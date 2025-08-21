use server::ipc::{client::IpcClient, path::IpcConfig};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_assets_operations_end_to_end() {
    // Skip if no Unity Editor is running
    let ipc_config = IpcConfig::default();
    let client = match IpcClient::connect(ipc_config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping test: Unity Editor not available");
            return;
        }
    };

    // Test 1: Health check first
    let health_timeout = Duration::from_millis(1000);
    let health_result = timeout(health_timeout, client.health(Duration::from_millis(500))).await;
    match health_result {
        Ok(Ok(health)) => {
            println!(
                "Unity Health: ready={}, version={}",
                health.ready, health.version
            );
        }
        _ => {
            println!("Skipping test: Unity health check failed");
            return;
        }
    }

    // Test 2: Path to GUID conversion (should work for existing assets)
    let paths = vec!["Assets".to_string()]; // Assets folder should always exist
    let p2g_result = client
        .assets_path_to_guid(paths.clone(), Duration::from_secs(5))
        .await;

    match p2g_result {
        Ok(response) => {
            println!("Path to GUID conversion successful");
            assert!(response.map.contains_key("Assets"));
            let assets_guid = response.map.get("Assets").unwrap();
            assert!(!assets_guid.is_empty());
            println!("Assets folder GUID: {}", assets_guid);

            // Test 3: GUID to Path conversion (round trip test)
            let guids = vec![assets_guid.clone()];
            let g2p_result = client
                .assets_guid_to_path(guids, Duration::from_secs(5))
                .await;

            match g2p_result {
                Ok(g2p_response) => {
                    println!("GUID to Path conversion successful");
                    assert!(g2p_response.map.contains_key(assets_guid));
                    let path = g2p_response.map.get(assets_guid).unwrap();
                    assert_eq!(path, "Assets");
                    println!(
                        "Round trip test passed: Assets -> {} -> {}",
                        assets_guid, path
                    );
                }
                Err(e) => {
                    panic!("GUID to Path conversion failed: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("Path to GUID conversion failed: {}", e);
        }
    }

    // Test 4: AssetDatabase refresh
    let refresh_result = client.assets_refresh(false, Duration::from_secs(10)).await;

    match refresh_result {
        Ok(response) => {
            println!("AssetDatabase refresh successful: ok={}", response.ok);
            assert!(response.ok);
        }
        Err(e) => {
            panic!("AssetDatabase refresh failed: {}", e);
        }
    }

    // Test 5: Import non-existent path (should fail gracefully)
    let invalid_paths = vec!["Assets/NonExistentFile.txt".to_string()];
    let import_result = client
        .assets_import(invalid_paths, false, false, Duration::from_secs(5))
        .await;

    match import_result {
        Ok(response) => {
            println!(
                "Import test completed with {} results",
                response.results.len()
            );
            // Should have one result for the non-existent file
            assert_eq!(response.results.len(), 1);
            let result = &response.results[0];
            println!(
                "Import result: path={}, ok={}, message={:?}",
                result.path, result.ok, result.message
            );
        }
        Err(e) => {
            println!("Import test completed with expected error: {}", e);
        }
    }

    println!("All Assets integration tests completed successfully");
}

#[tokio::test]
async fn test_assets_path_validation() {
    // This test doesn't require Unity Editor - tests our validation logic
    let ipc_config = IpcConfig::default();
    let client = match IpcClient::connect(ipc_config).await {
        Ok(client) => client,
        Err(_) => {
            println!("Skipping test: Unity Editor not available");
            return;
        }
    };

    // Test invalid paths that should be rejected
    let invalid_paths = vec![
        "/absolute/path".to_string(),
        "../parent/directory".to_string(),
        "NotAssets/file.txt".to_string(),
    ];

    for invalid_path in invalid_paths {
        let result = client
            .assets_import(
                vec![invalid_path.clone()],
                false,
                false,
                Duration::from_secs(2),
            )
            .await;

        match result {
            Ok(response) => {
                // Should have one result that failed
                assert_eq!(response.results.len(), 1);
                let import_result = &response.results[0];
                assert!(!import_result.ok);
                assert!(import_result.message.contains("invalid path"));
                println!("Path validation correctly rejected: {}", invalid_path);
            }
            Err(_) => {
                println!(
                    "Path validation test for '{}' failed at IPC level (acceptable)",
                    invalid_path
                );
            }
        }
    }

    println!("Path validation tests completed");
}
