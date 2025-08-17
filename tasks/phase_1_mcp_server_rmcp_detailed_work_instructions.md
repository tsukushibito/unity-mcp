# Phase 1 — MCP Server (rmcp) Detailed Work Instructions
*(Module layout without `mod.rs`; use `module_name.rs` files)*

> **Scope**: Implement a minimal **MCP server** (stdio transport) exposing one tool `unity.health`, internally calling Unity Bridge via existing **gRPC clients** (`ChannelManager`). No SSE/HTTP, no auth, Linux CI only.

---

## 0) Deliverables / Definition of Done (DoD)

- `server` builds and runs an **rmcp MCP server** over **stdio**.
- MCP `list_tools` shows `unity.health`.
- `call_tool("unity.health")` performs a gRPC call to the (dummy) `EditorControl.Health` and returns structured JSON:
  ```json
  { "ready": true, "version": "X.Y.Z" }
  ```
- Unit/integration tests cover success, deadline exceeded, and unavailable cases.
- CI (`ci.yml`) runs build + tests (Linux). **No `mod.rs`** anywhere in new code.

---

## 1) File/Module Layout (no `mod.rs`)

```
server/
├─ Cargo.toml
├─ build.rs                   # already present: gRPC *client* codegen
└─ src/
   ├─ main.rs
   ├─ config.rs               # (recommended) config/env loader
   ├─ observability.rs        # (recommended) tracing init
   ├─ mcp.rs                  # root of MCP module tree (no mod.rs)
   ├─ mcp/
   │  ├─ service.rs           # McpService + serve_stdio()
   │  ├─ tools.rs             # 'tools' module (file), defines pub mod health;
   │  └─ tools/
   │     └─ health.rs         # #[tool] unity.health implementation
   ├─ grpc/                   # (existing) channel_manager.rs, clients.rs, etc.
   └─ mcp_types.rs            # DTOs for tool I/O (Json<T>)

tests/
└─ health_mcp.rs              # integration test using in-proc dummy gRPC server
```

> Rust module declarations (no `mod.rs`):
> - In `main.rs`: `mod mcp; mod config; mod observability;`
> - In `mcp.rs`: `pub mod service; pub mod tools;`
> - In `mcp/tools.rs`: `pub mod health;`

---

## 2) Dependency Changes

Run with `server/` as CWD:

```bash
cargo add rmcp --features server
cargo add serde --features derive
cargo add serde_json
cargo add schemars --dev            # only if you want JsonSchema for tooling
cargo add tracing tracing-subscriber
```

> Keep existing `tonic`, `prost`, `tokio` pins as-is.

---

## 3) Implementation Steps

### Step 3.1 — `observability.rs`
Minimal tracing init with env override.

```rust
// server/src/observability.rs
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).with_target(false).init();
}
```

### Step 3.2 — `config.rs`
Simple config (env-first, then defaults).

```rust
// server/src/config.rs
use std::env;

#[derive(Clone, Debug)]
pub struct BridgeConfig {
    pub host: String,   // e.g. "127.0.0.1"
    pub port: u16,      // e.g. 50051
    pub health_timeout_ms: u64, // e.g. 2000
}

impl BridgeConfig {
    pub fn load() -> Self {
        let host = env::var("UNITY_BRIDGE_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("UNITY_BRIDGE_PORT").ok()
            .and_then(|s| s.parse::<u16>().ok()).unwrap_or(50051);
        let health_timeout_ms = env::var("UNITY_HEALTH_TIMEOUT_MS").ok()
            .and_then(|s| s.parse::<u64>().ok()).unwrap_or(2000);
        Self { host, port, health_timeout_ms }
    }
}
```

### Step 3.3 — `mcp_types.rs`
DTOs for tool I/O.

```rust
// server/src/mcp_types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthOut {
    pub ready: bool,
    pub version: String,
}
```

### Step 3.4 — `mcp.rs`
Module root (no `mod.rs`).

```rust
// server/src/mcp.rs
pub mod service;
pub mod tools;
```

