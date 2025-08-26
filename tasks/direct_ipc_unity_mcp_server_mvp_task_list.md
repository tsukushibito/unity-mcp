# Direct-IPC Unity MCP Server — MVP Task List

> NOTE (Frozen Roadmap): This document is kept as a reference roadmap for the MVP scope and definition of done. Active progress tracking has moved to: `tasks/mvp_worklist_checklist.md` (MVP Closeout Checklist). Please update progress there. (Last banner update: 2025-08-26)

**Goal** Ship a minimum viable Unity MCP Server that communicates with the Unity Editor via a direct IPC transport (initially TCP on localhost), exposes a small but useful tool surface over MCP (health, logs, core asset ops, minimal build), and is reproducible locally and in CI.

**Scope (MVP must include)**

- Direct IPC handshake (auth token, version/features, schema hash) with strict validation
- Rust IpcClient fully functional (framing, correlation, response routing, event stream)
- MCP tools: health, asset basics (path↔GUID, import, refresh), minimal build kickoff with operation events
- Log streaming: Unity → Rust → surfaced to MCP/logs
- Unified error vocabulary + mapping policy; clear, one-line messages
- Baseline security (token required, path policy enforced)
- Runtime deps resolved on the Unity side (Google.Protobuf et al.)
- CI baseline (Rust build/test/lint + proto parity check)
- Developer quickstart doc

**Out of scope for MVP (mark as Stretch)**

- UDS/Named Pipe transports as default (keep TCP for MVP)
- Cancel/Retry for operations
- Tracing/metrics and rich log levels

---

## Milestone Definition of Done (MVP)

1. A fresh clone can run **Unity Editor** in a sample project, start the Editor IPC server, then run the Rust server and:
   - call `unity.health` and receive `{ ready: true, version: … }`.
   - stream Unity logs into the Rust process.
   - resolve path↔GUID and perform `Import/Refresh` successfully.
   - start a minimal build and see operation progress/complete events.
2. CI passes on Linux (Rust build/test/clippy/fmt + proto parity check).
3. README/Quickstart allows a new dev to reproduce the above in <15 minutes.

---

## Task List (in execution order)

### T01 — Freeze and Implement IPC Handshake

**Why:** Establish trust and compatibility before any request/response.

**Work:**

- Specify `Hello/Welcome` messages (token, ipc\_version, features[], schema\_hash of proto set).
- Unity: validate token, echo accepted features, reply Welcome; structured error on failure.
- Rust: implement handshake in `IpcClient::connect()`; fail-fast with readable diagnostics.

**Acceptance:**

- Rust logs show `Handshake OK: version=X, features=[…], schema=…`.
- Invalid token → connection refused with explicit code/message.

---

### T02 — Complete Rust IpcClient

**Why:** Reliable request/response and event streaming.

**Work:**

- Length-prefixed framing and robust decode.
- Correlation ID generator; pending map with timeouts.
- Response routing and backpressure; broadcast channel for events.
- Reconnect policy (basic): exponential backoff, replay of in-flight? (No; fail pending with retryable error).
- Unit tests: framing, correlation, timeout, reconnect.

**Acceptance:**

- Unit tests pass; soak test: 1k echo requests round-trip without leak.

---

### T03 — Health Tool E2E

**Why:** Fast sanity probe from MCP clients.

**Work:**

- MCP tool `unity.health` → Unity Health IPC → response mapping.
- Include server/editor versions in payload; keep message < 1 line per field.

**Acceptance:**

- `unity.health` returns `{ ready: true, server_version, editor_version }`.

---

### T04 — Log Stream Wiring

**Why:** Observability for agents and developers.

**Work:**

- Unity: forward Editor logs as `IpcEvent.Log`.
- Rust: subscribe to event stream, print to structured logger, optionally surface to MCP notifications later.

**Acceptance:**

- Creating a Console log in Unity appears in Rust logs in near-real-time.

---

### T05 — Assets Basics E2E

**Why:** Foundational editor control.

**Work:**

- Path normalization + policy (reject traversal, outside project).
- Path↔GUID convert; `ImportAsset`, `Refresh` minimal support.
- Rust MCP tools: `unity.assets.path_to_guid`, `guid_to_path`, `import`, `refresh`.
- Tests: round-trip conversions; import of a dummy asset.

**Acceptance:**

- Round-trip path↔GUID produces the same canonical path.
- Importing a new file then `Refresh` makes it visible to Unity database.

---

### T06 — Minimal Build with Operation Events

