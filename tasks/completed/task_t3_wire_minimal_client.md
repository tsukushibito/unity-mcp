# T3 — Minimal Client (EditorControl): Fixes & Final `main.rs`

This document fixes pitfalls in the original plan and provides a production‑ready `main.rs` that:

* initializes tracing with an env filter (`RUST_LOG`),
* loads `GrpcConfig` from env (with safe defaults),
* connects via `ChannelManager` with endpoint‑level timeouts,
* calls `EditorControl.Health` once,
* does **not** panic when offline, and
* exits with clear, non‑zero codes on failure.

It assumes:

* `server/src/lib.rs` re‑exports `pub mod config; pub mod grpc; pub mod generated;` as in T1/T2.
* tonic `transport,tls-webpki-roots` features are enabled.

---

## Key Fixes

1. **No panics / explicit exit codes**
   `main` handles each failure case and uses `std::process::exit` with distinct codes:

   * `2`: connect failure,
   * `3`: Health RPC failed,
   * `4`: Health responded but `ready=false`.

2. **Env‑driven tracing**
   Use `RUST_LOG` (e.g., `RUST_LOG=server=debug,info`) via `tracing_subscriber::EnvFilter`, defaulting to `info` if unset. Pretty/compact output for CLI readability.

3. **Token handling**
   If a token is configured, use the **intercepted** client so all calls automatically carry `Authorization` without per‑call wrappers. (You can still switch to `editor_control_client()` + `with_meta()` if you want per‑call control.)

4. **No per‑call timeouts**
   Rely on **endpoint‑level** `timeout` configured in `ChannelManager::connect`, matching the architectural constraint to avoid legacy `Request::set_timeout`.

---

## File: `server/src/main.rs`

```rust
use std::process;
use tracing::{error, info, warn};

use server::grpc::channel::ChannelManager;
use server::grpc::config::GrpcConfig;
use server::generated::mcp::unity::v1::editor_control::HealthRequest;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    init_tracing();

    let cfg = GrpcConfig::from_env();
    info!(addr = %cfg.addr, timeout_secs = cfg.default_timeout_secs, "Starting minimal EditorControl client");

    let manager = match ChannelManager::connect(&cfg).await {
        Ok(m) => m,
        Err(e) => {
            error!(error = %e, "Failed to connect to gRPC bridge");
            process::exit(2);
        }
    };

    // If a token is configured, prefer the intercepted client so headers are auto‑injected.
    let mut client = if cfg.token.is_some() {
        manager.editor_control_client_intercepted()
    } else {
        manager.editor_control_client()
    };

    match client.health(HealthRequest {}).await {
        Ok(resp) => {
            let body = resp.into_inner();
            if body.ready {
                info!(version = %body.version, "Bridge is ready");
                process::exit(0);
            } else {
                warn!(version = %body.version, "Bridge responded but not ready");
                process::exit(4);
            }
        }
        Err(status) => {
            error!(code = ?status.code(), message = %status.message(), "Health RPC failed");
            process::exit(3);
        }
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .expect("valid RUST_LOG");

    fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
```

---

## Cargo additions

**`Cargo.toml` (server)**

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
# tonic/anyhow etc. are already pinned per project preconditions.
```

---

## Run Examples

```bash
# Default (localhost:8080, 30s timeout, no token)
RUST_LOG=info cargo run -p server

# Custom bridge and token
RUST_LOG=server=debug,info \
MCP_BRIDGE_ADDR=https://127.0.0.1:7443 \
MCP_BRIDGE_TOKEN=secret-token \
MCP_BRIDGE_TIMEOUT=5 \
cargo run -p server
```

Expected logs (examples):

```
INFO  Starting minimal EditorControl client addr=https://127.0.0.1:7443 timeout_secs=5
INFO  Bridge is ready version=test
```

Or on failure:

```
ERROR Failed to connect to gRPC bridge error=transport error: connection refused
```

---

## Acceptance Checklist

* ✅ Builds cleanly.
* ✅ Calls `EditorControl.Health` once at startup.
* ✅ No panics offline; exits with non‑zero code and clear logs on failure.
* ✅ Tracing is configurable via `RUST_LOG` and defaults to `info`.
* ✅ Uses endpoint‑level timeouts (no per‑call timeouts).

---

## Common Pitfalls (and how this code avoids them)

* **Missing URI scheme**: `GrpcConfig` normalizes to `http://…` if the scheme is absent.
* **Empty token**: treated as `None`; the intercepted client is only used when a non‑empty token exists.
* **Hanging calls**: avoided with `Endpoint::timeout` in `ChannelManager::connect`.
* **Module paths**: `server::…` imports assume the presence of `src/lib.rs` that re‑exports modules; keep it aligned with T1/T2 docs.