# Sprint 1 — Detailed Task List (Unity MCP Server, revised with `proto` crate)

The goal of Sprint 1 is to establish the **core transport and session layer** across all components—Server, CLI client, and Unity Bridge—using the updated directory structure that now includes a dedicated **`mcp/proto`** crate.

---

## 0. Preliminary Setup

1. **Git workspace hygiene** – ensure `.gitignore` excludes `target/`, `bridge/Library`, and `bridge/Logs`.
2. **Rust workspace declaration** – list

   ```toml
   [workspace]
   members = [
       "mcp/proto",
       "mcp/server",
       "mcp/client"
   ]
   ```
3. **CI boilerplate** – validate *lint*, *rust*, *dotnet*, and *unity* workflows compile on a clean checkout.
4. **Pre‑commit hooks** – add `rustfmt`, `clippy`, and `cargo deny` to `.pre‑commit‑config.yaml`.

---

## 1. Protocol Specification

5. Decide **frame format** – 4‑byte little‑endian length prefix.
6. Finalize **JSON‑RPC 2.0 field rules** (`id`, `method`, `params`, `result`, `error`).
7. Draft **error‑code matrix** (Markdown table).
8. Commit `docs/protocol.md` with examples for *Auth* and *HealthCheck*.

---

## 2. Shared Library – `mcp/proto`

9. Create crate skeleton (`Cargo.toml`, `src/lib.rs`).
10. Define `RpcRequest`, `RpcResponse`, `RpcError` structs with `serde` derives.
11. Implement **framer** API: `encode_frame` / `decode_frame`.
12. Add unit tests: normal, truncated, and malformed frames.
13. Generate docs (`cargo doc --no-deps` must be warning‑free).

---

## 3. MCP Server – `mcp/server`

14. Wire **async entry‑point** with `tokio` runtime.
15. Integrate `proto` crate for framing/parsing.
16. Implement **stdio reader task** and **writer queue**.
17. Add handlers:

    * `session.authenticate`
    * `transport.healthCheck`
18. Introduce **auth middleware** – reject all but allowlist before authentication.
19. Centralise error/timeout handling via `thiserror` + `anyhow`.
20. Configure structured logging with `tracing`.
21. Write unit tests for each handler.
22. Write integration test launching server as a subprocess and executing Auth→Ping via stdio.

---

## 4. CLI Client – `mcp/client`

23. Add subprocess utility (`std::process::Command`) to spawn the server binary.
24. Re‑export `proto` types to construct RPC payloads.
25. Implement helper `async fn call(method, params)` returning `Result<Value, RpcError>`.
26. **Success‑path integration test** – Auth then Ping.
27. **Failure cases** – unauthenticated Ping, malformed JSON, timeout.
28. Register tests in GitHub Actions matrix (Win/macOS/Linux).

---

## 5. Unity Bridge – `bridge/Packages/com.example.mcp‑bridge`

29. Generate UPM package skeleton (`package.json`, asmdef).
30. Implement `ProcessTransport` (C#) that starts the Rust server and streams stdio asynchronously.
31. Port frame encode/decode to C# (`System.Buffers.Binary`).
32. Add Editor Tests (PlayMode) performing Auth→Ping round‑trip.
33. Post‑build script copies compiled server binary to `Runtime~`.
34. Document package installation steps in `bridge/README.md`.

---

## 6. CI / DevOps

35. Extend **rust‑ci.yml** to build & test `proto`, `server`, and `client` crates.
36. Configure artifact upload of server binary for downstream Unity job.
37. Update **unity‑ci.yml** – depend on rust job via `needs:` and run Editor Tests headless.
38. Aggregate coverage with `grcov` and publish to Coveralls.

---

## 7. Documentation

39. Update `README.md` with quick‑start commands for each crate.
40. Add `CONTRIBUTING.md` covering lint, tests, and auth token setup.
41. Start `CHANGELOG.md` (`v0.1.0‑alpha – Sprint 1`).

---

## 8. Definition of Done ✅

* [ ] `cargo test --workspace` passes locally.
* [ ] GitHub Actions matrix is 100 % green.
* [ ] Unity Editor Tests pass in headless CI.
* [ ] `docs/protocol.md` merged and referenced from README.
* [ ] CHANGELOG entry *Sprint 1 completed* committed.

When these check‑boxes are all satisfied, the communication layer—**authentication & health‑check across Server, CLI, and Unity Bridge—will be CI‑guaranteed** for future Sprints.