**Why:** Demonstrate a long-running job with progress.

**Work:**

- Unity: start build (headless acceptable), emit `OperationStarted/Progress/Completed` events.
- Rust: MCP tool `unity.build.player(start)` returns `operation_id`; subscribe and log progress; final result surface as structured JSON.

**Acceptance:**

- Starting a build returns an `operation_id`; progress events arrive; completion carries summary (duration, result path, success flag).

---

### T07 — Error Vocabulary & Mapping Policy

**Why:** Predictable automation and debugging.

**Work:**

- Define a small, stable set (e.g., INVALID\_ARGUMENT, NOT\_FOUND, FAILED\_PRECONDITION, UNAUTHENTICATED, INTERNAL, UNAVAILABLE, DEADLINE\_EXCEEDED).
- Map Unity-side failures to these; Rust maps to MCP `ErrorData`.
- Document table in `/docs/errors.md` and reference from server logs.

**Acceptance:**

- Intentional bad inputs produce expected codes with a single-sentence message.

---

### T08 — Security Baseline

**Why:** Prevent accidental or malicious writes.

**Work:**

- Token required for handshake (configurable via env/flag during dev).
- PathPolicy: allowlist project subfolders (e.g., `Assets/`, `ProjectSettings/` read-only), deny absolute/system paths.
- Build output policy: default to `Builds/` under project.

**Acceptance:**

- Any path escaping project root is rejected with `FAILED_PRECONDITION`.
- Missing token blocks the session.

---

### T09 — Unity Runtime Dependencies Resolved

**Why:** Eliminate missing DLL issues.

**Work:**

- Bundle `Google.Protobuf.dll` (and any transitive essentials) under `Editor/Plugins/` with .meta.
- Alternatively declare a UPM dependency if available and stable.

**Acceptance:**

- Unity Editor loads the package without reference resolution errors.

---

### T10 — CLI/Examples for Local Dev

**Why:** One-command sanity workflows.

**Work:**

- `examples/test_unity_ipc.rs`: connect, handshake, health, log tail for 10s.
- A small cargo alias or shell script to run it.

**Acceptance:**

- `cargo run --example test_unity_ipc` succeeds end-to-end with Unity running.

---

### T11 — CI Baseline

**Why:** Keep the trunk green and schema-aligned.

**Work:**

- GitHub Actions (Linux): `cargo build`, `cargo test`, `fmt --check`, `clippy -D warnings`.
- Proto parity check: ensure generated sources match proto (Rust via build.rs; C# via script that re-generates and diffs, or checksum of proto tree).

**Acceptance:**

- CI passes; changes to proto without regen cause CI failure with actionable message.

---

### T12 — Developer Quickstart & Docs

**Why:** Lower onboarding friction.

**Work:**

- Update `/server/README.md` or `/docs/quickstart.md` for direct-IPC flow:
  1. Open Unity sample project; start Editor IPC server (playmode not required).
  2. Set `MCP_IPC_TOKEN` env (dev token) and `MCP_ENDPOINT` (tcp\://127.0.0.1:7777).
  3. `cargo run --example test_unity_ipc`; then run the MCP server/tools.
- Include troubleshooting (port busy, token mismatch, protobuf DLL missing).

**Acceptance:**

- A new developer (not original author) confirms they reached Milestone DoD following the doc.

---

## Stretch (Post-MVP)

- **S01 — UDS/Named Pipe Transports:** Make them default per-OS; keep TCP as override.
- **S02 — Cancel/Retry Operations:** IPC verb + server-side state machine.
- **S03 — Structured Logging & Verbosity Flags:** JSON logs with request IDs.
- **S04 — Tracing/Metrics:** Otel exporter optional; spans around build/asset ops.
- **S05 — Tool Surface Growth:** Asset move/delete, selection queries, playmode control.

---

## Dependency Graph (Lightweight)

- T01 → T02 → T03/T04 → T05 → T06 → T07/T08 → T09/T10 → T11 → T12

## Ownership Hints

- **Rust server:** T01 (client side), T02, T03, T04 (sink), T05, T06 (MCP side), T07 (mapping), T08 (validation), T10–T12
- **Unity bridge:** T01 (server side), T04 (source), T05, T06 (build ops), T07 (origin), T08 (PathPolicy), T09

## Tracking Template (copy per task)

```
Task: Txx — <title>
Owner:
PR(s):
Status: ☐ Todo ☐ In progress ☐ Review ☐ Done
Notes:
```
