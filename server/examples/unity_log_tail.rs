// Tail Unity log events (~10s) via Direct IPC
// Run with: cargo run --example unity_log_tail
use server::generated::mcp::unity::v1 as pb;
use server::ipc::{client::IpcClient, features::FeatureFlag, path::IpcConfig};
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("[unity_log_tail] Connecting to Unity EditorIpcServer...");

    // Minimal configuration. Adjust token/endpoint if needed.
    let cfg = IpcConfig {
        endpoint: Some("tcp://127.0.0.1:7777".to_string()),
        token: Some("test-token".to_string()),
        connect_timeout: Duration::from_secs(10),
        handshake_timeout: Duration::from_secs(5),
        total_handshake_timeout: Duration::from_secs(15),
        call_timeout: Duration::from_secs(10),
        max_reconnect_attempts: Some(3),
    };

    // Connect
    let client = match IpcClient::connect(cfg).await {
        Ok(c) => {
            println!("[OK] Connected and handshake completed");
            c
        }
        Err(e) => {
            println!("[ERR] Failed to connect: {e}");
            println!("Hint: Ensure Unity is running and MCP.IpcToken is set in EditorUserSettings");
            std::process::exit(1);
        }
    };

    // Check feature negotiation
    if !client.has_feature(FeatureFlag::EventsLog).await {
        println!("[WARN] 'events.log' feature not negotiated. Logs may not be delivered.");
    } else {
        println!("[OK] 'events.log' negotiated");
    }

    // Subscribe to events
    let mut rx = client.events();

    // Counters
    let mut n_info = 0usize;
    let mut n_warn = 0usize;
    let mut n_error = 0usize;
    let mut n_debug = 0usize;
    let mut n_trace = 0usize;
    let mut n_total_logs = 0usize;

    // Tail duration: env MCP_TAIL_SECS or first CLI arg (seconds), default 10s
    let tail_secs: u64 = std::env::var("MCP_TAIL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .or_else(|| std::env::args().nth(1).and_then(|s| s.parse().ok()))
        .unwrap_or(10);

    println!(
        "[unity_log_tail] Tailing logs for ~{}s. Interact with the Editor to generate logs...",
        tail_secs
    );

    let until = time::Instant::now() + Duration::from_secs(tail_secs);
    let mut hb_interval = time::interval(Duration::from_secs(3));
    loop {
        tokio::select! {
            biased;
            _ = time::sleep_until(until) => {
                break;
            }
            _ = hb_interval.tick() => {
                // Lightweight heartbeat to help reconnection supervisor detect dead connections
                let _ = client.health(Duration::from_secs(1)).await;
            }
            msg = rx.recv() => {
                match msg {
                    Ok(pb::IpcEvent { payload: Some(pb::ipc_event::Payload::Log(log)), .. }) => {
                        use std::convert::TryFrom;
                        use pb::log_event::Level;
                        n_total_logs += 1;
                        match Level::try_from(log.level).unwrap_or(Level::Info) {
                            Level::Error => { n_error += 1; println!("[ERROR] {} :: {}", log.category, log.message); },
                            Level::Warn  => { n_warn  += 1; println!("[WARN ] {} :: {}", log.category, log.message); },
                            Level::Info  => { n_info  += 1; println!("[INFO ] {} :: {}", log.category, log.message); },
                            Level::Debug => { n_debug += 1; println!("[DEBUG] {} :: {}", log.category, log.message); },
                            Level::Trace => { n_trace += 1; println!("[TRACE] {} :: {}", log.category, log.message); },
                        }
                    }
                    Ok(_) => { /* ignore non-log events here */ }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        println!("[WARN] Event receiver lagged, skipped {skipped} events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        println!("[ERR] Event stream closed");
                        break;
                    }
                }
            }
        }
    }

    println!(
        "\n[summary] total_logs={n_total_logs} info={n_info} warn={n_warn} error={n_error} debug={n_debug} trace={n_trace}"
    );
    if n_total_logs == 0 {
        println!("[WARN] No logs received in the sampling window");
    }
    if n_error > 0 {
        std::process::exit(1);
    }

    Ok(())
}
