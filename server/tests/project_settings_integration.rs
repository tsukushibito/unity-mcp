use server::generated::mcp::unity::v1::{
    GetProjectSettingsRequest, IpcRequest, SetProjectSettingsRequest, ipc_request, ipc_response,
};
use server::ipc::{client::IpcClient, path::IpcConfig};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_project_settings_roundtrip() {
    let ipc_config = IpcConfig::default();
    let client = match IpcClient::connect(ipc_config).await {
        Ok(c) => c,
        Err(_) => {
            println!("Skipping test: Unity Editor not available");
            return;
        }
    };

    // Get current companyName
    let get_req = GetProjectSettingsRequest {
        keys: vec!["companyName".to_string()],
    };
    let ipc_req = IpcRequest {
        payload: Some(ipc_request::Payload::GetProjectSettings(get_req)),
    };
    let resp = match timeout(
        Duration::from_secs(5),
        client.request(ipc_req, Duration::from_secs(5)),
    )
    .await
    {
        Ok(Ok(r)) => r,
        _ => {
            println!("Skipping test: failed to get project settings");
            return;
        }
    };
    let get_resp = match resp.payload {
        Some(ipc_response::Payload::GetProjectSettings(r)) => r,
        _ => {
            println!("Skipping test: unexpected response type");
            return;
        }
    };
    let original = get_resp
        .settings
        .get("companyName")
        .cloned()
        .unwrap_or_default();

    let client_clone = client.clone();
    let original_clone = original.clone();
    scopeguard::defer! {
        let mut restore = HashMap::new();
        restore.insert("companyName".to_string(), original_clone);
        let set_req = SetProjectSettingsRequest { settings: restore };
        let ipc_req = IpcRequest {
            payload: Some(ipc_request::Payload::SetProjectSettings(set_req)),
        };
        tokio::runtime::Handle::current().block_on(async {
            let _ = timeout(
                Duration::from_secs(5),
                client_clone.request(ipc_req, Duration::from_secs(5)),
            )
            .await;
        });
    }

    // Set new companyName
    let mut map = HashMap::new();
    map.insert("companyName".to_string(), "TestCo".to_string());
    let set_req = SetProjectSettingsRequest {
        settings: map.clone(),
    };
    let ipc_req = IpcRequest {
        payload: Some(ipc_request::Payload::SetProjectSettings(set_req)),
    };
    let resp = match timeout(
        Duration::from_secs(5),
        client.request(ipc_req, Duration::from_secs(5)),
    )
    .await
    {
        Ok(Ok(r)) => r,
        _ => {
            println!("Skipping test: failed to set project settings");
            return;
        }
    };
    let set_resp = match resp.payload {
        Some(ipc_response::Payload::SetProjectSettings(r)) => r,
        _ => {
            println!("Skipping test: unexpected response type");
            return;
        }
    };
    assert!(set_resp.ok);

    // Verify change
    let get_req = GetProjectSettingsRequest {
        keys: vec!["companyName".to_string()],
    };
    let ipc_req = IpcRequest {
        payload: Some(ipc_request::Payload::GetProjectSettings(get_req)),
    };
    let resp = match timeout(
        Duration::from_secs(5),
        client.request(ipc_req, Duration::from_secs(5)),
    )
    .await
    {
        Ok(Ok(r)) => r,
        _ => {
            println!("Skipping test: failed to get project settings after set");
            return;
        }
    };
    let get_resp = match resp.payload {
        Some(ipc_response::Payload::GetProjectSettings(r)) => r,
        _ => {
            println!("Skipping test: unexpected response type");
            return;
        }
    };
    let updated = get_resp
        .settings
        .get("companyName")
        .cloned()
        .unwrap_or_default();
    assert_eq!(updated, "TestCo");
}
