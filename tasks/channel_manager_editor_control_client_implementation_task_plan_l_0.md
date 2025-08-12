# Objective

Implement a reusable **ChannelManager** and one minimal gRPC client (**EditorControl**) on the Rust side, aligned with L0 policy (no googleapis). Result must build locally (macOS/Ubuntu) and in CI, without manual steps.

Status: **PROVISIONAL** (breaking changes allowed pre–Schema Freeze)

---

## Preconditions

- Repo layout matches the project’s authoritative tree
  - `proto/mcp/unity/v1/*.proto` present
  - `server/build.rs` already fixed to use `CARGO_MANIFEST_DIR` and `build_server(false)`
- `server/Cargo.toml` pins:
  ```toml
  anyhow = "1.0.99"
  prost = "0.14.1"
  tokio = { version = "1.47.1", features = ["macros", "rt-multi-thread"] }
  tonic = { version = "0.14.1", features = ["transport", "tls-webpki-roots"] }
  tonic-prost = "0.14.1"
  ```
  **[build-dependencies]**: `tonic-prost-build = "0.14.1"`
- `protoc >= 3.21` installed (local and CI)

Acceptance: `cargo build -p server` succeeds before you start (proves codegen path OK).

---

## Deliverables (Definition of Done)

1. `server/src/config.rs` – typed env config for gRPC (addr/token/timeout)
2. `server/src/grpc/channel.rs` – ChannelManager (connect via `Endpoint::timeout`, auth metadata injection, typed client getters)
3. `server/src/main.rs` – demo wiring: call `EditorControl.Health` once and log result/error
4. A minimal smoke test (`tests/smoke.rs`) that spins an in-process tonic server for `Health` and verifies client round-trip
5. CI updated to use fixed `protoc` version and to run the smoke test, clippy, and fmt checks

---

## Task List (checkpointed, in order)

### T1 — Add configuration module

- Define `GrpcConfig { addr: String, token: Option<String>, default_timeout_secs: u64 }`
- Implement `GrpcConfig::from_env()` reading env vars for bridge address, token, timeout
- Unit test covers defaults and parsing

### T2 — Implement ChannelManager

- Provide `connect`, `with_meta`, and `editor_control_client` methods
- Configure `Endpoint` with `timeout(Duration::from_secs(cfg.default_timeout_secs))`, optional token injection
- Remove any legacy `Request::set_timeout` usage; rely on endpoint-level timeouts
- Include generated proto once

### T3 — Wire up a minimal client (EditorControl)

- Initialize tracing
- Read config, connect ChannelManager
- Call `Health` RPC, log result or error without panicking if offline

### T4 — Add a smoke test (no Unity Bridge required)

- Start minimal tonic server for `EditorControl`
- Connect with ChannelManager and assert round-trip
- **Note**: since `build.rs` uses `build_server(false)`, add an env flag (e.g., `TONIC_BUILD_SERVER=1`) to enable server stub generation during test builds

### T5 — CI hardening

- Pin `protoc` setup
- Add Cargo cache, clippy, fmt checks, run tests

---

## Reference Skeletons

Skeletons for `config.rs`, `channel.rs`, `main.rs`, and `smoke.rs` remain as in the original plan, adjusted for the crate versions above.

---

## Nice-to-haves (later)

- `tower::ServiceBuilder` for retry/backoff and concurrency limits
- TLS support toggle
- Per-CallKind timeouts

---

## Rollback Plan

Remove added files and restore `main.rs` to its previous minimal state.

