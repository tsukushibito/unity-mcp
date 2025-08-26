# Editor Bridge — Post‑Refactor Class Diagram

This page shows the target class structure **after** introducing `EditorDispatcher` and splitting IO/Main responsibilities. The diagram emphasizes: (1) one connection = one `ConnectionActor`, (2) all Unity API calls are marshaled onto the main thread via `EditorDispatcher`, and (3) all outbound traffic goes through a single per‑connection `OutboundWriter`.

```mermaid
classDiagram
    direction LR

    class EditorIpcServer {
      +listen(endpoint)
      +acceptLoop()
      +register(conn: ConnectionActor)
      +unregister(connId)
      +broadcast(evt: IpcEvent)
      -connections: Map<ConnectionId, ConnectionActor>
    }

    class ConnectionActor {
      +id: ConnectionId
      -stream: NetworkStream
      -writer: OutboundWriter
      -featureGuard: FeatureGuard
      +startReaderLoop()  %% BG thread
      +dispatchOnMain(req: IpcRequest) %% via EditorDispatcher
      +send(ctrl/resp/evt)
    }

    class OutboundWriter {
      -queue: Channel~byte[]~
      +startWriteLoop()  %% BG thread
      +send(bytes)
      +close()
    }

    class EditorDispatcher {
      <<static>>
      +RunOnMainAsync(Action): Task
      +RunOnMainAsync~T~(Func~T~): Task~T~
      +RunOnMainAsync~T~(Func~Task~T~~): Task~T~
      -Pump() %% EditorApplication.update
    }

    class EditorStateMirror {
      <<static>>
      +IsCompiling: bool
      +IsUpdating: bool
      +UnityVersion: string
      -UpdatePerFrame() %% EditorApplication.update
    }

    class MainThreadGuard {
      <<static>>
      +AssertMainThread()
    }

    class EnvelopeCodec {
      +Encode(control/response/event): byte[]
      +DecodeRequest(bytes): IpcRequest
    }

    class Framing {
      +ReadFrameAsync(stream): Task~byte[]~
      +WriteFrameAsync(stream, bytes): Task
    }

    class AssetsHandler {
      +HandleAsync(msg): IpcResponse
    }
    class BuildHandler {
      +HandleAsync(msg): IpcResponse
    }
    class HealthHandler {
      +Handle(req): IpcResponse
    }

    class FeatureGuard {
      +Accepted: Set~FeatureFlag~
      +IsEnabled(flag): bool
    }
    class FeatureFlag {
      <<enum>>
    }

    class IpcRequest
    class IpcResponse
    class IpcControl
    class IpcEvent

    %% Relationships
    EditorIpcServer "1" o-- "*" ConnectionActor : hosts
    ConnectionActor --> OutboundWriter : uses
    ConnectionActor ..> EditorDispatcher : dispatches via
    ConnectionActor ..> EnvelopeCodec : enc/dec
    OutboundWriter ..> Framing : writes via
    EditorIpcServer ..> IpcEvent : emits

    ConnectionActor ..> AssetsHandler : routes to
    ConnectionActor ..> BuildHandler : routes to
    ConnectionActor ..> HealthHandler : routes to

    AssetsHandler ..> MainThreadGuard : asserts
    BuildHandler ..> MainThreadGuard : asserts
    HealthHandler ..> MainThreadGuard : asserts

    HealthHandler ..> EditorStateMirror : reads snapshot

    AssetsHandler ..> IpcResponse
    BuildHandler ..> IpcResponse
    HealthHandler ..> IpcResponse
    ConnectionActor ..> IpcRequest
    ConnectionActor ..> IpcControl

    FeatureGuard <.. ConnectionActor : holds

    %% Notes
    note for EditorDispatcher "Runs on main thread via EditorApplication.update. All Unity API calls MUST pass through here."
    note for OutboundWriter "Single writer per connection. Serializes all outbound frames to avoid concurrent writes."
    note for HealthHandler "May use EditorStateMirror for fast reads; use Dispatcher for strict correctness."
```

## Legend
- **BG thread**: background IO / writer loops (non‑Unity API).
- **Main thread**: anything touching Unity Editor or `UnityEngine.Application` — invoked only via `EditorDispatcher`.

## Design Guarantees
- IO and Unity API are separated by an explicit boundary (`ConnectionActor.dispatchOnMain` → `EditorDispatcher`).
- Outbound writes are serialized (`OutboundWriter`), eliminating races on the stream.
- Health checks can be fast (mirror) or strict (dispatcher), chosen per call site.

