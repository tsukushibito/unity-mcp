# Step 6 — Remove gRPC Completely & Finalize CI/Docs (Revised)

**Objective:** Make **direct IPC** the sole transport. Remove all gRPC code/paths, simplify CI, and update documentation. After this step:
- No `tonic*` crates or gRPC artifacts remain.
- Tests run against IPC stubs and/or Unity Editor (opt-in), not gRPC.
- CI is green on Windows/macOS/Linux.
- README/ARCH docs describe IPC-only operation.

> Prereqs: Steps 0–5 completed (Health/Logs/Operations/Assets/Build over IPC). This revision adds deeper checklists, search queries, and safety rails.

---

## 0) Removal Plan — Repo-wide Checklist

### Source tree
- [ ] Delete `server/src/grpc/**` (clients, channel managers, interceptors, server stubs).
- [ ] Delete any `*_grpc.rs`/`*_grpc_tests.rs` and their mod exports.
- [ ] Replace all `tonic::include_proto!` occurrences with the message-only generated modules.
- [ ] Remove gRPC feature flags (`transport-grpc`) and any `#[cfg(feature = "transport-grpc")]` blocks.
- [ ] Remove gRPC-specific configs (env vars like `MCP_BRIDGE_HOST/PORT`), or mark **deprecated**.

### Protobuf
- [ ] Ensure `build.rs` uses `prost-build` **messages only**; no service generation.
- [ ] Confirm `src/generated/*` (or `OUT_DIR`) inclusion paths are stable.

### Tests
- [ ] Convert or delete tests that spin up gRPC servers.
- [ ] Provide IPC stub servers for Health/Assets/Build in `tests/` (UDS on Unix; Named Pipe or TCP fallback elsewhere).
- [ ] Ensure parallel-safe endpoints (unique socket/pipe names per test).

---

## 1) Dependency Cleanup (commands)
Run in `server/` crate:

```bash
# Remove gRPC crates (idempotent)
cargo remove tonic tonic-build tonic-prost || true

# Sanity: verify nothing pulls tonic anymore
cargo tree -e features | rg -i 'tonic|grpc' || echo 'OK: no gRPC in dependency tree'
```

If a transitive dependency still brings `tonic`, replace it or gate it behind optional features that are **disabled**.

---

## 2) Feature Flags & Conditional Compilation

- [ ] In `Cargo.toml`, delete `transport-grpc`; make IPC the default (or un-gated):
  - `default = ["transport-ipc"]` (optional) → eventually collapse to **no feature** for transport.
- [ ] Remove all `#[cfg(feature = "transport-grpc")]` blocks; migrate any shared logic to non-transport modules.
- [ ] `cargo check --all-targets` should pass without defining any transport feature.

**Tip:** If other, non-transport features exist, test with `--no-default-features` to ensure transport isn’t accidentally gated.

---

## 3) Protobuf Codegen Policy (messages only)

- Keep `prost-build` message-only generation.
- Choose one strategy and document it:
  - **`src/generated`**: great IDE UX; commit to repo or `.gitignore` based on your policy.
  - **`OUT_DIR`**: clean source tree; IDE needs one build before navigation works.
- If committing generated code, add a `tools/regenerate-proto.sh` script and a CI job that fails if generated files are out-of-date.

**Example CI guard (pseudo):**
```bash
# run in repo root
./tools/regenerate-proto.sh
if ! git diff --quiet -- src/generated; then
  echo 'Generated proto is stale. Run tools/regenerate-proto.sh'; exit 1
fi
```

---

## 4) Testing Overhaul — IPC-first

### Unit tests
- Framing: length-delimited encode/decode roundtrip.
- Envelope: `IpcEnvelope` with `Request/Response/Event` roundtrip.
- Path parsing & defaults (per-OS behavior with `#[cfg]`).

### Integration tests (Rust-only)
- Stub server that accepts Hello/Welcome and handles:
  - Health with a canned `HealthResponse`.
  - Assets minimal ops (e.g., `GuidToPath` for an injected sandbox) with deterministic outputs.
  - Build minimal no-op (return a fake `BuildPlayerResponse`) to verify wiring.
- Event stream: emit `LogEvent` (WARN/ERROR) and `OperationEvent` burst; assert throttling and responsiveness.

