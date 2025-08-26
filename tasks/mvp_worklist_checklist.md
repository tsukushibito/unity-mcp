# MVP Closeout Checklist — Direct IPC Unity MCP Server

Purpose: Track remaining high-level work to reach the MVP DoD. Keep progress here; link detailed implementation notes to PRs/issues as needed.

## Handshake & Schema
- [ ] Implement schema hash validation on Unity bridge
  - [ ] Generate/ship `SCHEMA_HASH` for C# (CI-generated constant)
  - [ ] Compare `hello.schema_hash` vs server hash; mismatch → `FAILED_PRECONDITION` Reject
  - [ ] Rust integration test: schema mismatch → `IpcError::SchemaMismatch`

## Security
- [ ] Enforce non-empty token (no dev mode)
  - [ ] Unity: Reject empty/missing token even when no expected token is configured
  - [ ] Quick guidance in error message on how to set token
  - [ ] EditMode test: empty/missing token → `UNAUTHENTICATED`

## CI & Proto Parity
- [ ] Add proto regeneration + diff check to CI (Rust side)
  - [ ] Fail CI on drift with actionable message
- [ ] Generate C# `SchemaHash` from Rust `SCHEMA_HASH_HEX` in CI to keep a single source of truth
- [ ] Document the CI steps in a brief developer note

## Developer Experience
- [ ] Quickstart doc for Direct IPC
  - [ ] Unity open → bridge auto-start (TCP 127.0.0.1:7777)
  - [ ] Set `MCP_IPC_TOKEN`, `MCP_PROJECT_ROOT`
  - [ ] `cargo run --example test_unity_ipc` (health + log tail)
  - [ ] Troubleshooting (port busy, token mismatch, missing protobuf DLL, schema mismatch)

## Examples
- [ ] Extend `examples/test_unity_ipc.rs`
  - [ ] Subscribe to event stream and print Unity logs for ~10s
  - [ ] Clear success/failure output and exit codes

## Stability (Optional for MVP if time-constrained)
- [ ] Finalize reconnection writer-channel swap in `spawn_supervisor`
  - [ ] Basic manual verification: Unity restart → auto reconnect

## Tests (Add/Update)
- [ ] Rust integration tests
  - [ ] Schema mismatch → Reject
  - [ ] Project root mismatch → `FailedPrecondition`
  - [ ] Feature negotiation drops unknown features
- [ ] Unity EditMode tests
  - [ ] Token required (empty/missing → Reject)
  - [ ] Health/Assets/Build basic happy paths remain green

## Documentation
- [ ] Error vocabulary table (`docs/errors.md`) and mapping notes (Unity → MCP)

## Milestone Verification
- [ ] Fresh clone E2E
  - [ ] Unity Editor opens, bridge starts (7777)
  - [ ] Handshake OK with features + schema hash
  - [ ] `unity.health` returns `{ ready, version }`
  - [ ] Unity logs visible in server output
  - [ ] Assets basic ops (p2g/g2p/import/refresh) succeed
  - [ ] Minimal build starts and completes with operation events
  - [ ] CI (Linux) green: build/test/clippy/fmt + proto parity check

Notes:
- Keep this list high-level; attach PRs/issues per item for details.
- Items marked Optional can be deferred if core DoD is satisfied.

