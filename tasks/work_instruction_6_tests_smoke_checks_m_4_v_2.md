# Work Instruction 6 — Tests & Smoke Checks (M4)

> **Updates (2025‑08‑25)**
> - **Scripting Define location:** Configure build‑time switching in **Project Settings → Player → Scripting Define Symbols** (Editor target). *asmdef cannot define new symbols.*
> - **Operational guidance:** Default to **Fast** (mirror). Use **Strict** during verification/troubleshooting. Client‑side Health polling should target **0.5–1.0s** with debouncing/throttling.
> - Added note on **assembly reload** stability: tests should tolerate domain reloads; dispatcher/mirror are re‑initialized on load.
> - Added hint for **two‑stage guard** (`BRIDGE_THREAD_GUARD_STRICT`) usage during tests.

**Scope**: Prove the dispatcher boundary is enforced and that handshake/health function without cross‑thread violations.

---

## Objectives
- Add EditMode tests for dispatcher behavior, handshake, and health.
- Define manual smoke procedures to validate on a running Editor.
- Ensure logs and guards reveal violations quickly.

## Inputs & Preconditions
- M1–M3 complete.
- Test assembly definition present for EditMode tests.

## Deliverables
- `Editor/Ipc/Tests/EditorDispatcherTests.cs`
- `Editor/Ipc/Tests/HandshakeTests.cs`
- `Editor/Ipc/Tests/HealthTests.cs`
- `docs/bridge/m4-smoke-checks.md`

---

## Test Implementation Sketches

### 1) Dispatcher runs on main thread
```csharp
using System.Threading;
using System.Threading.Tasks;
using NUnit.Framework;
using Bridge.Editor.Ipc.Infra;

public class EditorDispatcherTests
{
    [Test]
    public async Task ReturnsOnMainThread()
    {
        var mainId = Thread.CurrentThread.ManagedThreadId;
        var id = await EditorDispatcher.RunOnMainAsync(() => Thread.CurrentThread.ManagedThreadId);
        Assert.AreEqual(mainId, id);
    }
}
```

### 2) Handshake produces Welcome on main
- Arrange: fake a valid Hello (token/version/path ok).
- Act: invoke the handshake handler; inside `ValidateEditorState`/`CreateWelcome`, assert `MainThreadGuard`.
- Assert: result is Welcome, contains `Application.unityVersion`.

Example check inside methods:
```csharp
MainThreadGuard.AssertMainThread(); // at top of ValidateEditorState/CreateWelcome
```

### 3) Health: Strict vs Fast
- Strict: enable `HEALTH_STRICT` (via **Project Settings → Player → Scripting Define Symbols** for Editor), call handler, assert it runs on main (guard), and version is non‑empty.
- Fast: remove define, call handler repeatedly while forcing a script recompile; assert booleans flip at some point.

---

## Manual Smoke Script
1. Start Unity Editor, open the project.
2. Ensure the bridge auto‑starts or start it manually.
3. Connect a simple IPC client (local loopback).
4. Send Hello → expect Welcome; check Editor log contains `[BRIDGE.THREAD MAIN]` during handshake.
5. Send Health repeatedly during/after a script change; observe stable responses and no exceptions.
6. Watch for any `MainThreadGuard` errors (should be none in normal flow).

### Optional: Stress
- Bombard Health at 10–20 req/sec to check latency and Editor responsiveness.
- Confirm no Editor freeze; if stutters occur, switch Health to Fast mode.

---

## Acceptance Criteria
- All EditMode tests pass locally and in CI (if applicable).
- Manual smoke passes with zero cross‑thread exceptions.
- Logs confirm main‑thread execution for Unity API touches.

## Risks & Mitigations
- **Flaky tests** caused by domain reloads: keep tests simple; avoid awaiting long editor operations. If needed, re‑run the test or design it to be resilient across reloads.
- **Client dependency**: keep handshake/health tests decoupled from actual sockets by testing handlers/invokers directly.