### E2E (Rust↔Unity)
- Keep optional for CI (licenses). Provide a local script to launch tests against an Editor instance.

**Isolation:**
- Unique UDS paths per test (`/tmp/unity-mcp-<pid>-<rand>.sock`) and unique pipe names on Windows (`\\.\pipe\unity-mcp-<pid>-<rand>`).

---

## 5) CI Pipeline — Simplify & Stabilize

**Goals**: No gRPC setup; matrix across platforms; cached builds; optional protoc installation.

### GitHub Actions (example)
```yaml
name: CI
on: [push, pull_request]

jobs:
  build-test:
    strategy:
      fail-fast: false
      matrix: { os: [ubuntu-latest, macos-latest, windows-latest] }
    runs-on: ${{ matrix.os }}
    defaults: { run: { working-directory: ./server } }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install protoc (if generating at build)
        if: runner.os == 'Linux'
        run: sudo apt-get update && sudo apt-get install -y protobuf-compiler
      - name: Build
        run: cargo build --locked --verbose
      - name: Test
        run: cargo test --locked --verbose
      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: Format
        run: cargo fmt --all -- --check
```

**Variants:**
- If generated code is **committed**, drop the `protoc` step.
- Add an **optional** job (manually triggered) that boots a Unity Editor runner to run E2E.

---

## 6) Documentation — Update & Consolidate

### What to change
- **Architecture doc**: replace the gRPC bridge with the IPC diagram; explain Envelope and Handshake.
- **Setup**: quickstart showing default endpoints and environment variables:
  - `MCP_IPC_ENDPOINT` (e.g., `unix:///.../ipc.sock`, `pipe://unity-mcp/default`, `tcp://127.0.0.1:7777`)
  - `MCP_IPC_TOKEN` (optional shared secret)
  - Timeouts: `MCP_IPC_CONNECT_TIMEOUT_MS`, `MCP_IPC_CALL_TIMEOUT_MS`
- **Migration notes**: see §7 below.
- **Troubleshooting**: cannot connect, permission denied (UDS dir perms), pipe conflicts, schema mismatch.

### Artifacts
- Update README, `docs/architecture/ipc.md`, and any code comments that mention gRPC.
- Provide a **one-page cheat sheet** with commands to build/test and a sample `Health` call.

---

## 7) Migration Notes for Consumers

- **Transport**: only direct IPC is supported; gRPC endpoints are removed.
- **Config**: `MCP_BRIDGE_HOST/PORT` are **deprecated**; use `MCP_IPC_ENDPOINT`.
- **Error semantics**: envelope `status_code` aligns with common RPC codes (0 OK, 2 INVALID_ARGUMENT, 5 NOT_FOUND, 9 FAILED_PRECONDITION, 13 INTERNAL, etc.).
- **Versioning**: bump **minor** or **major** depending on external API exposure; document any message changes.
- **Security**: Handshake token recommended; avoid logging secrets.

---

## 8) Hygiene & Tooling

- [ ] Run `cargo deny` (optional) to confirm no gRPC deps.
- [ ] Add `#![deny(unused_imports)]` to non-generated modules; clean imports.
- [ ] Keep `#![allow(...)]` at the generated module boundary for known `prost` quirks.
- [ ] Add `make lint` / `make ci` phony targets for local parity with CI.

---

## 9) Verification — Definition of Done (Step 6)

- [ ] `cargo tree` shows **no** `tonic*` or `grpc*` crates.
- [ ] Source tree contains **no** `server/src/grpc/**` or gRPC code.
- [ ] All tests pass using **IPC-only** stubs (and optional Unity E2E).
- [ ] CI matrix (Win/macOS/Linux) is green.
- [ ] Docs updated; README quickstart demonstrates a `Health` IPC round-trip.

---

## 10) Rollback Plan

- Revert the PR that removes gRPC.
- If urgently required, temporarily restore `transport-grpc` feature and legacy client modules (short-lived branch only).

---

## 11) Release Checklist

- [ ] Tag the release; update CHANGELOG with IPC-only migration notes.
- [ ] Notify stakeholders (issue tracker, Slack, etc.).
- [ ] Archive/close issues and PRs related to gRPC bridge.
- [ ] Record the Unity Editor versions validated in E2E (for future triage).

