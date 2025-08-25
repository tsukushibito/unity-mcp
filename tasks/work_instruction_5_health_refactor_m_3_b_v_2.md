# Work Instruction 5 — Health Refactor (M3‑B)

> **Updates (2025‑08‑25)**
> - **Scripting Define location:** Configure build‑time switching in **Project Settings → Player → Scripting Define Symbols** (Editor target). *asmdef cannot define new symbols.*
> - Added **operational guidance**: Default **Fast** (mirror). Use **Strict** during verification/troubleshooting. Recommend client‑side Health polling at **0.5–1.0s** with debouncing/throttling.

**Scope**: Make Health reporting safe and flexible. Support two modes: **Strict** (use dispatcher, always main‑thread reads) and **Fast** (use `EditorStateMirror` snapshots).

---

## Objectives
- Eliminate BG reads of `EditorApplication.*` / `Application.*`.
- Provide a build‑time or runtime switch to choose between Strict vs Fast.
- Keep the handler small and side‑effect‑free.

## Inputs & Preconditions
- M2 `EditorStateMirror` and `MainThreadGuard` are available.

## Deliverables
- Health handler using Strict or Fast mode.
- Toggle mechanism via Scripting Define or runtime flag.

---

## Design

### Mode A — Strict (Main‑thread)
- Always collect `isCompiling`, `isUpdating`, `unityVersion` inside `EditorDispatcher.RunOnMainAsync`.
- Strongest correctness; slightly higher latency under load.

### Mode B — Fast (Mirror)
- Read `EditorStateMirror` fields from BG.
- Eventual consistency; minimal latency.

---

## Implementation

```csharp
// Editor/Ipc/Handlers/HealthHandler.cs
using System.Threading.Tasks;
using Bridge.Editor.Ipc.Infra;

internal static class HealthHandler
{
#if HEALTH_STRICT
    public static async Task<IpcResponse> HandleAsync(HealthRequest req)
    {
        var snap = await EditorDispatcher.RunOnMainAsync(() => new
        {
            compiling = UnityEditor.EditorApplication.isCompiling,
            updating  = UnityEditor.EditorApplication.isUpdating,
            version   = UnityEngine.Application.unityVersion,
        });
        return IpcResponse.Health(new HealthResponse
        {
            IsCompiling = snap.compiling,
            IsUpdating  = snap.updating,
            UnityVersion = snap.version,
        });
    }
#else
    public static Task<IpcResponse> HandleAsync(HealthRequest req)
    {
        var resp = new HealthResponse
        {
            IsCompiling = EditorStateMirror.IsCompiling,
            IsUpdating  = EditorStateMirror.IsUpdating,
            UnityVersion = EditorStateMirror.UnityVersion,
        };
        return Task.FromResult(IpcResponse.Health(resp));
    }
#endif
}
```

### Switching Modes
- **Build‑time**: Add/remove `HEALTH_STRICT` under **Project Settings → Player → Scripting Define Symbols** (Editor).
- **Runtime** (optional): keep both implementations and choose based on a config flag read once at startup.

---

## Tests
- Strict mode: assert handler runs on main (guard/logs) and returns non‑empty version.
- Fast mode: toggle Editor compiling state (e.g., force script recompile) and observe mirror changes across multiple requests.

## Acceptance Criteria
- No BG code calls `EditorApplication`/`Application` directly.
- Health responses are stable and match Editor state per selected mode.

## Risks & Mitigations
- **Mirror staleness**: Document that Fast mode is eventually consistent; prefer Strict for critical gating.
- **Performance**: If Strict causes UI jank under high request rate, throttle or coalesce Health requests client‑side.

