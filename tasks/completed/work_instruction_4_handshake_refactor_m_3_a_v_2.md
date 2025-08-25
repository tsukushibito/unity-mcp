# Work Instruction 4 — Handshake Refactor (M3‑A)

> **Updates (2025‑08‑25)**
> - Clarified decision flow: **BG‑safe checks first**, then **final decision and payload construction inside a single Dispatcher block**. Reject may be decided without touching Unity APIs, but message assembly is unified on main for consistency. Any BG‑immediate‑reject optimization is **out of scope** for this milestone.

**Scope**: Make Hello → Welcome/Reject fully main‑thread‑safe. All Unity API touches are marshaled through `EditorDispatcher`.

---

## Objectives
- Ensure `ValidateEditorState()` and `CreateWelcome(...)` run on the main thread.
- Keep non‑Unity validations (token, protocol version, filesystem checks) off the main thread when possible.
- Produce the final control message (Welcome or Reject) inside a dispatcher block and queue for sending.

## Inputs & Preconditions
- M1 `EditorDispatcher` and M2 scaffolding in place.
- Existing handshake code path (reader loop → message parse → handler).

## Deliverables
- Refactored handshake handler that never calls Unity APIs from BG.
- Unit/Editor tests covering success and Reject paths.

---

## Step‑by‑Step

### 1) Identify the handshake entry point
Locate the place where the incoming Hello control is decoded (connection reader loop or control handler).

### 2) Split validations
- **BG‑safe** (can run before dispatcher): token verification, protocol/IPC version compatibility, project path checks (`System.IO`).
- **Unity‑touching** (must run in dispatcher): `ValidateEditorState()` (uses `EditorApplication.*`), `CreateWelcome()` (uses `Application.unityVersion`, `Application.platform`).

### 3) Refactor to a single main‑thread block

```csharp
// Pseudocode inside connection handler
var hello = control.Hello;

// BG‑safe checks first
var tok = ValidateToken(hello.Token);
var ver = ValidateProtocol(hello.IpcVersion);
var pathOk = ValidateProjectRoot(hello.ProjectRoot);

var ctrl = await EditorDispatcher.RunOnMainAsync(() =>
{
    // Do not touch Unity APIs for an early Reject decision
    if (!tok.IsValid) return ControlReject("invalid_token");
    if (!ver.IsValid) return ControlReject("protocol_mismatch");
    if (!pathOk.IsValid) return ControlReject("invalid_project_root");

    // Unity touches must be here
    MainThreadGuard.AssertMainThread();

    var editorOk = ValidateEditorState(); // EditorApplication.*
    if (!editorOk.IsValid) return ControlReject("editor_unavailable");

    var welcome = CreateWelcome(hello);    // Application.unityVersion/platform
    RegisterFeaturesForConnection(connectionId, welcome.AcceptedFeatures);
    return ControlWelcome(welcome);
});

EnqueueToSend(ctrl); // current send path; single writer is a later milestone
```

### 4) Harden `ValidateEditorState()` and `CreateWelcome()`
- Add `MainThreadGuard.AssertMainThread()` at method entry.
- Remove any accidental I/O or long CPU work from these methods.

### 5) Feature negotiation locality
- Store negotiation results (e.g., `FeatureGuard`) in the **connection object/state**, not in a global map keyed by stream if feasible.

---

## Tests

### Editor test: Success path
- Simulate a valid Hello (token/version/path ok).
- Inside the dispatcher block, stub `ValidateEditorState` to return valid.
- Assert the result is a Welcome control and guard confirms main thread.

### Editor test: Reject cases
- Token/Version/Path reject: confirm **no Unity API** was touched before Reject decision; payload still constructed on main.
- Editor reject: force `ValidateEditorState` to fail; expect Reject.

---

## Acceptance Criteria
- No Unity API is invoked from BG during handshake (verified by guard/logs).
- Welcome includes correct `Application.unityVersion` and platform.
- Reject messages are produced deterministically, with early decisions not touching Unity APIs.

## Risks & Mitigations
- **Ordering pitfalls**: Keep all final message construction in the main‑thread block to avoid TOCTOU issues.
- **Latency**: Handshake now hops to main thread; keep the block small and pure.

