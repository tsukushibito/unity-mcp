use server::ipc::{client::IpcClient, path::IpcConfig};
use std::{fs, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;

#[tokio::test]
async fn test_scene_operations_end_to_end() {
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
        Ok(Ok(h)) => {
            println!("Unity Health: ready={}, version={}", h.ready, h.version);
        }
        _ => {
            println!("Skipping test: Unity health check failed");
            return;
        }
    }

    // Save current active scene to a Temp path to avoid polluting Assets
    let save_path = format!("Temp/TestScene-{}.unity", Uuid::new_v4());
    let _ = client
        .scenes_save(save_path.clone(), Duration::from_secs(5))
        .await;

    // Open the saved scene
    let open_res = client
        .scenes_open(save_path.clone(), false, Duration::from_secs(5))
        .await;
    match open_res {
        Ok(r) => println!("Opened scene: ok={}", r.ok),
        Err(e) => println!("Scene open failed: {}", e),
    }

    // Get open scenes list
    match client.scenes_get_open_scenes(Duration::from_secs(5)).await {
        Ok(list) => {
            println!("Open scenes: {:?}", list.scenes);
            if !list.scenes.is_empty() {
                let target = list.scenes[0].clone();
                // Set active scene to first scene
                let _ = client
                    .scenes_set_active_scene(target, Duration::from_secs(5))
                    .await;
            }
        }
        Err(e) => println!("Get open scenes failed: {}", e),
    }

    // Clean up the saved scene file if it exists
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let unity_root = project_root.join("bridge");
    let full_path = unity_root.join(&save_path);
    let _ = fs::remove_file(full_path);
}
