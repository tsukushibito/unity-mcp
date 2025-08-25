# EditorDispatcher Adoption — Task List (Editor Bridge)

> Scope: Introduce a single, explicit main‑thread execution path for **all Unity Editor API** calls in the bridge, with minimal but decisive refactors to make this boundary obvious and safe. Destructive changes are allowed; cleanliness and maintainability take priority.

---

## Goals
- Enforce **IO on background**, **Unity API on main** via a single utility: `EditorDispatcher`.
- Remove ad‑hoc `delayCall + TaskCompletionSource` patterns and direct BG calls to `EditorApplication` / `Application`.
- Convert **Handshake** and **Health** to use the dispatcher (the two most error‑prone touchpoints).
- Keep the rest of the code compiling and runnable with the new path, even if further refactors (e.g., `OutboundWriter`, `ConnectionActor`) come later.

## Non‑Goals (for this milestone)
- Full actorization (`ConnectionActor`) and write‑path singletons.
- Broad package/namespace renames beyond what is necessary to compile.
- Event broadcast redesign.

---

## Milestone Plan (M0 → M4)

### M0 — Baseline & Inventory
- [ ] Run current bridge in Editor (playmode not required) and reproduce the runtime error from BG thread Unity API access.
- [ ] Inventory all **Unity API touchpoints** (reads/writes to `EditorApplication.*`, `Application.*`, `AssetDatabase.*`, etc.) and tag them `// UNITY_API`.
- [ ] Inventory all **BG origins** that may call into these (socket reader loops, `Task.Run`, async handlers).
- [ ] Establish a temporary logging tag `BRIDGE.THREAD` to print current thread id and main‑thread detection.

**Acceptance:** A list (or TODO comments) marking every Unity API usage and the place where it is invoked from.

---

### M1 — Introduce `EditorDispatcher`
- [ ] Create `Editor/Ipc/Infra/EditorDispatcher.cs` with:
  - [ ] `RunOnMainAsync(Action)`
  - [ ] `RunOnMainAsync<T>(Func<T>)`
  - [ ] `RunOnMainAsync<T>(Func<Task<T>>)`
  - [ ] Internal concurrent queue of `Func<Task>`; pump via `EditorApplication.update` in `[InitializeOnLoadMethod]`.
  - [ ] Ensure continuations run asynchronously (`TaskCreationOptions.RunContinuationsAsynchronously`).
  - [ ] Graceful exception surfacing: catch, log, and set on TCS.
- [ ] Add XML docs explaining **when** and **why** to use it.
- [ ] Add `UNITY_EDITOR && DEBUG` self‑test at load (enqueue a work item that asserts main thread).

**Acceptance:** You can enqueue a function that returns a value from the main thread and receive it from a background task deterministically.

---

### M2 — Thread‑Safety Scaffolding
- [ ] Add `Editor/Ipc/Infra/MainThreadGuard.cs` with `AssertMainThread()` (debug‑only; use `UnityEditorInternal.InternalEditorUtility.CurrentThreadIsMainThread()` if available, or equivalent heuristic).
- [ ] Add `Editor/Ipc/Infra/EditorStateMirror.cs`:
  - [ ] Static, updated every frame in `EditorApplication.update`.
  - [ ] Public volatile snapshots: `IsCompiling`, `IsUpdating`, `UnityVersion`.
  - [ ] **No** Unity API calls outside the update.
- [ ] Replace any stale, one‑time caches like `_cachedIsCompiling/_cachedIsUpdating` with reads from `EditorStateMirror`.

**Acceptance:** Reading `EditorStateMirror` from a BG thread never touches Unity APIs directly; tests confirm values move as Editor state changes.

---

### M3 — Convert the Two Critical Paths

#### M3‑A — Handshake (Hello → Welcome/Reject)
- [ ] Route **all** of the following through `EditorDispatcher.RunOnMainAsync(...)` in a single block:
  - [ ] `ValidateEditorState()` (uses `EditorApplication.*`).
  - [ ] `CreateWelcome(...)` (uses `Application.unityVersion`, `Application.platform`).