### Step 3.5 — `mcp/tools.rs`
Tools module file (exposes submodules in directory).

```rust
// server/src/mcp/tools.rs
pub mod health;
```

### Step 3.6 — `mcp/service.rs`
`McpService` and server boot. Holds `ChannelManager`.

```rust
// server/src/mcp/service.rs
use rmcp::{prelude::*, server::Server};
use crate::grpc::channel_manager::ChannelManager;

#[derive(Clone)]
pub struct McpService {
    cm: ChannelManager,
    tool_router: ToolRouter<Self>,
}

impl McpService {
    pub fn new(cm: ChannelManager) -> Self {
        Self { cm, tool_router: Self::tool_router() }
    }

    pub async fn serve_stdio(self) -> anyhow::Result<()> {
        let transport = rmcp::transport::stdio();
        Server::new(self).serve(transport).await?;
        Ok(())
    }
}

#[rmcp::tool_router]
impl McpService {}

// Tool methods are implemented in submodules and use inherent impl extension pattern:
impl McpService {
    pub(crate) fn cm(&self) -> &ChannelManager { &self.cm }
}
```

### Step 3.7 — `mcp/tools/health.rs`
Implement the tool `unity.health`.

```rust
// server/src/mcp/tools/health.rs
use rmcp::{prelude::*, types::Json};
use crate::mcp_types::HealthOut;
use crate::mcp::service::McpService;
use tonic::Code;

impl McpService {
    #[rmcp::tool(name = "unity.health", description = "Unity Bridge health check")]
    pub async fn tool_unity_health(&self) -> Result<Json<HealthOut>, ToolError> {
        // Get typed gRPC client from ChannelManager
        let mut ec = self.cm().editor_control_client().await
            .map_err(to_tool_err)?;

        // Deadline from config
        let deadline = std::time::Duration::from_millis(self.cm().config().health_timeout_ms);

        let resp = ec.health_with_deadline(deadline).await
            .map_err(to_tool_err)?;

        Ok(Json(HealthOut {
            ready: resp.ready,
            version: resp.version,
        }))
    }
}

// gRPC Status -> MCP ToolError mapping (minimal)
fn to_tool_err(e: impl std::error::Error + Send + Sync + 'static) -> ToolError {
    // Prefer tonic::Status specifics when available
    if let Some(status) = e.downcast_ref::<tonic::Status>() {
        match status.code() {
            Code::Unavailable => ToolError::from_message("Unity Bridge unavailable"),
            Code::DeadlineExceeded => ToolError::from_message("Unity Bridge deadline exceeded"),
            Code::Unauthenticated => ToolError::from_message("Unauthenticated to Unity Bridge"),
            _ => ToolError::from_message(status.message().to_string()),
        }
    } else {
        ToolError::from_message(e.to_string())
    }
}
```

> `editor_control_client()` and `health_with_deadline()` are expected from your existing `ChannelManager` wrappers; adjust names to your actual API.

### Step 3.8 — `main.rs`
Wire everything and run stdio server.

```rust
// server/src/main.rs
mod config;
mod observability;
mod mcp;
mod mcp_types;

use crate::config::BridgeConfig;
use crate::mcp::service::McpService;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    observability::init_tracing();
    let cfg = BridgeConfig::load();

    // ChannelManager is assumed to exist in crate::grpc
    let cm = crate::grpc::channel_manager::ChannelManager::new(cfg).await?;

    let svc = McpService::new(cm);
    svc.serve_stdio().await
}
```

---

## 4) Integration Points (expected existing API)

Ensure your `ChannelManager` provides:

- `fn config(&self) -> &BridgeConfig`
- `async fn editor_control_client(&self) -> Result<EditorControlClient, anyhow::Error>`
- `impl EditorControlClient { async fn health_with_deadline(&mut self, d: Duration) -> Result<HealthResponse, tonic::Status> }`

If names differ, update the tool code accordingly.

---

## 5) Tests

