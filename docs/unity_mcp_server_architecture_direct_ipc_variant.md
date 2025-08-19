# Unity MCP Server — Architecture (Direct IPC Variant)

> **Revision scope**: Remove the external BridgeServer process. The MCP Server (Rust) talks **directly** to the Unity Editor via **local IPC** while retaining **Proto as the single source of truth (SSoT)** for all message types.

---

## 1) High‑Level Architecture

```mermaid
flowchart LR
  subgraph Client[LLM Client / IDE / Agent]
    M[MCP Client]
  end

  subgraph Server[MCP Server (Rust)]
    R[rmcp runtime\n(JSON-RPC over stdio/SSE)]
    I[IPC Client\n(Protobuf over Pipes/UDS)]
  end

  subgraph Unity[Unity Editor (UPM package)]
    UIPC[EditorIpcServer\n(Named Pipe / UDS)]
    D[Dispatcher\n(Main Thread)]
    UA[Unity API\n(AssetDatabase, BuildPipeline, ...)]
  end

  M -- MCP (stdio/SSE) --> R
  R --> I
  I -- IPC (length‑prefixed Protobuf) --> UIPC
  UIPC --> D --> UA
  UA --> UIPC
  UIPC -- events/logs --> I --> R --> M
```

**Roles**
- **Rust MCP Server**: Presents MCP tools, validates inputs, speaks **IPC client** to Unity, aggregates results and streams, maps errors.
- **Unity Package**: Provides **IPC server**, executes Unity API on the main thread via a dispatcher, publishes logs and operation updates as push streams.

---

## 2) Naming & Directory Layout

```
repo-root/
├─ server/                       # Rust MCP Server (rmcp + IPC client)
│  ├─ build.rs                   # prost-build (message types only)
│  ├─ Cargo.toml
│  └─ src/
│     ├─ generated/              # prost output (messages only)
│     ├─ ipc/{ client.rs, framing.rs, codec.rs }
│     └─ tools/...               # MCP tools calling IPC
├─ bridge/                       # Unity project (Editor package lives here)
│  └─ Packages/com.example.mcp-editor-bridge/
│     ├─ Editor/
│     │  ├─ EditorIpcServer.cs
│     │  ├─ EditorDispatcher.cs
│     │  ├─ Services/
│     │  └─ Generated/Proto/     # CI-generated C# message types
│     └─ package.json
└─ proto/
   └─ mcp/unity/v1/*.proto       # SSoT for all contracts
```

---

## 3) Contract & Code Generation Strategy (SSoT via Proto)

**Principle**: `.proto` files are the **only** source of contracts.

- **Rust (server/)**: use `prost-build` for **message types only** (no gRPC service generation). The runtime uses `prost::Message` for encode/decode over IPC.
- **Unity (UPM)**: CI generates **C# message classes** with `protoc --csharp_out` and places them under `Editor/Generated/Proto/`. Unity references **`Google.Protobuf` only** (no gRPC runtime).

**Rust build.rs (sketch)**
```rust
fn main() {
    let root = std::path::PathBuf::from("../proto");
    let files = [
        "mcp/unity/v1/common.proto",
        "mcp/unity/v1/editor_control.proto",
        "mcp/unity/v1/assets.proto",
        "mcp/unity/v1/build.proto",
        "mcp/unity/v1/operations.proto",
        "mcp/unity/v1/events.proto",
    ].into_iter().map(|p| root.join(p)).collect::<Vec<_>>();

    prost_build::Config::new()
        .out_dir("src/generated")
        .compile_protos(&files, &[root])
        .unwrap();
}
```

**Unity CI (excerpt)**
```bash
mkdir -p bridge/Packages/com.example.mcp-editor-bridge/Editor/Generated/Proto
protoc -I proto \
  --csharp_out=bridge/Packages/com.example.mcp-editor-bridge/Editor/Generated/Proto \
  proto/mcp/unity/v1/*.proto
```

---

## 4) IPC Transport & Endpoints

- **Windows**: Named Pipe `\\.\pipe\unity-mcp/<project-hash>/ipc` (ACL: current user only).
- **macOS/Linux**: Unix Domain Socket `${XDG_RUNTIME_DIR}/unity-mcp/<project-hash>/ipc.sock` (permissions `0600`).
- **Fallback**: Loopback TCP `127.0.0.1:<ephemeral>` (development only).
- **Lifecycle**: Unity hosts the **listener**; Rust connects and **reconnects with backoff**. One connection per Editor instance.

---

## 5) Framing & Multiplexing

- **Encoding**: Protobuf binary.
- **Framing**: **Length‑prefixed** (4‑byte little‑endian) followed by message bytes.
- **Multiplexing**: A single duplex stream carries all traffic via an **envelope** message (oneof).

**Envelope & Handshake (add to proto)**
```proto
message IpcEnvelope {
  uint64 seq = 1; // per-direction sequence
  oneof kind {
    IpcHello    hello    = 10;
    IpcWelcome  welcome  = 11;
    IpcRequest  request  = 12;
    IpcResponse response = 13;
    IpcEvent    event    = 14;
    IpcAck      ack      = 15; // optional backpressure
  }
}

message IpcHello   { string token = 1; string ipc_version = 2; string project_path = 3; repeated string features = 4; }
message IpcWelcome { string server_version = 1; string schema_hash = 2; repeated string features = 3; }
```

