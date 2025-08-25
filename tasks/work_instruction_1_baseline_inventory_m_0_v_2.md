# Work Instruction 1 — Baseline & Inventory (M0)

**Scope**: Establish a factual baseline of where Unity Editor APIs are touched and from which threads they are invoked. Tag all risky call sites and prepare the ground for the dispatcher refactor.

---

## Objectives
- Reproduce the current cross‑thread runtime errors.
- Inventory **all Unity API touchpoints** and **all BG (background) origins**.
- Introduce temporary diagnostics to reveal threads and call paths.
- Produce a crisp change map for subsequent refactors.

## Inputs & Preconditions
- Current bridge source (Editor side) builds in Unity Editor.
- EditMode tests are available (or create a placeholder test assembly).
- Ability to run a simple IPC client to trigger Hello/Health.

## Deliverables
- `docs/bridge/m0-inventory.md` (or issue comments) containing:
  - List of Unity API calls (file:line, method, API used).
  - List of BG origins (reader loops, `Task.Run`, timers, `async void`, etc.).
  - A table mapping **caller → callee (Unity API)** and the **thread** observed.
- Temporary logging helper committed behind `#if UNITY_EDITOR && DEBUG`.

---

## Step‑by‑Step

### 1) Enable thread diagnostics
Create a small helper to tag logs with thread info.

```csharp
// Editor/Ipc/Infra/Diag.cs
#if UNITY_EDITOR
using System;
using System.Threading;
using UnityEngine;

internal static class Diag
{
    public static int MainThreadId { get; private set; }

    [UnityEditor.InitializeOnLoadMethod]
    private static void Init()
    {
        MainThreadId = Thread.CurrentThread.ManagedThreadId;
        Log($"MainThreadId={MainThreadId}");
    }

    public static string ThreadTag() =>
        Thread.CurrentThread.ManagedThreadId == MainThreadId ? "MAIN" : "BG";

    public static void Log(string msg)
        => Debug.Log($"[BRIDGE.THREAD {ThreadTag()}] {msg}");
}
#endif
```

### 2) Reproduce the error and capture logs
- Start the Editor, run the bridge, connect the client.
- Trigger Hello → Health.
- Capture any exceptions and correlate with `[BRIDGE.THREAD]` logs.

### 3) Inventory Unity API touchpoints
Search the codebase for common Editor/Engine APIs:

- `UnityEditor.` (e.g., `EditorApplication`, `AssetDatabase`, `CompilationPipeline`)
- `UnityEngine.Application`
- `UnityEngine.Object` mutations (load/destroy/create)

Record findings as rows:

| File | Method | API | Notes |
|------|--------|-----|-------|
| `.../EditorIpcServer.cs:123` | `ValidateEditorState` | `EditorApplication.isCompiling` | read |
| `...` | `CreateWelcome` | `Application.unityVersion` | read |

### 4) Inventory BG origins
Search for:
- `Task.Run(`, `new Thread(`, `ThreadPool.QueueUserWorkItem(`
- `async void` methods (consider replacing later)
- Long‑running loops around sockets/streams

Record caller → callee chains where a BG origin may end up calling a Unity API.

### 5) Mark risky call sites in code
Add a temporary comment tag at call sites that touch Unity APIs **or** sit on a BG origin path:

```csharp
// TODO(UNITY_API): touches EditorApplication — must run on main via EditorDispatcher
```

### 6) Summarize change impact
- Which features are blocked by the cross‑thread issue (Handshake, Health, Assets, Build)?
- Which calls can be trivially moved (pure reads) vs. those needing careful sequencing (stateful operations)?

---

## Acceptance Criteria
- A complete table of Unity API touchpoints and BG origins exists and is checked into the repo or an issue.
- Logs clearly show which paths execute on `MAIN` vs `BG`.
- All risky sites are marked with `TODO(UNITY_API)` comments.

## Rollback / Cleanup
- Keep `Diag.cs`; it is harmless and useful during refactors. You may later gate it with a scripting define (e.g., `BRIDGE_DIAG`).

## Risks & Mitigations
- **False negatives**: Some call chains may be missed; rely on grep + code review + run‑time logs.
- **Noise**: Logging can be verbose; keep it behind `#if UNITY_EDITOR && DEBUG`.

