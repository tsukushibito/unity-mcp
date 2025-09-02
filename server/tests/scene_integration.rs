use server::ipc::{client::IpcClient, path::IpcConfig};
use std::{fs, path::Path, time::Duration};
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

    // Save current active scene under Assets so Unity accepts the path
    let temp_dir = "Assets/__codex_tmp";
    let save_path = format!("{}/TestScene-{}.unity", temp_dir, Uuid::new_v4());
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let unity_root = project_root.join("bridge");
    let full_path = unity_root.join(&save_path);
    if let Some(parent) = full_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
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

    // Clean up the saved scene directory and its meta file if they exist
    if let Some(parent) = full_path.parent() {
        let _ = fs::remove_dir_all(parent);
        let _ = fs::remove_file(parent.with_extension("meta"));
    }
}
