# Sprint 1 — Detailed Task List (Unity MCP Server, revised with `proto` crate)

The goal of Sprint 1 is to establish the **core transport and session layer** across all components—Server, CLI client, and Unity Bridge—using the updated directory structure that now includes a dedicated **`mcp/proto`** crate.

---

## 0. Preliminary Setup

- [x] **Git workspace hygiene** – ensure `.gitignore` excludes `target/`, `bridge/Library`, and `bridge/Logs`.
- [x] **Rust workspace declaration** – list

   ```toml
   [workspace]
   members = [
       "mcp/proto",
       "mcp/server",
       "mcp/client"
   ]
   ```
- [ ] **CI boilerplate** – validate *lint*, *rust*, *dotnet*, and *unity* workflows compile on a clean checkout.
- [ ] **Pre‑commit hooks** – add `rustfmt`, `clippy`, and `cargo deny` to `.pre‑commit‑config.yaml`.

---

## 1. Protocol Specification

- [x] Decide **frame format** – 4‑byte little‑endian length prefix.
- [x] Finalize **JSON‑RPC 2.0 field rules** (`id`, `method`, `params`, `result`, `error`).
- [x] Draft **error‑code matrix** (Markdown table).
- [x] Commit `docs/protocol.md` with examples for *Auth* and *HealthCheck*.

---

## 2. Shared Library – `mcp/proto`

- [x] Create crate skeleton (`Cargo.toml`, `src/lib.rs`).
- [x] Define `RpcRequest`, `RpcResponse`, `RpcError` structs with `serde` derives.
- [x] Implement **framer** API: `encode_frame` / `decode_frame`.
- [x] Add unit tests: normal, truncated, and malformed frames.
- [ ] Generate docs (`cargo doc --no-deps` must be warning‑free).

---

## 3. MCP Server – `mcp/server`

 - [ ] Wire **async entry‑point** with `tokio` runtime.
 - [ ] Integrate `proto` crate for framing/parsing.
 - [ ] Implement **stdio reader task** and **writer queue**.
 - [ ] Add handlers:

    * `session.authenticate`
    * `transport.healthCheck`
 - [ ] Introduce **auth middleware** – reject all but allowlist before authentication.
 - [ ] Centralise error/timeout handling via `thiserror` + `anyhow`.
 - [ ] Configure structured logging with `tracing`.
 - [ ] Write unit tests for each handler.
 - [ ] Write integration test launching server as a subprocess and executing Auth→Ping via stdio.

---

## 4. CLI Client – `mcp/client`

 - [ ] Add subprocess utility (`std::process::Command`) to spawn the server binary.
 - [ ] Re‑export `proto` types to construct RPC payloads.
 - [ ] Implement helper `async fn call(method, params)` returning `Result<Value, RpcError>`.
 - [ ] **Success‑path integration test** – Auth then Ping.
 - [ ] **Failure cases** – unauthenticated Ping, malformed JSON, timeout.
 - [ ] Register tests in GitHub Actions matrix (Win/macOS/Linux).

---

## 5. Unity Bridge – `bridge/Packages/com.example.mcp‑bridge`

 - [ ] Generate UPM package skeleton (`package.json`, asmdef).
 - [ ] Implement `ProcessTransport` (C#) that starts the Rust server and streams stdio asynchronously.
 - [ ] Port frame encode/decode to C# (`System.Buffers.Binary`).
 - [ ] Add Editor Tests (PlayMode) performing Auth→Ping round‑trip.
 - [ ] Post‑build script copies compiled server binary to `Runtime~`.
 - [ ] Document package installation steps in `bridge/README.md`.

---

## 6. CI / DevOps

 - [ ] Extend **rust‑ci.yml** to build & test `proto`, `server`, and `client` crates.
 - [ ] Configure artifact upload of server binary for downstream Unity job.
 - [ ] Update **unity‑ci.yml** – depend on rust job via `needs:` and run Editor Tests headless.
 - [ ] Aggregate coverage with `grcov` and publish to Coveralls.

---

## 7. Documentation

 - [ ] Update `README.md` with quick‑start commands for each crate.
 - [ ] Add `CONTRIBUTING.md` covering lint, tests, and auth token setup.
 - [ ] Start `CHANGELOG.md` (`v0.1.0‑alpha – Sprint 1`).

---

## 8. Definition of Done ✅

* [ ] `cargo test --workspace` passes locally.
* [ ] GitHub Actions matrix is 100 % green.
* [ ] Unity Editor Tests pass in headless CI.
* [ ] `docs/protocol.md` merged and referenced from README.
* [ ] CHANGELOG entry *Sprint 1 completed* committed.

When these check‑boxes are all satisfied, the communication layer—**authentication & health‑check across Server, CLI, and Unity Bridge—will be CI‑guaranteed** for future Sprints.
