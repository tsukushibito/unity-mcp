# Phase 1 — MCP Server Foundation: Detailed Task List

**Goal:** Stand up an MCP (stdio) server in the Rust `server/` crate, expose two minimal tools — `unity.editor.health` and `unity.events.subscribe_logs` — and wire them to the existing gRPC client stack (`ChannelManager`), with CI green.

**Inputs considered:** current `server/` sources (zip), `proto/mcp/unity/v1/*.proto` (L0), and the uploaded `ci.yml` (skeleton present; finalize in this phase).

---

## 0) Scope & DoD (Definition of Done)

* [ ] **MCP stdio server** boots via `cargo run` and exposes tool list.
* [ ] **`unity.editor.health`** calls gRPC `EditorControl.Health` and returns structured JSON.
* [ ] **`unity.events.subscribe_logs`** streams heartbeat events from a relay (dummy for Phase 1).
* [ ] **Per-call timeouts** are honored (Health=5s; Logs subscribe start=30s) via `ChannelManager` override.
* [ ] **Unit tests** cover Health happy/timeout paths and Logs heartbeat reception.
* [ ] **CI** (current `ci.yml`) builds, formats, lints, and runs tests on Ubuntu with `protoc 31.1`.

Out of scope (Phase 2+): SSE transport, Unity Bridge implementation, true Events stream from the Bridge.

---

## 1) Decisions (lock before coding)

* Transport: **stdio** only for Phase 1 (SSE later).
* Auth: continue using `Authorization: Bearer <token>` injected by `ChannelManager`.
* Timeouts: Health = **5s**, Logs subscription start = **30s** (stream thereafter).
* Tool names (stable for E2E):

  * `unity.editor.health`
  * `unity.events.subscribe_logs`
* **Config keys (env):**

  * `MCP_BRIDGE_ADDR` (e.g., `http://127.0.0.1:8080`)
  * `MCP_BRIDGE_TOKEN` (optional)
  * `MCP_BRIDGE_TIMEOUT` (seconds; default 30)
* **Error mapping (gRPC → MCP):**

  * `Unavailable` → `service_unavailable` *(fallback: `internal_error` if ambiguous)*
  * `DeadlineExceeded` → `timeout_error`
  * *otherwise* → `internal_error`
* **Per-call timeouts:** `ChannelManager` may override default endpoint timeouts per request (e.g., Health=5s) without changing global config.
* **Rust edition:** keep current project setting (as-is).
* **MCP runtime:** use `rmcp` (installable via `cargo add rmcp`, e.g., `0.5.x`).

---

## 2) Dependencies & Layout Changes

### 2.1 Cargo dependencies (edit `server/Cargo.toml`)

Add with `cargo add` (prefer latest minor versions):

```bash
cd server
cargo add rmcp@^0.5 # MCP runtime
cargo add serde@^1 --features derive
cargo add serde_json@^1
cargo add tokio-stream@^0.1
# Optional when composing custom streams/utilities
cargo add futures@^0.3
```

Notes:

* Keep existing `tokio`, `tracing`, `tonic`, `prost` pins as-is.
* No change to Rust edition.

### 2.2 Source tree additions (all under `server/src`)

Create the following modules:

```
server/src/
  mcp/
    mod.rs           # MCP stdio bootstrap: init server, register tools, serve
    context.rs       # AppContext { cm: ChannelManager, cfg: GrpcConfig }
    error.rs         # (optional) tonic::Status → MCP error mapping
  tools/
    editor.rs        # Tool: unity.editor.health
    events.rs        # Tool: unity.events.subscribe_logs (stream)
  relay/
    logs.rs          # Heartbeat producer + broadcast relay (dummy for Phase 1)
```

> Note: this adopts the agreed layout (tools at `server/src/tools/*`).

### 2.3 Main entry wiring (`server/src/main.rs`)

* Default behavior: **start MCP stdio server**.
* Retain the existing health-check CLI behind a flag, e.g. `--health-check` (invokes the direct gRPC client path).
* Startup sequence:

  1. init tracing
  2. load `GrpcConfig` from env (`MCP_BRIDGE_*`)
  3. build `ChannelManager`
  4. spawn `LogsRelay` (dummy heartbeat)
  5. `mcp::run_stdio_server(AppContext{ cm, cfg })`

Per-call timeout examples (pseudo):

```rust
let mut ec = ctx.cm.editor_control_client().with_timeout(Duration::from_secs(5));
let rsp = ec.health(HealthRequest{}).await?;
```

---

## 3) MCP Tools — Implementation Details

### 3.1 `unity.editor.health`

* Input: none.
* Logic:

  1. Acquire `EditorControlClient` from `ChannelManager`.
  2. Call `Health(HealthRequest {})` with a 5s deadline.
  3. Return MCP `tool_result` JSON like:

     ```json
     { "ready": true, "version": "<bridge_version>", "status": "OK", "bridge_addr": "<cfg.addr>" }
     ```
* Errors: translate gRPC `Status` to MCP error (Unavailable / DeadlineExceeded / PermissionDenied).

### 3.2 `unity.events.subscribe_logs`

* Input: none for Phase 1 (no filters; simple subscription).
* Logic:

  1. Subscribe to `LogsRelay` broadcast channel.
  2. Yield events as a stream of JSON objects, e.g.:

     ```json
     { "kind": "heartbeat", "message": "bridge: idle", "ts": 1699999999 }
     ```
* Future-proofing: the relay type should be replaceable with a consumer of `Events.SubscribeOperation` without changing the MCP tool’s external shape.

---

## 4) Relay — Minimal Heartbeat Producer (`relay/logs.rs`)

