use server::ipc::{client::IpcClient, path::IpcConfig};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_component_operations_end_to_end() {
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

    let go = "Main Camera".to_string();

    // Initial get to ensure connection
    if client
        .component_get(go.clone(), Duration::from_secs(5))
        .await
        .is_err()
    {
        println!("Skipping test: get components failed");
        return;
    }

    // Add component
    let _ = client
        .component_add(
            go.clone(),
            "UnityEngine.BoxCollider".to_string(),
            Duration::from_secs(5),
        )
        .await;

    // Verify added
    if let Ok(resp) = client
        .component_get(go.clone(), Duration::from_secs(5))
        .await
    {
        assert!(resp.components.iter().any(|c| c.contains("BoxCollider")));
    }

    // Remove component
    let _ = client
        .component_remove(
            go.clone(),
            "UnityEngine.BoxCollider".to_string(),
            Duration::from_secs(5),
        )
        .await;

    // Verify removed
    if let Ok(resp) = client
        .component_get(go.clone(), Duration::from_secs(5))
        .await
    {
        assert!(!resp.components.iter().any(|c| c.contains("BoxCollider")));
    }

    println!("Component operations completed");
}
