use server::ipc::{client::IpcClient, path::IpcConfig};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_prefab_operations_end_to_end() {
    let ipc_config = IpcConfig::default();
    let client = match IpcClient::connect(ipc_config).await {
        Ok(c) => c,
        Err(_) => {
            println!("Skipping test: Unity Editor not available");
            return;
        }
    };

    let health_timeout = Duration::from_millis(1000);
    let health_result = timeout(health_timeout, client.health(Duration::from_millis(500))).await;
    match health_result {
        Ok(Ok(_)) => {}
        _ => {
            println!("Skipping test: Unity health check failed");
            return;
        }
    }

    let result = client
        .prefab_apply_overrides("NonExistent".to_string(), Duration::from_secs(5))
        .await;

    match result {
        Ok(resp) => {
            println!("Prefab apply overrides ok={}", resp.ok);
        }
        Err(e) => {
            println!("Prefab apply overrides failed: {}", e);
        }
    }
}
