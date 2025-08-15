# MCP–gRPC Architecture: Class & Sequence Diagrams

This document visualizes how the rmcp runtime, ToolRouter, Rust-side gRPC clients, and the shared `AppContext` fit together, and how requests traverse the stack for Health and Logs in Phase 1 (dummy relay) and later (real Events stream).

---

## Class Diagram

```mermaid
classDiagram
  direction LR

  class RmcpRuntime {
    +serve_stdio()
    +register_tools(router: ToolRouter)
  }

  class ToolRouter {
    +register(name, handler)
    +dispatch(name, params): ToolResult|Stream
  }

  class AppContext {
    +config(): &GrpcConfig
    +channel_manager(): &ChannelManager
    +logs_relay(): LogsRelayHandle
  }

  class GrpcConfig {
    +addr: String <<MCP_BRIDGE_ADDR>>
    +token: Option<String> <<MCP_BRIDGE_TOKEN>>
    +timeout_secs: u64 <<MCP_BRIDGE_TIMEOUT>>
  }

  class ChannelManager {
    -channel: TonicChannel
    +editor_control_client(): EditorControlClient
    +events_client(): EventsClient
    +with_timeout(dur): ChannelManager
    +with_metadata(token): ChannelManager
  }

  class TonicChannel {
    <<tonic::transport::Channel>>
  }

  class EditorControlClient {
    +health(HealthRequest): HealthResponse
    +get_play_mode(Empty): PlayMode
    +set_play_mode(SetPlayModeReq): SetPlayModeRes
  }

  class EventsClient {
    +subscribe_operation(OperationRef): Stream<OperationEvent>
  }

  class LogsRelay {
    -tx: broadcast::Sender<LogEvent>
    +spawn(cm: ChannelManager): LogsRelay
    +subscribe(): broadcast::Receiver<LogEvent>
  }

  class EditorTool {
    <<MCP Tool: unity.editor.health>>
    +handle(ctx: AppContext): Json
  }

  class EventsTool {
    <<MCP Tool: unity.events.subscribe_logs>>
    +stream(ctx: AppContext): Json Stream
  }

  class UnityBridgeServer {
    <<C# gRPC Server>>
    +EditorControl
    +Events
    +Assets/Build/Operations/...
  }

  class EditorControlService { <<C# service>> }
  class EventsService { <<C# service>> }

  %% Relationships
  RmcpRuntime --> ToolRouter : uses
  ToolRouter ..> EditorTool : dispatches
  ToolRouter ..> EventsTool : dispatches

  AppContext *-- GrpcConfig
  AppContext *-- ChannelManager
  AppContext o-- LogsRelay

  ChannelManager *-- TonicChannel
  ChannelManager ..> EditorControlClient : creates
  ChannelManager ..> EventsClient : creates

  EditorTool ..> AppContext : DI
  EditorTool ..> EditorControlClient : via ChannelManager
  EventsTool ..> AppContext : DI
  EventsTool ..> LogsRelay : subscribe

  UnityBridgeServer *-- EditorControlService
  UnityBridgeServer *-- EventsService

  EditorControlClient --> UnityBridgeServer : gRPC calls
  EventsClient --> UnityBridgeServer : streaming gRPC
```

**Key Responsibilities**

- **rmcp / ToolRouter**: transport + dispatch; normalize errors to MCP codes.
- **AppContext**: shared dependency container (config, client factory, relay handle).
- **ChannelManager**: builds typed clients; injects auth (`Authorization: Bearer …`) and per-call deadlines.
- **LogsRelay (Phase 1)**: dummy heartbeat broadcaster; later swapped for real Events stream.

---

## Sequence Diagram — Health (unity.editor.health)

```mermaid
sequenceDiagram
  autonumber
  participant C as MCP Client
  participant R as RmcpRuntime
  participant TR as ToolRouter
  participant T as EditorTool
  participant AC as AppContext
  participant CM as ChannelManager
  participant EC as EditorControlClient
  participant U as UnityBridge:EditorControlService

  C->>R: JSON-RPC call "unity.editor.health"
  R->>TR: dispatch(name, params)
  TR->>T: handle(ctx, {})
  T->>AC: config() / editor_client_with_timeout(5s)
  AC->>CM: get client (w/ token, deadline)
  CM->>EC: build client (tonic stub)
  T->>EC: Health(HealthRequest{})
  EC->>U: gRPC Health
  U-->>EC: HealthResponse{ ready, version, status }
  EC-->>T: response
  T-->>TR: MCP JSON { ready, version, status, bridge_addr, observed_at, latency_ms }
  TR-->>R: ToolResult
  R-->>C: JSON-RPC result

  rect rgb(255,240,240)
  note over T,U: Error mapping
  U-->>EC: gRPC Status(DeadlineExceeded)
  EC-->>T: error
  T-->>TR: MCP error { code: timeout_error }
  end
```

**Notes**

- Per-call timeout is enforced (5s) via `ChannelManager`.
- Error mapping (gRPC → MCP): `Unavailable→service_unavailable`, `DeadlineExceeded→timeout_error`, others → `internal_error`.

---

## Sequence Diagram — Logs (unity.events.subscribe\_logs)

```mermaid
sequenceDiagram
  autonumber
  participant C as MCP Client
  participant R as RmcpRuntime
  participant TR as ToolRouter
  participant T as EventsTool
  participant AC as AppContext
  participant LR as LogsRelay
  participant CM as ChannelManager
  participant EV as EventsClient
  participant U as UnityBridge:EventsService

  C->>R: JSON-RPC call "unity.events.subscribe_logs"
  R->>TR: dispatch(name, params)
  TR->>T: stream(ctx)

  alt Phase 1 (Dummy Heartbeat)
    T->>AC: logs_relay()
    AC->>LR: subscribe()
    loop every 2s
      LR-->>T: { kind:"heartbeat", message:"bridge: idle", ts, seq }
      T-->>R: stream next JSON event
      R-->>C: send next item
    end
  else Future (Real Events Stream)
    T->>AC: channel_manager()
    AC->>CM: events_client( with timeout=30s first connect )
    CM->>EV: build client
    T->>EV: SubscribeOperation(op_id or logs_ref)
    EV->>U: gRPC stream
    loop while stream open
      U-->>EV: OperationEvent / LogEvent
      EV-->>T: event
      T-->>R: stream next JSON event
      R-->>C: send next item
    end
  end
```

**Notes**

- Phase 1 uses a server-internal broadcast relay; later we switch to the Unity `Events` gRPC stream without changing the MCP tool’s external shape.
- The relay should be bounded (`broadcast`) and drop-oldest under pressure.

---

## Error Mapping & Config Keys (for reference)

- **gRPC → MCP**: `Unavailable→service_unavailable`, `DeadlineExceeded→timeout_error`, else `internal_error`.
- **Env keys**: `MCP_BRIDGE_ADDR`, `MCP_BRIDGE_TOKEN`, `MCP_BRIDGE_TIMEOUT` (seconds).

