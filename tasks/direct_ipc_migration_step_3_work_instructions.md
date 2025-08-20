# Step 3 — Event Streaming: **Logs** & **Operations** over IPC (with reconnection)

**Objective:** Extend the IPC protocol and implementations so Unity can push **asynchronous events** to Rust: editor logs and long‑running operation updates. Add a reconnect‑robust client on Rust, with backpressure and throttling safeguards.

> Prereqs: Step 0 (message‑only Protobuf), Step 1 (Rust `IpcClient` basic request/response), Step 2 (Unity `EditorIpcServer` handshake + `Health`). This step adds bidirectional eventing and a reconnect loop.

---

## 0) Protocol Additions (messages)
Ensure the `.proto` set contains these events and enums. If not present, add them now and regenerate.

```proto
message LogEvent {
  enum Level { TRACE=0; DEBUG=1; INFO=2; WARN=3; ERROR=4; }
  int64  monotonic_ts_ns = 1;    // sender clock
  Level  level           = 2;
  string message         = 3;
  string category        = 4;    // e.g., "Unity", "Build", "Assets"
  string stack_trace     = 5;    // optional, compacted
}

message OperationEvent {
  enum Kind { START=0; PROGRESS=1; COMPLETE=2; }
  string op_id       = 1;        // unique per operation
  Kind   kind        = 2;
  int32  progress    = 3;        // 0..100, for PROGRESS/COMPLETE
  int32  code        = 4;        // 0=OK; nonzero=error codes on COMPLETE
  string message     = 5;        // short human message
  string payload_json= 6;        // optional structured data
}

message IpcEvent {
  int64 monotonic_ts_ns = 1;
  oneof payload {
    LogEvent       log = 10;
    OperationEvent op  = 11;
  }
}
```

No correlation ID is required for events.

---

## 1) Dependencies to add (commands)
Run in `server/`.

```bash
# Structured logging and setup
cargo add tracing
cargo add tracing-subscriber --features "fmt"

# Streams/utilities if not already present
cargo add tokio-stream
```

> Keep versions unspecified; rely on `Cargo.lock`.

---

## 2) Rust — IpcClient: event read‑loop & reconnection

### 2.1 Reader routes **Response** vs **Event**
Your Step 1 reader already demuxes `IpcEnvelope`. Extend it so `IpcEvent` is broadcast. Keep the existing `broadcast::Sender<pb::IpcEvent>`.

### 2.2 Reconnection loop
Add a supervisor task that:
- Detects connection loss (reader exits)
- Retries `connect_endpoint()` with exponential backoff (e.g., 200ms → 400ms → … → max 5s + jitter)
- On reconnect, re‑do handshake and resume the reader; **fail and drop** all in‑flight requests; keep the `events_tx` channel alive

Sketch:
```rust
// in IpcClient::connect, spawn a supervisor task
// Pseudocode
loop {
  match establish_connection(&inner).await { Ok(()) => { wait_until_closed().await; } Err(e) => { /* log */ } }
  backoff.sleep().await; // cap at 5s
}
```

> Keep writer side as an mpsc sender; when disconnected, `send()` returns error → surface to callers.

### 2.3 Public API for events
```rust
pub fn events(&self) -> broadcast::Receiver<pb::IpcEvent> { self.inner.events_tx.subscribe() }
```
Consumers attach `tokio_stream::wrappers::BroadcastStream` or receive in their own task.

---

## 3) Rust — Server integration: log handling and op tracking

### 3.1 Log subscription & throttling
Create a task that listens to `IpcClient::events()` and forwards WARN/ERROR immediately, while **throttling INFO/DEBUG** to avoid floods. A simple token bucket per category works; or batch INFO lines every N ms with a cap.

Sketch:
```rust
use tokio_stream::StreamExt;
use std::time::Duration;

let mut rx = ipc.events();
let mut s = tokio_stream::wrappers::BroadcastStream::new(rx);
while let Some(Ok(ev)) = s.next().await {
  if let Some(pb::ipc_event::Payload::Log(log)) = ev.payload {
    match log.level() {
      pb::log_event::Level::Warn | pb::log_event::Level::Error => {
        tracing::warn!(target: "unity", msg = %log.message);
      }
      _ => { /* throttle or batch */ }
    }
  }
}
```

### 3.2 Operation progress surface
Maintain a map `op_id → last_state`. Expose a query API if MCP tools need it, and/or relay updates to MCP clients.

---

## 4) Unity — Event emission

### 4.1 Log capture (Editor)
Register for `Application.logMessageReceivedThreaded` to capture logs off the main thread. Convert Unity types to `LogEvent.Level`.

```csharp
// Editor/Ipc/EditorLogBridge.cs
using UnityEngine;
using Pb = Mcp.Unity.V1;

[InitializeOnLoad]
internal static class EditorLogBridge
{
    static EditorLogBridge()
    {
        Application.logMessageReceivedThreaded += OnLog;
    }

    private static void OnLog(string condition, string stackTrace, LogType type)
    {
        var lvl = type switch {
            LogType.Error or LogType.Exception => Pb.LogEvent.Types.Level.Error,
            LogType.Assert or LogType.Warning => Pb.LogEvent.Types.Level.Warn,
            LogType.Log => Pb.LogEvent.Types.Level.Info,
            _ => Pb.LogEvent.Types.Level.Debug,
        };
        var ev = new Pb.IpcEvent {
            MonotonicTsNs = NowNs(),
            Log = new Pb.LogEvent {
                MonotonicTsNs = NowNs(),
                Level = lvl,
                Message = condition ?? string.Empty,
                Category = "Unity",
                StackTrace = stackTrace ?? string.Empty,
            }
        };
        IpcEventSender.TryEnqueue(ev); // thread-safe queue (see 4.3)
    }

    private static long NowNs() => (long)(System.Diagnostics.Stopwatch.GetTimestamp() * (1e9 / System.Diagnostics.Stopwatch.Frequency));
}
```