* Use `tokio::sync::broadcast` for fan-out; bounded channel with drop-oldest on overflow.
* Spawn task with `tokio::time::interval(Duration::from_secs(2))` that publishes heartbeat events.
* **Event JSON (recommended):**

  * Required: `kind`, `message`, `ts`
  * Optional: `level`, `source`, `op_id`, `seq`
* Example produced item:

```json
{ "kind": "heartbeat", "message": "bridge: idle", "ts": 1699999999, "source": "server", "seq": 1 }
```

* Provide `LogsRelay::subscribe()` → `broadcast::Receiver<LogEvent>` for the MCP tool.

---

## 5) Tests

### 5.1 Keep existing gRPC smoke (`tests/smoke.rs`)

* Ensure it still runs under `--features server-stubs` and validates basic round-trip.

### 5.2 Add unit tests (Phase 1 focus)

* `tests/mcp_tools_health.rs`: call the Health tool handler directly (no subprocess), assert happy and timeout error mapping (`timeout_error`).
* `tests/logs_relay.rs`: subscribe to `LogsRelay`, assert N heartbeat events within T seconds; verify channel drop policy does not panic under load.

> Spawning a real stdio subprocess is not required in Phase 1 (can be added later as an integration test).

---

## 6) CI Finalization (`.github/workflows/ci.yml`)

**Intent:** cache Cargo, build & test (with `server-stubs`), format & clippy, using the **current protoc 31.1** setting.

> Keep your existing `ci.yml`’s protoc pin at **31.1** (Ubuntu). Add caching and ensure the working-directory steps target `server/`.

**Suggested shape (adapt to your file):**

```yaml
name: CI
on: [push, pull_request]

jobs:
  build-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup protoc (keep 31.1)
        uses: arduino/setup-protoc@v3
        with:
          version: 31.1

      - uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: "server -> target"

      - name: Build
        working-directory: server
        run: cargo build --locked --verbose

      - name: Test (with server stubs)
        working-directory: server
        run: cargo test --locked --verbose --features server-stubs

      - name: Check formatting
        working-directory: server
        run: cargo fmt --all -- --check

      - name: Run clippy
        working-directory: server
        run: cargo clippy --all-targets -- -D warnings
```

---

## 7) README Update (`server/README.md`)

Add a minimal Quickstart and config reference:

* **Quickstart**

  * `cargo run` → start MCP stdio server
  * `cargo run -- --health-check` → run the legacy gRPC health-check CLI
* **Config (env)**

  * `MCP_BRIDGE_ADDR` (default `http://127.0.0.1:8080`)
  * `MCP_BRIDGE_TOKEN` (optional)
  * `MCP_BRIDGE_TIMEOUT` (seconds; default `30`)
* **Tools**

  * `unity.editor.health` → returns `{ ready, version, status, bridge_addr, observed_at, latency_ms }`
  * `unity.events.subscribe_logs` → streams `{ kind, message, ts [, level, source, op_id, seq] }`
* **Tests**

  * `cargo test --features server-stubs`
* **Error mapping**

  * `Unavailable` → `service_unavailable`
  * `DeadlineExceeded` → `timeout_error`
  * otherwise → `internal_error`

---

## 8) File-by-File Checklist

* [ ] `server/Cargo.toml`: add `rmcp`, `serde`, `serde_json`, `tokio-stream`, (optional) `futures`.
* [ ] `server/src/mcp/mod.rs`: stdio server bootstrap & tool registration.
* [ ] `server/src/mcp/context.rs`: shared `AppContext`.
* [ ] `server/src/mcp/error.rs`: gRPC → MCP error mapping.
* [ ] `server/src/tools/editor.rs`: implement `unity.editor.health`.
* [ ] `server/src/tools/events.rs`: implement `unity.events.subscribe_logs`.
* [ ] `server/src/relay/logs.rs`: heartbeat relay with broadcast channel.
* [ ] `server/src/main.rs`: default MCP, keep `--health-check` client mode.
* [ ] `server/tests/mcp_tools_health.rs`: unit tests for tool, error mapping.
* [ ] `server/tests/logs_relay.rs`: unit tests for relay heartbeat and backpressure.
* [ ] `.github/workflows/ci.yml`: keep protoc 31.1; add cargo cache; ensure `server/` working dir.
* [ ] `server/README.md`: quickstart, config, tools, tests, error mapping.

---

## 9) Rollout Plan

1. **PR 1** — Add MCP scaffolding (`mcp/*`, relay, main wiring), compile only.
2. **PR 2** — Implement `unity.editor.health` tool; unit test happy/timeout paths.
3. **PR 3** — Implement `unity.events.subscribe_logs` with heartbeat; streaming test.
4. **PR 4** — CI finalization (`ci.yml`); README.

Each PR should keep tests green and avoid impacting the existing gRPC smoke tests.

---

## 10) Risks & Mitigations

* **Deadlines and retries:** use per-call timeouts to avoid long hangs; avoid stacking multiple timeout layers.
* **Stream backpressure:** use bounded `broadcast` and drop-oldest policy; heartbeat at ≥2s.
* **CI stability:** keep protoc at 31.1 per current environment; cache Cargo to reduce flakiness.

## 11) Nice-to-haves (not required for DoD)

* Add `--transport sse|stdio` CLI arg (stdio default) for future parity.

* Structured tracing keys (`op_id`, `req_id`) on tool entry/exit.

* A `versions` tool returning crate and protocol versions. (not required for DoD)

* Add `--transport sse|stdio` CLI arg (stdio default) for future parity.

* Structured tracing keys (`op_id`, `req_id`) on tool entry/exit.

* A `versions` tool returning crate and protocol versions.