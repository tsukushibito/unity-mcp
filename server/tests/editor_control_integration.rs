use server::ipc::{client::IpcClient, path::IpcConfig};
use std::time::Duration;

#[tokio::test]
async fn test_editor_menu_and_window_operations() {
    let ipc_config = IpcConfig::default();
    let client = match IpcClient::connect(ipc_config).await {
        Ok(c) => c,
        Err(_) => {
            println!("Skipping test: Unity Editor not available");
            return;
        }
    };

    // Execute a simple menu item; should succeed for valid menu path
    let menu_result = client
        .execute_menu_item("File/New Scene".to_string(), Duration::from_secs(5))
        .await;
    match menu_result {
        Ok(resp) => println!("ExecuteMenuItem ok={}", resp.ok),
        Err(e) => println!("ExecuteMenuItem failed: {}", e),
    }

    // Focus SceneView window by type name
    let focus_result = client
        .focus_window("UnityEditor.SceneView".to_string(), Duration::from_secs(5))
        .await;
    match focus_result {
        Ok(resp) => println!("FocusWindow ok={}", resp.ok),
        Err(e) => println!("FocusWindow failed: {}", e),
    }
}