### `tests/health_mcp.rs` (outline)

- Spin up an **in-process tonic** test server that implements `EditorControl.Health`:
  - Case 1: returns `{ ready: true, version: "dummy" }`
  - Case 2: delays > deadline to trigger `DeadlineExceeded`
  - Case 3: server not running → `Unavailable`
- Launch the rmcp server in-process (or call `McpService` methods directly via router).
- Assert:
  - `list_tools` contains `unity.health`
  - `call_tool` success → `Json(HealthOut{ ready: true, version: "dummy" })`
  - Error cases map to readable ToolError messages.

Pseudo:

```rust
#[tokio::test]
async fn health_success() -> anyhow::Result<()> {
    let (bridge_addr, _server_guard) = start_dummy_editor_control_server().await?;
    std::env::set_var("UNITY_BRIDGE_HOST", bridge_addr.ip().to_string());
    std::env::set_var("UNITY_BRIDGE_PORT", bridge_addr.port().to_string());

    let cfg = BridgeConfig::load();
    let cm = ChannelManager::new(cfg).await?;
    let svc = McpService::new(cm);

    // Call tool directly
    let out = svc.tool_unity_health().await?;
    assert!(out.0.ready);
    Ok(())
}
```

---

## 6) CI

- Ensure `ci.yml` runs:
  - `cargo build --locked --verbose -p server`
  - `cargo test  --locked --verbose -p server`
- Protoc remains `3.21.12` (already handled by your pipeline).
- No network flakiness: bind test gRPC server to `127.0.0.1:0` and pass the resolved port via env.

---

## 7) Documentation

- Update `server/README.md` with:
  - **Run**:
    ```bash
    RUST_LOG=info UNITY_BRIDGE_HOST=127.0.0.1 UNITY_BRIDGE_PORT=50051 cargo run -p server
    ```
  - **Tools**:
    - `unity.health`: Returns `{ ready, version }`
  - **Config**: `UNITY_BRIDGE_HOST`, `UNITY_BRIDGE_PORT`, `UNITY_HEALTH_TIMEOUT_MS`
  - **Known Issues**: Bridge down → “Unity Bridge unavailable”.

---

## 8) PR Breakdown

1. **PR-1: Scaffolding**
   - Add `observability.rs`, `config.rs`, `mcp.rs`, `mcp/service.rs`, `mcp/tools.rs`, `mcp/tools/health.rs`, `mcp_types.rs`.
   - Cargo deps (`rmcp`, `serde*`, `tracing*`), `main.rs` wiring.
   - DoD: builds locally, `list_tools` shows `unity.health` (can be unit-tested by calling tool method directly).

2. **PR-2: Health Tool → gRPC Integration**
   - Hook `ChannelManager` and real gRPC client.
   - Implement deadline handling and error mapping.
   - DoD: manual run against a dummy server returns JSON.

3. **PR-3: Tests & CI**
   - Add `tests/health_mcp.rs` with success/timeout/unavailable.
   - Ensure CI passes on Linux.

---

## 9) Error Mapping (minimal policy)

| gRPC `tonic::Code`    | MCP ToolError message             |
|-----------------------|-----------------------------------|
| `Unavailable`         | `Unity Bridge unavailable`        |
| `DeadlineExceeded`    | `Unity Bridge deadline exceeded`  |
| `Unauthenticated`     | `Unauthenticated to Unity Bridge` |
| (others)              | `status.message()` passthrough    |

---

## 10) Rollback / Safety

- The new code is additive and behind the `server` binary path; revert by removing `mcp*` files and `rmcp` dep.
- No changes to proto or gRPC codegen pipeline in Phase 1.

---

### Notes & Conventions

- **No `mod.rs`**: Root `mcp.rs` + directory submodules (`mcp/tools.rs` + `mcp/tools/*.rs`) satisfy the requirement.
- Keep tool method names unique (`#[tool(name="unity.health")]`) and return `Json<T>` for structured outputs.
- Timeouts are read from config; default `2000ms`.

