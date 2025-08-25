# Work Instruction 2 — Implement `EditorDispatcher` (M1)

> **Updates (2025‑08‑25)**
> - Added **assembly reload** handling guidance (clear/cancel queue on `beforeAssemblyReload`, re‑init on `afterAssemblyReload`).
> - Added optional **frame budget** for Pump (cap items per frame or time‑slice by `Time.realtimeSinceStartup`).
> - Clarified **dispatcher invariants**: queue is private; only public `RunOnMainAsync` APIs can enqueue.
> - Tests section clarified about **main‑thread detection** in EditMode.

**Scope**: Provide a single, safe path to execute code on Unity’s main thread from background contexts.

---

## Objectives
- Implement `EditorDispatcher` with a queued work model.
- Guarantee that Unity API calls can be marshaled to the main thread deterministically.
- Provide ergonomic overloads and robust exception propagation.

## Inputs & Preconditions
- M0 inventory completed; risky call sites are tagged.
- Unity Editor environment for EditMode tests.

## Deliverables
- `Editor/Ipc/Infra/EditorDispatcher.cs` with public APIs:
  - `Task RunOnMainAsync(Action)`
  - `Task<T> RunOnMainAsync<T>(Func<T>)`
  - `Task<T> RunOnMainAsync<T>(Func<Task<T>>)`
- XML docs and usage examples.
- Minimal EditMode test proving main‑thread execution.

---

## Design
- Use a `ConcurrentQueue<Func<Task>>` to store work items.
- Drive the queue from `EditorApplication.update` (Editor main thread).
- Use `TaskCompletionSource<T>(RunContinuationsAsynchronously)` for awaiters.
- Do **not** block the main thread; each item should be small and fast.
- **Assembly reload**: clear or cancel pending items on reload; re‑subscribe on load.

## Implementation

```csharp
// Editor/Ipc/Infra/EditorDispatcher.cs
using System;
using System.Collections.Concurrent;
using System.Threading.Tasks;
using UnityEditor;
using UnityEngine;

namespace Bridge.Editor.Ipc.Infra
{
    internal static class EditorDispatcher
    {
        private static readonly ConcurrentQueue<Func<Task>> Q = new();
        private const int MaxItemsPerFrame = 256; // optional cap

        [InitializeOnLoadMethod]
        private static void Init()
        {
            AssemblyReloadEvents.beforeAssemblyReload += OnBeforeReload;
            AssemblyReloadEvents.afterAssemblyReload  += OnAfterReload;
            EditorApplication.update += Pump;
        }

        private static void OnBeforeReload()
        {
            // best‑effort cancel/clear to avoid orphaned TCS
            while (Q.TryDequeue(out var _)) { }
        }

        private static void OnAfterReload()
        {
            // nothing specific for now; Pump subscription is set in Init
        }

        private static void Pump()
        {
            var processed = 0;
            var frameDeadline = Time.realtimeSinceStartup + 0.002f; // 2ms slice (optional)
            while (Q.TryDequeue(out var work))
            {
                try { _ = work(); }
                catch (Exception ex) { Debug.LogException(ex); }
                if (++processed >= MaxItemsPerFrame || Time.realtimeSinceStartup > frameDeadline)
                    break; // let next frame continue
            }
        }

        public static Task RunOnMainAsync(Action action)
            => RunOnMainAsync<object>(() => { action(); return null; });

        public static Task<T> RunOnMainAsync<T>(Func<T> func)
        {
            var tcs = new TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously);
            Q.Enqueue(() =>
            {
                try { tcs.SetResult(func()); }
                catch (Exception ex) { tcs.SetException(ex); }
                return Task.CompletedTask;
            });
            return tcs.Task;
        }

        public static Task<T> RunOnMainAsync<T>(Func<Task<T>> func)
        {
            var tcs = new TaskCompletionSource<T>(TaskCreationOptions.RunContinuationsAsynchronously);
            Q.Enqueue(async () =>
            {
                try { tcs.SetResult(await func().ConfigureAwait(false)); }
                catch (Exception ex) { tcs.SetException(ex); }
            });
            return tcs.Task;
        }
    }
}
```

### Usage Example
```csharp
var version = await EditorDispatcher.RunOnMainAsync(() => UnityEngine.Application.unityVersion);
```

### Anti‑Patterns to Avoid
- Blocking waits (`.Result`, `.Wait()`) on dispatcher tasks → can deadlock in Editor. Always `await`.
- Long‑running operations inside dispatcher blocks (I/O, sleeps). Only touch Unity API; do heavy work on BG threads.
- Enqueuing directly into the queue from outside → **not allowed**. Use only public `RunOnMainAsync` APIs.

---

## Test (EditMode)
Create `Editor/Ipc/Infra/EditorDispatcherTests.cs` and verify main‑thread execution.

```csharp
using System.Threading;
using System.Threading.Tasks;
using NUnit.Framework;
using Bridge.Editor.Ipc.Infra;

public class EditorDispatcherTests
{
    [Test]
    public async Task RunsOnMainThread()
    {
        var mainId = Thread.CurrentThread.ManagedThreadId; // EditMode test runs on main
        var calledId = await EditorDispatcher.RunOnMainAsync(() => Thread.CurrentThread.ManagedThreadId);
        Assert.AreEqual(mainId, calledId);
    }
}
```

> **Note:** If relying on internal APIs for main‑thread detection (e.g., `UnityEditorInternal.InternalEditorUtility.CurrentThreadIsMainThread()`), keep a fallback using thread IDs to avoid brittleness.

---

## Acceptance Criteria
- Dispatcher compiles and pumps on Editor load (and across assembly reloads).
- Example usage returns expected values and exceptions propagate to awaiters.
- Optional frame budget prevents long frames under a flood of work items.
- EditMode test passes.

## Risks & Mitigations
- **Starvation**: If BG floods the queue, Editor stutters. Keep work small; cap items per frame or time‑slice.
- **Silent failures**: Always log exceptions in `Pump()` and propagate via `TCS`.
- **Assembly reload**: Clear queue before reload to avoid orphaned tasks.