**Request/Response & Events (example)**
```proto
message IpcRequest {
  string id = 1; // client-issued correlation id
  oneof payload {
    BuildPlayerRequest    build_player = 10;
    RefreshAssetsRequest  refresh      = 11;
    // ... other API entry points
  }
}

message IpcResponse {
  string request_id = 1;
  ErrorData error   = 2; // optional
  oneof payload {
    BuildPlayerResponse    build_player = 10;
    RefreshAssetsResponse  refresh      = 11;
  }
}

message IpcEvent {
  string op_id = 1;
  uint64 seq   = 2; // in-operation sequence
  oneof payload {
    LogEvent        log        = 10;
    OperationUpdate op_update  = 11; // PENDING/RUNNING/SUCCEEDED/FAILED/CANCELLED
    Heartbeat       heartbeat  = 12;
  }
}
```

---

## 6) Execution Model (Unity main thread)

- The IPC server reads messages on a background thread and enqueues work to a **Dispatcher** bound to the **Editor main thread** (e.g., `EditorApplication.update` pump).
- Action calls return either:
  - Immediate `IpcResponse`, or
  - A lightweight `Operation{id}` then progress/logs are published via `IpcEvent`.
- Cancellations map to a `CancelOperation(op_id)` request; best‑effort interruption in Unity.

**Unity receive → dispatch (concept)**
```csharp
int len = br.ReadInt32();
var bytes = br.ReadBytes(len);
var env = IpcEnvelope.Parser.ParseFrom(bytes);
if (env.KindCase == IpcEnvelope.KindOneofCase.Request) {
    var req = env.Request;
    EditorDispatcher.Enqueue(async () => {
        var resp = await HandleAsync(req); // Execute Unity API
        var outEnv = new IpcEnvelope { Response = resp };
        var outBytes = outEnv.ToByteArray();
        bw.Write(outBytes.Length); bw.Write(outBytes); bw.Flush();
    });
}
```

---

## 7) Backpressure & Reliability

- **Logs**: coalesce INFO in 100–200 ms windows; never drop WARN/ERROR.
- **Event state**: keep the **latest `OperationStatus` per `op_id`** in Unity; upon reconnection, replay the latest state to Rust.
- **Optional acks**: `IpcAck{ seq }` to implement a simple sliding window when high throughput causes pressure.
- **Payload limits**: 2–4 MB per frame; return large artifacts by file reference.

---

## 8) Security

- Pipes/UDS created with **user‑only** permissions.
- First handshake validates a shared **token** (e.g., `MCP_BRIDGE_TOKEN`).
- IPC paths are project‑scoped with a **project hash** to avoid collisions.

---

## 9) Observability

- Structured logs on both sides; correlate by `operation_id` and `request_id`.
- Health check flow: `IpcHello/IpcWelcome` + periodic `Heartbeat` events.
- Metrics (optional): counters for requests, failures, bytes sent, reconnects.

---

## 10) Failure Modes & Mitigations

- **Editor closed**: connection drops → Rust retries with backoff; tool calls fail fast with a helpful status.
- **IPC path in use**: Unity rotates to a new suffix; publishes the active endpoint for the session.
- **Unity not responding**: IPC timeouts → mark operation `FAILED` with diagnostics; keep the server alive.
- **High log volume**: rate limit INFO; bound buffers with drop‑oldest for verbose streams.

---

## 11) Compatibility & Versioning

- **Schema evolution**: only add fields; keep field numbers stable. Unknown fields are ignored by both sides.
- **Handshake** returns `schema_hash` (hash of `proto/**/*`) and `features[]`; Rust may warn on mismatch.

---

## 12) MVP Milestones

1. **Health**: Echo request/response over IPC + handshake.
2. **Logs**: Unity pushes EditorConsole logs as `IpcEvent.LogEvent`.
3. **Operations**: Start → progress → completion (e.g., `Assets.Refresh`).
4. **Build**: Minimal `BuildPlayer` with cancel semantics.
5. **Error model**: INVALID_ARGUMENT / NOT_FOUND / INTERNAL mapping.

---

## 13) Testing Strategy

- **Rust unit tests**: framing, encode/decode, reconnect logic, backpressure windowing.
- **Unity playmode/editor tests**: Dispatcher main‑thread guarantees, asset ops no‑op/mocked.
- **End‑to‑end (local)**: launch Unity project headless, connect Rust, run a vertical slice (Health→Logs→Operation).

---

## 14) Appendix — Rust IPC helpers (length‑prefix IO)

```rust
use bytes::{BufMut, BytesMut};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn write_msg<T: Message>(io: &mut (impl AsyncWriteExt + Unpin), m: &T) -> anyhow::Result<()> {
    let mut buf = BytesMut::with_capacity(4 + m.encoded_len());
    buf.put_u32_le(m.encoded_len() as u32);
    m.encode(&mut buf)?;
    io.write_all(&buf).await?;
    Ok(())
}

pub async fn read_msg<M: Message + Default>(io: &mut (impl AsyncReadExt + Unpin)) -> anyhow::Result<M> {
    let mut len = [0u8; 4];
    io.read_exact(&mut len).await?;
    let n = u32::from_le_bytes(len) as usize;
    let mut body = vec![0u8; n];
    io.read_exact(&mut body).await?;
    Ok(M::decode(&*body)?)
}
```

---

## 15) Appendix — Unity Dispatcher sketch

```csharp
// EditorDispatcher.cs (concept)
using System;
using System.Collections.Concurrent;
using UnityEditor;

static class EditorDispatcher {
  private static readonly ConcurrentQueue<Action> q = new();
  [InitializeOnLoadMethod]
  static void Init() => EditorApplication.update += Pump;
  public static void Enqueue(Action a) => q.Enqueue(a);
  private static void Pump() { while (q.TryDequeue(out var a)) a(); }
}
```

---

## 16) Rationale (Why direct IPC?)

- **Simpler distribution**: one less process and runtime (no .NET server binary).
- **Lower latency & fewer failure points**: no gRPC hop.
- **Keeps doors open**: gRPC gateway can be added later if external access is needed; schema remains Proto‑based.