- [ ] Keep non‑Unity validations (token, protocol version, path IO) outside the dispatcher where safe; but **compose the final decision** on the main thread.
- [ ] Ensure the response is produced synchronously within the dispatcher block and queued to send.
- [ ] Add `MainThreadGuard.AssertMainThread()` inside `ValidateEditorState()` and `CreateWelcome()`.

**Acceptance:** Handshake runs without cross‑thread exceptions; logs show `BRIDGE.THREAD` main‑thread when composing Welcome/Reject.

#### M3‑B — Health
- [ ] Implement Health in one of two modes:
  - Mode **Strict**: wrap collection of `isCompiling/isUpdating/unityVersion` inside `RunOnMainAsync`.
  - Mode **Fast**: read from `EditorStateMirror` only (no dispatcher).
- [ ] Provide a feature flag or compile‑time define to switch modes easily.

**Acceptance:** Health request never calls Unity API from BG; returns correct values.

---

### M4 — Tests & Smoke Checks
- [ ] **Editor tests** (EditMode):
  - [ ] `EditorDispatcher` enqueues and executes on main thread.
  - [ ] Handshake path executes `ValidateEditorState`/`CreateWelcome` on main thread (use guard or logs to assert).
  - [ ] Health returns plausible values in both Strict and Fast modes.
- [ ] **Manual smoke**: start the bridge, connect a dummy client, perform Hello → Welcome, then query Health repeatedly while toggling domain reload/compilation.

**Acceptance:** All tests pass; no exceptions thrown from cross‑thread Unity API access.

---

## Change Map (Old → New)
- `EditorApplication.delayCall + TaskCompletionSource` → **`EditorDispatcher.RunOnMainAsync`**.
- BG thread reading `EditorApplication.isCompiling/isUpdating` → **`EditorStateMirror`** or **Strict Health via Dispatcher**.
- Handshake: BG‑origin `ValidateEditorState` / `CreateWelcome` → **Main‑thread block**.
- One‑shot static caches for Editor state → **per‑frame mirror** (volatile fields).

---

## File & Layout (minimal for this milestone)
```
Editor/
  Ipc/
    Infra/
      EditorDispatcher.cs
      EditorStateMirror.cs
      MainThreadGuard.cs
    // Existing files unchanged except for call‑sites
```

---

## Coding Guidelines (for this milestone)
- Any method that **touches Unity APIs** must either:
  1) Be called **only** from within a `RunOnMainAsync(...)` block, or
  2) Immediately assert main thread via `MainThreadGuard` at method entry.
- Prefer returning **data‑only DTOs** from dispatcher blocks; serialize/send outside the block.
- Use `TaskCreationOptions.RunContinuationsAsynchronously` for all TCS to avoid inline continuation surprises.

---

## Risks & Mitigations
- **Deadlocks**: Avoid waiting synchronously on `RunOnMainAsync` (`.Result`/`.Wait()`); always `await`.
- **Starvation**: Keep dispatcher work small; do heavy IO/compute outside, only Unity API bits inside.
- **Order‑sensitive state**: If Health in Fast mode reads the mirror mid‑frame, accept eventual consistency or use Strict mode for tests.

---

## Definition of Done (Milestone Complete)
- No BG thread ever calls Unity Editor APIs directly.
- Handshake and Health paths are refactored to the new boundary.
- Dispatcher, Mirror, Guard are in place, documented, and covered by EditMode tests.
- Bridge can accept a connection, reply Welcome/Reject, and answer Health without exceptions.

---

## Stretch (Post‑Adoption; not required here)
- Single writer per connection (`OutboundWriter`) and actorized connection (`ConnectionActor`).
- Event broadcast built on top of writer registry rather than raw stream access.
- Namespace cleanup and folderization of Core/Handlers/Infra across the whole bridge.

