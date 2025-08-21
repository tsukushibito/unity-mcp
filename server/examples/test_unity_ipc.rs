// Example to test Unity IPC Server from Rust
// Run with: cargo run --example test_unity_ipc
use server::ipc::{client::IpcClient, path::IpcConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Testing connection to Unity EditorIpcServer...");

    // Configure client to connect to Unity TCP server on 127.0.0.1:7777
    let cfg = IpcConfig {
        endpoint: Some("tcp://127.0.0.1:7777".to_string()),
        token: Some("test-token".to_string()),
        project_root: Some(".".to_string()),
        connect_timeout: Duration::from_secs(10),
        handshake_timeout: Duration::from_secs(5),
        total_handshake_timeout: Duration::from_secs(15),
        call_timeout: Duration::from_secs(10),
    };

    println!("Attempting to connect to tcp://127.0.0.1:7777...");

    // Test connection and handshake
    match IpcClient::connect(cfg).await {
        Ok(client) => {
            println!("✓ Successfully connected to Unity IPC server!");
            println!("✓ Handshake completed successfully!");

            // Test health request
            println!("Sending health request...");
            match client.health(Duration::from_secs(5)).await {
                Ok(health) => {
                    println!("✓ Health response received:");
                    println!("  - Ready: {}", health.ready);
                    println!("  - Version: {}", health.version);
                    println!("  - Status: {}", health.status);
                }
                Err(e) => {
                    println!("✗ Health request failed: {}", e);
                    return Err(e.into());
                }
            }

            println!("\n🎉 All tests passed! Unity IPC server is working correctly.");
        }
        Err(e) => {
            println!("✗ Failed to connect to Unity IPC server: {}", e);
            println!("\nMake sure Unity Editor is running with the MCP bridge package loaded.");
            println!("The Unity EditorIpcServer should be listening on 127.0.0.1:7777");
            return Err(e.into());
        }
    }

    Ok(())
}
