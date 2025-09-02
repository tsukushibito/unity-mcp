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
    let save_res = client
        .scenes_save(save_path.clone(), Duration::from_secs(5))
        .await
        .expect("scenes_save failed");
    assert!(save_res.ok, "scenes_save returned not ok");

    // Open the saved scene
    let open_res = client
        .scenes_open(save_path.clone(), false, Duration::from_secs(5))
        .await
        .expect("scenes_open failed");
    assert!(open_res.ok, "scenes_open returned not ok");

    // Get open scenes list
    let list = client
        .scenes_get_open_scenes(Duration::from_secs(5))
        .await
        .expect("scenes_get_open_scenes failed");
    assert!(!list.scenes.is_empty(), "no open scenes returned");
    if !list.scenes.is_empty() {
        let target = list.scenes[0].clone();
        // Set active scene to first scene
        let set_res = client
            .scenes_set_active_scene(target, Duration::from_secs(5))
            .await
            .expect("scenes_set_active_scene failed");
        assert!(set_res.ok, "scenes_set_active_scene returned not ok");
    }

    // Clean up the saved scene file if it exists
    let project_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let unity_root = project_root.join("bridge");
    let full_path = unity_root.join(&save_path);
    let _ = fs::remove_file(full_path);
}