### 4.2 Operation tracker helper
Provide a simple API to wrap long tasks and emit `START/PROGRESS/COMPLETE`.

```csharp
// Editor/Ipc/OperationTracker.cs
using Pb = Mcp.Unity.V1;
using System;

internal static class OperationTracker
{
    public static string Start(string kind, string message)
    {
        string id = Guid.NewGuid().ToString("n");
        Publish(new Pb.OperationEvent { OpId = id, Kind = Pb.OperationEvent.Types.Kind.Start, Message = message });
        return id;
    }
    public static void Progress(string id, int pct, string msg = "")
        => Publish(new Pb.OperationEvent { OpId = id, Kind = Pb.OperationEvent.Types.Kind.Progress, Progress = pct, Message = msg });
    public static void Complete(string id, int code, string msg = "")
        => Publish(new Pb.OperationEvent { OpId = id, Kind = Pb.OperationEvent.Types.Kind.Complete, Progress = 100, Code = code, Message = msg });

    private static void Publish(Pb.OperationEvent op)
        => IpcEventSender.TryEnqueue(new Pb.IpcEvent { MonotonicTsNs = EditorLogBridge_NowNs(), Op = op });
}
```

> Use this in build/import code paths. Ensure calls that touch Unity API are run on the main thread.

### 4.3 Event sender (thread‑safe queue + writer)
Buffer events in a lock‑free queue and flush on a background task to the active connection. Apply throttling: drop INFO when queue length exceeds a threshold; always send WARN/ERROR and operation events.

```csharp
// Editor/Ipc/IpcEventSender.cs
using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;
using Pb = Mcp.Unity.V1;

internal static class IpcEventSender
{
    private static readonly ConcurrentQueue<Pb.IpcEvent> Q = new();
    private static int _started;

    public static void TryEnqueue(Pb.IpcEvent ev)
    {
        // Simple drop policy for low-importance events when congested
        if (Q.Count > 5000 && ev.PayloadCase == Pb.IpcEvent.PayloadOneofCase.Log && ev.Log.Level < Pb.LogEvent.Types.Level.Warn)
            return;
        Q.Enqueue(ev);
        if (Interlocked.Exchange(ref _started, 1) == 0) _ = PumpAsync();
    }

    private static async Task PumpAsync()
    {
        // Assume EditorIpcServer manages a per-connection Stream reference
        while (EditorIpcServer.TryGetActiveStream(out var s))
        {
            while (Q.TryDequeue(out var ev))
            {
                var env = new Pb.IpcEnvelope { Event = ev };
                var bytes = EnvelopeCodec.Encode(env);
                await Framing.WriteFrameAsync(s, bytes);
            }
            await Task.Delay(10); // light pacing
        }
        Interlocked.Exchange(ref _started, 0);
    }
}
```

> `EditorIpcServer` should expose `TryGetActiveStream(out Stream)` or an event to notify connect/disconnect.

---

## 5) Unity — Multi‑connection (optional, nice‑to‑have)
Maintain a list of active clients (rare in Editor), and broadcast events to all. Replace `TryGetActiveStream` with a thread‑safe `List<Stream>` under a lock; iterate and remove on write errors.

---

## 6) Testing

### 6.1 Manual
- Start Unity; confirm logs flow to Rust (`INFO` visible at low volume; `WARN/ERROR` always visible).
- Trigger a synthetic long operation using `OperationTracker` and watch updates in Rust.
- Kill Unity mid‑stream; verify Rust auto‑reconnects and resumes receiving events after Unity restarts.

### 6.2 Automated (Rust only)
- Replace Unity with a stub server that streams `IpcEvent{log}` and `IpcEvent{op}` at high rate; assert throttling doesn’t block requests and the client remains responsive.
- Simulate connection drops; assert reconnect and pending‑request failure semantics.

### 6.3 Performance
- Flood test 20k INFO logs; ensure WARN/ERROR are not delayed; measure enqueue drop rate and CPU.

---

## 7) Security & Robustness
- Never include secrets in logs; redact tokens/paths.
- Enforce a max frame size on both sides (e.g., 64 MB) and reject oversize.
- Ensure event sender does not run on the Unity main thread.
- Unity shutdown: flush best‑effort, then close the stream; Rust should treat EOF as a reconnect signal.

---

## 8) Definition of Done (Step 3)
- Unity emits `LogEvent` and `OperationEvent` via IPC with backpressure safeguards.
- Rust subscribes to events, throttles low‑importance logs, and exposes operation status.
- `IpcClient` reconnects automatically and resumes event intake after editor restarts.
- Request/response path remains functional (Health still passes).

---

## 9) What’s next (Step 4 preview)
- Convert the first real tool (e.g., **Assets** or **Build**) to IPC request/response + `OperationEvent` progress.
- Add a simple UI/CLI in Rust to display operations and tail logs.

