# Unity Bridge — Step-by-Step Work Plan (Steps 0–12)

This document defines a practical, PR‑oriented plan to implement and verify the Unity Bridge that runs a local gRPC server **inside the Unity Editor** and exposes minimal `EditorControl` endpoints. It is designed to be executed incrementally and reviewed per step.

> **Target**: Unity **6** (6000.x) or later — Editor‑only hosting with `Grpc.Core` (C‑core).

---

## Assumptions

- The Rust side already has a gRPC client and can call `EditorControl`.
- Protobuf definitions live under `proto/mcp/unity/v1/*.proto`.
- Bridge code is Editor‑only (no player builds) and lives under `bridge/Assets/Editor/`.

---

## Step 0 — Pre‑flight (Definition of Ready)

**Objective**: Lock the local endpoint, token policy, folder layout, and smoke calls.

**Tasks**

- **Endpoint**: `127.0.0.1:50061` (loopback only). Check collisions:
  - Windows: `netstat -ano | findstr LISTENING | findstr 50061`
  - macOS/Linux: `lsof -iTCP -sTCP:LISTEN -nP | grep 50061 || true`
- **Auth token** (optional for now): header `x-bridge-token`. Store in `BridgeSettings.asset` or `EditorPrefs`, not environment variables.
  - Generate: `openssl rand -hex 16` (or PowerShell one‑liner).
- **Layout**:
  - Generated C# → `Assets/Editor/Generated/Proto/` (vendored/committed)
  - Editor‑only assembly → `Assets/Editor/Bridge/Bridge.asmdef`
- **Rust smoke calls**: `Health`, `GetPlayMode`, `SetPlayMode(true/false)`.

**DoD**

- Readme notes the chosen port, token policy, and smoke commands; port verified unused.

---

## Step 1 — Protobuf → C# (Vendor the stubs)

**Objective**: Generate and commit the C# gRPC/Protobuf sources.

**Tasks**

- From repo root:
  ```bash
  PROTO_ROOT=proto
  OUT=bridge/Assets/Editor/Generated/Proto
  mkdir -p "$OUT"
  protoc \
    -I"$PROTO_ROOT" \
    --csharp_out="$OUT" \
    --grpc_out="$OUT" \
    --plugin=protoc-gen-grpc=/path/to/grpc_csharp_plugin \
    $(find "$PROTO_ROOT/mcp/unity/v1" -name '*.proto')
  ```
- Commit results so Unity can compile without MSBuild.

**DoD**

- Unity compiles with generated types present (no missing references).

---

## Step 2 — Editor‑only Assembly & Dependencies

**Objective**: Create Editor‑only asmdef and load `Grpc.Core` (plus native libgrpc) correctly.

**Tasks**

- Create `Bridge.asmdef` flagged **Editor** only.
- Import `Grpc.Core` DLLs and platform‑specific native libgrpc; mark **Editor only** in Import Settings.

**DoD**

- Unity Console shows no DLL/native load errors; `using Grpc.Core` resolves.

**Risks**

- Wrong native binary per OS. Validate on Windows/macOS/Linux dev machines.

---

## Step 3 — Hosting Layer (GrpcHost / BridgeServer)

**Objective**: Stand up a minimal host with start/stop and service registration.

**Tasks**

- `GrpcHost`: `Start()`, `Stop()`, `AddService(...)`, `UseInterceptor(...)`.
- `BridgeServer`: menu actions (*Bridge ▸ Start/Stop*) and/or auto‑start on Editor load; idempotent stop.

**DoD**

- Log lines appear on start/stop, e.g., `[Bridge] Started on 127.0.0.1:50061`.

**Risks**

- Double start. Guard with a state flag/lock.

---

## Step 4 — Abstractions (Unity API isolation)

**Objective**: Decouple Unity APIs for testability.

**Tasks**

- `IUnityEditorFacade` / `UnityEditorFacade`:
  - `IsPlaying()`, `SetPlayMode(bool)`, `HookLogEvents(...)`.
  - Dispatch play/stop to main thread (`EditorApplication.delayCall`).
- `IVersionProvider` (static or package‑based), `IClock` / `SystemClock`.

**DoD**

- EditorControl can operate without direct Unity API references outside the facade.

---

