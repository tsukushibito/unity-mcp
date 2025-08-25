# Work Instruction 3 — Thread‑Safety Scaffolding (M2)

> **Updates (2025‑08‑25)**
> - Added **two‑stage guard** option via `BRIDGE_THREAD_GUARD_STRICT` (throw vs log error).
> - `EditorStateMirror` now performs an **immediate first refresh** at init to avoid "unknown" version at startup.

**Scope**: Introduce supporting utilities that make thread correctness obvious and cheap: `MainThreadGuard` for assertions and `EditorStateMirror` for BG‑safe reads.

---

## Objectives
- Add a debug‑only guard to catch accidental BG access to Unity APIs.
- Provide a per‑frame mirror of selected Editor state for fast, BG‑safe reads.
- Replace stale one‑shot caches with the mirror.

## Inputs & Preconditions
- `EditorDispatcher` (M1) is available.
- M0 list of Unity API touchpoints and stale caches.

## Deliverables
- `Editor/Ipc/Infra/MainThreadGuard.cs`
- `Editor/Ipc/Infra/EditorStateMirror.cs`
- Replacements of `_cachedIsCompiling/_cachedIsUpdating/...` with mirror reads.

---

## Implementation

### 1) `MainThreadGuard`
```csharp
// Editor/Ipc/Infra/MainThreadGuard.cs
#if UNITY_EDITOR
using System.Threading;
using UnityEngine;
using UnityEditor;

namespace Bridge.Editor.Ipc.Infra
{
    internal static class MainThreadGuard
    {
        private static int _mainId;

        [InitializeOnLoadMethod]
        private static void Init()
        {
            _mainId = Thread.CurrentThread.ManagedThreadId;
        }

        [System.Diagnostics.Conditional("UNITY_EDITOR"), System.Diagnostics.Conditional("DEBUG")]
        public static void AssertMainThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != _mainId)
            {
#if BRIDGE_THREAD_GUARD_STRICT
                throw new System.InvalidOperationException($"Unity API on BG thread. Expected main={_mainId}, got={Thread.CurrentThread.ManagedThreadId}");
#else
                Debug.LogError($"Unity API on BG thread. Expected main={_mainId}, got={Thread.CurrentThread.ManagedThreadId}");
#endif
            }
        }
    }
}
#endif
```

### 2) `EditorStateMirror`
```csharp
// Editor/Ipc/Infra/EditorStateMirror.cs
#if UNITY_EDITOR
using UnityEditor;
using UnityEngine;

namespace Bridge.Editor.Ipc.Infra
{
    internal static class EditorStateMirror
    {
        public static volatile bool IsCompiling;
        public static volatile bool IsUpdating;
        public static volatile string UnityVersion = "unknown";

        [InitializeOnLoadMethod]
        private static void Init()
        {
            // Immediate first refresh to avoid "unknown" at startup
            RefreshOnce();
            EditorApplication.update += () => RefreshOnce();
        }

        private static void RefreshOnce()
        {
            IsCompiling = EditorApplication.isCompiling;
            IsUpdating  = EditorApplication.isUpdating;
            UnityVersion = Application.unityVersion;
        }
    }
}
#endif
```

### 3) Replace stale caches
- Delete any `_cachedIsCompiling/_cachedIsUpdating` fields.
- Replace reads with `EditorStateMirror.IsCompiling/IsUpdating`.
- Where strong correctness is required (e.g., handshake), prefer `EditorDispatcher.RunOnMainAsync` instead of the mirror.

---

## Coding Rules
- Any method touching Unity APIs should begin with `MainThreadGuard.AssertMainThread()` (debug‑only) **or** be called strictly inside a dispatcher block.
- The mirror may only be **written** from the main thread (`EditorApplication.update`). Reads are allowed anywhere.

## Acceptance Criteria
- Guard throws (strict) or logs (default) when a Unity API is called from BG in DEBUG builds.
- Mirror values change as the Editor transitions compilation/update states and are initialized immediately.
- All stale caches are replaced; no direct BG reads of `EditorApplication.*` remain.

## Risks & Mitigations
- **Mirror staleness**: It is eventually consistent by design. Use dispatcher for critical checks.
- **Guard noise**: Keep `throw` behind `BRIDGE_THREAD_GUARD_STRICT`; default to `LogError` for developer convenience.

