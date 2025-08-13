# T4 — Smoke Test (no Unity Bridge): Fixes & Final Skeleton

This document fixes pitfalls in the original plan and provides a clean, copy‑pasteable skeleton for the smoke test, build‑time gating for server stubs, and CI wiring. It targets the **L0 gRPC** protocol and validates **ChannelManager** connectivity with an in‑process gRPC server.

---

## What Changed & Why

1. **Use a bound listener to capture the OS‑assigned port**
   `serve_with_shutdown(addr)` cannot tell you the actual port when you pass `127.0.0.1:0`. Bind a `TcpListener`, read `local_addr`, and serve **with incoming**.

2. **Match L0 Health schema**
   L0 `HealthResponse` is `{ version: string, ready: bool }` (not `status: "OK"`). The test asserts `ready == true` and `version != ""`.

3. **Spawn the server & implement deterministic shutdown**
   Use `tokio::spawn` and `oneshot` to avoid hanging tests and to shut down cleanly.

4. **Gate server stubs at build time**
   Add a Cargo **feature** (`server-stubs`) and keep an **env var** fallback (`TONIC_BUILD_SERVER=1`). This makes CI & local runs predictable.

5. **Expose generated modules to integration tests**
   Integration tests (`server/tests/*.rs`) compile as a separate crate; ensure `src/lib.rs` re‑exports `pub mod generated;` (and the `grpc` modules that expose `ChannelManager`).

6. **CI knobs**
   Two equivalent ways to enable server stubs in CI: feature flag or env var. Examples provided below.

---

## `build.rs` (final)

```rust
use std::{env, fs, path::PathBuf};

fn want_server_stubs() -> bool {
    // Prefer a Cargo feature, allow env var as a fallback
    env::var("CARGO_FEATURE_SERVER_STUBS").is_ok()
        || env::var("TONIC_BUILD_SERVER").map(|v| v == "1").unwrap_or(false)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let proto_root = manifest_dir.join("..").join("proto");
    let out_dir = manifest_dir.join("src").join("generated");
    fs::create_dir_all(&out_dir)?;

    let protos = &[
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
    ];

    tonic_build::configure()
        .build_server(want_server_stubs())
        .out_dir(&out_dir)
        .compile(
            &protos.iter().map(|p| proto_root.join(p)).collect::<Vec<_>>(),
            &[proto_root.clone()],
        )?;

    println!("cargo:rerun-if-changed={}", proto_root.display());
    Ok(())
}
```

> **Notes**
>
> * Use `tonic_build::configure()` (standard) rather than non‑standard builders.
> * Output into `server/src/generated` so tests can import via the server crate.

---

## `Cargo.toml` additions (server crate)

```toml
[features]
# Enable gRPC server stubs (needed only for tests here)
server-stubs = []
```

---

## `src/lib.rs` (re‑exports for integration tests)

```rust
// Re-export the prost/tonic generated modules for tests and other crates.
pub mod generated;

// Re-export ChannelManager and config so tests can use them as `server::grpc::...`
pub mod grpc {
    pub mod config;   // defines GrpcConfig
    pub mod channel;  // defines ChannelManager
    pub mod clients;  // typed client constructors
}
```

> Ensure these modules exist and are `pub`. The test below assumes `GrpcConfig` and `ChannelManager::connect` + `editor_control_client()` are available at `server::grpc::...`.

---

## Integration test: `server/tests/smoke.rs`

```rust
use std::{net::SocketAddr, time::Duration};
use tokio::{net::TcpListener, sync::oneshot, time::timeout};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

// Import generated types & service stubs re-exported from the server crate
use server::generated::mcp::unity::v1::editor_control::{
    editor_control_server::{EditorControl, EditorControlServer},
    HealthRequest, HealthResponse,
};

#[derive(Debug, Default)]
struct TestEditorControlService;

#[tonic::async_trait]
impl EditorControl for TestEditorControlService {
    async fn health(
        &self,
        _req: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            version: "test".to_string(),
            ready: true,
        }))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn channel_manager_roundtrip_health() -> anyhow::Result<()> {
    // 1) Bind a local port and capture it before serving
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr: SocketAddr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);

    // 2) Spawn the in-process gRPC server with clean shutdown
    let (tx, rx) = oneshot::channel::<()>();
    let svc = TestEditorControlService::default();
    let server = tokio::spawn(async move {
        Server::builder()
            .add_service(EditorControlServer::new(svc))
            .serve_with_incoming_shutdown(incoming, async {
                let _ = rx.await; // wait for shutdown signal
            })
            .await
    });

    // 3) Connect ChannelManager to the test server
    let cfg = server::grpc::config::GrpcConfig {
        addr: format!("http://{}", addr),
        token: None,
        default_timeout_secs: 5,
    };
    let cm = server::grpc::channel::ChannelManager::connect(&cfg).await?;
    let mut client = cm.editor_control_client();

    // 4) Round-trip: call Health and assert L0 schema
    let resp = timeout(Duration::from_secs(5), client.health(HealthRequest {})).await??;
    let HealthResponse { version, ready } = resp.into_inner();
    assert!(ready, "bridge should report ready");
    assert!(!version.is_empty(), "version should be non-empty");

    // 5) Cleanup
    let _ = tx.send(());
    server.await??;
    Ok(())
}
```

> **Why this shape?**
>
> * `TcpListener` → you reliably obtain the port.
> * `serve_with_incoming_shutdown` → deterministic shutdown (no dangling tasks/ports).
> * `timeout(...)` wraps the client call so failures fail fast and don’t hang CI.

---

## How to run

### Locally

**Option A — feature flag**

```bash
cargo test -p server --features server-stubs
```

**Option B — env var**

```bash
TONIC_BUILD_SERVER=1 cargo test -p server
```

### CI (GitHub Actions snippet)

**Feature flag style:**

```yaml
- name: Test (server)
  run: cargo test -p server --features server-stubs -- --nocapture
```

**Env var style:**

```yaml
- name: Test (server)
  env:
    TONIC_BUILD_SERVER: "1"
  run: cargo test -p server -- --nocapture
```

---

## Acceptance Checklist

* ✅ Smoke test passes with no Unity Bridge.
* ✅ Server stubs are built **only** for tests (feature/env‑gated).
* ✅ ChannelManager round‑trip to `EditorControl.Health` verified.
* ✅ Clean startup/shutdown: no hanging tasks, no port leaks.

---

## Nice‑to‑Have (later)

* Add retries/backoff to `ChannelManager` and assert they don’t trigger in this test.
* Add a second test covering unary error mapping and deadline behavior.
* Add a minimal Events stream test (shared logs subscription fan‑out), still without Unity Bridge.