## Step 5 — Operation Model (minimal)

**Objective**: Provide an in‑memory operation to return from `SetPlayMode`.

**Tasks**

- `Operation { Id, Kind, Status, StartedAt, FinishedAt?, ErrorMessage?, Metadata }`.
- `IOperationStore` / `MemoryOperationStore` with thread‑safe dictionary.
- Statuses: `PENDING | RUNNING | SUCCEEDED | FAILED | CANCELED`.

**DoD**

- Store can create, complete, get (and optionally cancel) operations.

---

## Step 6 — EditorControl Service

**Objective**: Implement the 3 endpoints.

**Tasks**

- `Health` → `{ version, ready = true }` from `IVersionProvider`.
- `GetPlayMode` → `IsPlaying()` via facade.
- `SetPlayMode(play)` → create operation → set play/stop (main thread) → complete `SUCCEEDED` → return operation.

**DoD**

- Local calls via a test client succeed; Unity toggles on `SetPlayMode`.

**Risks**

- Exceptions during play toggle. On error, mark `FAILED` and include message.

---

## Step 7 — Operations Service (stub)

**Objective**: Expose `GetOperation` and a placeholder `CancelOperation`.

**Tasks**

- `GetOperation(id)` returns current state from store.
- `CancelOperation(id)` returns `{ accepted = false }` for now.

**DoD**

- Service compiles and returns store data; ready for future long‑running ops.

---

## Step 8 — Token Auth (optional baseline)

**Objective**: Add a metadata interceptor gated by settings.

**Tasks**

- `TokenAuthInterceptor` reads `x-bridge-token` and validates via `IAuthValidator`.
- `NoopAuthValidator` (accept all) or fixed token equality.

**DoD**

- When a token is configured, missing/invalid token results in `UNAUTHENTICATED`.

**Risks**

- Do not enable by default until team is ready to pass the header from Rust.

---

## Step 9 — UX/DX: Start/Stop & Settings

**Objective**: Make the bridge easy to operate.

**Tasks**

- Editor menu: *Bridge ▸ Start*, *Bridge ▸ Stop*.
- `BridgeSettings` ScriptableObject UI: `Port`, `Token`.
- Clear logs; show current state (running/not running).

**DoD**

- Non‑developers can start/stop and configure the bridge without touching code.

---

## Step 10 — Manual E2E (Rust ↔ Unity)

**Objective**: Validate the end‑to‑end path on a developer machine.

**Tasks**

- Start Bridge in Unity; from Rust client call:
  ```bash
  mcp-cli --host 127.0.0.1 --port 50061 health
  mcp-cli --host 127.0.0.1 --port 50061 get-playmode
  mcp-cli --host 127.0.0.1 --port 50061 set-playmode --play true
  mcp-cli --host 127.0.0.1 --port 50061 set-playmode --play false
  # If enabled:
  --header "x-bridge-token: <YOUR_TOKEN_HERE>"
  ```

**DoD**

- Health returns `ready=true`; GetPlayMode matches Editor; SetPlayMode toggles Editor and returns an operation with `SUCCEEDED`.

---

## Step 11 — Minimal Automation (optional)

**Objective**: Reduce manual effort and catch regressions.

**Tasks**

- Provide a tiny Rust CLI or script for the three calls.
- Optionally auto‑start Bridge on Editor load in a dedicated *test profile*.

**DoD**

- One‑command smoke test is possible on a dev machine.

---

## Step 12 — Polish & Documentation

**Objective**: Stabilize and document.

**Tasks**

- README updates: endpoint, token policy, troubleshooting.
- Idempotent start/stop, clearer errors (e.g., port in use), and native lib checks.
- Track known issues/TODO (long‑running ops, log streaming, strict auth/TLS).

**DoD**

- New contributors can start the Bridge and complete the smoke in <10 minutes using the docs.

---

## Suggested PR Breakdown

1. Step 1–2: Generated stubs + asmdef/deps (green build)
2. Step 3: Hosting skeleton (start/stop, no services)
3. Step 4–5: Abstractions + Operation model
4. Step 6–7: EditorControl + Operations stub
5. Step 8–9: Token interceptor + Settings/UI
6. Step 10–12: E2E smoke + automation + docs/polish

---

## Verification Checklist (roll‑up)

-

