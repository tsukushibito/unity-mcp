# Step 1 — Introduce the IPC layer (path/framing/codec/client)

**Objective:** Add a production-ready IPC layer to the Rust MCP server using Tokio I/O + length-delimited framing + Protobuf messages. After this step, the server can connect to the Unity-side IPC server, complete a handshake, send a `HealthRequest`, and receive a `HealthResponse` via the `IpcClient` (typed convenience method).

> This step assumes Step 0 is completed: message-only Protobuf generation is in place and gRPC codepaths are feature-gated or removed. It also assumes the envelope messages (e.g., `IpcEnvelope`, `IpcRequest`, `IpcResponse`, `IpcEvent`, `IpcHello`, `IpcWelcome`) have been added to your `.proto` set and are generated into `crate::generated::mcp::unity::v1` (aliased below as `pb`).

---

## 0) Dependencies to add (commands)
Run in `server/` crate. We avoid pinning versions; rely on `Cargo.lock`.

```bash
# Error type and random correlation IDs
cargo add thiserror
cargo add rand

# (Optional) Streaming helpers — only if you plan to use Stream utilities
cargo add tokio-stream
```

> All other core deps (`tokio`, `tokio-util[codec]`, `bytes`, `prost`, `prost-build`) were added in Step 0.

---

## 1) Directory layout
Create the following modules under `server/src/ipc/`:

```
server/
  src/
    ipc/
      mod.rs
      path.rs       # Endpoint resolution (UDS/NamedPipe/TCP) and defaults
      framing.rs    # Length-delimited codec glue
      codec.rs      # Protobuf encode/decode helpers for IpcEnvelope
      client.rs     # IpcClient (connect, handshake, request/response, events)
```

`server/src/ipc/mod.rs`:
```rust
pub mod path;
pub mod framing;
pub mod codec;
pub mod client;
```

---

## 2) `path.rs` — endpoint resolution & defaults
Goal: accept `MCP_IPC_ENDPOINT` like `unix:///path/to/ipc.sock`, `pipe://unity-mcp/default`, or `tcp://127.0.0.1:7777`. Provide a sane OS-specific default when env is absent.

```rust
// server/src/ipc/path.rs
use std::{env, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub enum Endpoint {
    #[cfg(unix)] Unix(PathBuf),
    #[cfg(windows)] Pipe(String),
    Tcp(String), // host:port (dev fallback)
}

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub endpoint: Option<String>, // raw string like "unix:///...", "pipe://...", "tcp://host:port"
    pub token: Option<String>,
    pub connect_timeout: Duration,
    pub call_timeout: Duration,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            endpoint: env::var("MCP_IPC_ENDPOINT").ok(),
            token: env::var("MCP_IPC_TOKEN").ok(),
            connect_timeout: Duration::from_millis(
                env::var("MCP_IPC_CONNECT_TIMEOUT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(2000),
            ),
            call_timeout: Duration::from_millis(
                env::var("MCP_IPC_CALL_TIMEOUT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(4000),
            ),
        }
    }
}

pub fn default_endpoint() -> Endpoint {
    if let Ok(raw) = env::var("MCP_IPC_ENDPOINT") { return parse_endpoint(&raw); }
    // OS-specific defaults
    cfg_if::cfg_if! {
        if #[cfg(unix)] {
            let dir = std::env::var("XDG_RUNTIME_DIR").ok()
                .map(PathBuf::from)
                .unwrap_or(std::env::temp_dir());
            Endpoint::Unix(dir.join("unity-mcp").join("ipc.sock"))
        } else if #[cfg(windows)] {
            Endpoint::Pipe(r"\\.\pipe\unity-mcp\default".to_string())
        } else {
            Endpoint::Tcp("127.0.0.1:7777".to_string())
        }
    }
}

pub fn parse_endpoint(s: &str) -> Endpoint {
    if let Some(rest) = s.strip_prefix("unix://") { return Endpoint::Unix(PathBuf::from(rest)); }
    if let Some(rest) = s.strip_prefix("pipe://") { return Endpoint::Pipe(rest.to_string()); }
    if let Some(rest) = s.strip_prefix("tcp://") { return Endpoint::Tcp(rest.to_string()); }
    // Fallback: bare strings are treated as TCP host:port
    Endpoint::Tcp(s.to_string())
}
```

> Note: This file uses `cfg_if` for clean platform switches. If not present, add `cargo add cfg-if` or inline `#[cfg]` blocks.

---

## 3) `framing.rs` — length-delimited frames
Use `tokio_util::codec::LengthDelimitedCodec` to avoid custom framing bugs.

```rust
// server/src/ipc/framing.rs
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type FramedIo<T> = Framed<T, LengthDelimitedCodec>;

pub fn codec() -> LengthDelimitedCodec {
    // Default settings: big enough for typical Protobuf payloads.
    // You can tweak max frame length if needed.
    LengthDelimitedCodec::new()
}

pub fn into_framed<T>(io: T) -> FramedIo<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    Framed::new(io, codec())
}
```

---

## 4) `codec.rs` — Protobuf encode/decode helpers
We encode/decode `pb::IpcEnvelope`. Keep it small: the LengthDelimitedCodec handles the frame length; we only provide bytes ↔ message.

```rust
// server/src/ipc/codec.rs
use bytes::Bytes;
use prost::Message;
use thiserror::Error;

use crate::generated::mcp::unity::v1 as pb;

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("encode error: {0}")] Encode(#[from] prost::EncodeError),
    #[error("decode error: {0}")] Decode(#[from] prost::DecodeError),
}

pub fn encode_envelope(env: &pb::IpcEnvelope) -> Result<Bytes, CodecError> {
    let mut buf = bytes::BytesMut::with_capacity(env.encoded_len());
    env.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode_envelope(b: Bytes) -> Result<pb::IpcEnvelope, CodecError> {
    pb::IpcEnvelope::decode(b)
}

// Optional: calculate a schema hash (TODO: wire to handshake)
pub fn schema_hash() -> String {
    // For now, a constant or build-time string. Replace with descriptor-set hash if desired.
    "schema-v1".to_string()
}
```

---

## 5) `client.rs` — the IpcClient
Responsibilities:
- Resolve endpoint + connect (UDS/Named Pipe/TCP dev fallback)
- Handshake: send `IpcHello{ipc_version, schema_hash, token?}` and wait for `IpcWelcome{ok}`
- Maintain a correlation map for in-flight requests
- Spawn a background read loop: route `Response` to waiters, broadcast `Event`
- Public `request()` + typed helpers (e.g., `health()`)

```rust
// server/src/ipc/client.rs
use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU64, Ordering}}, time::Duration};
use tokio::{io::{AsyncRead, AsyncWrite}, net, sync::{mpsc, oneshot, Mutex, broadcast}, time};
use bytes::Bytes;
use rand::Rng;
use thiserror::Error;

use crate::generated::mcp::unity::v1 as pb;
use super::{path::{IpcConfig, Endpoint, default_endpoint, parse_endpoint}, framing, codec};

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("connect timeout")] ConnectTimeout,
    #[error("handshake failed: {0}")] Handshake(String),
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("codec: {0}")] Codec(#[from] super::codec::CodecError),
    #[error("request timeout")] RequestTimeout,
    #[error("closed")] Closed,
}

#[derive(Clone)]
pub struct IpcClient {
    inner: Arc<Inner>,
}

struct Inner {
    cfg: IpcConfig,
    corr: AtomicU64,
    pending: Mutex<HashMap<String, oneshot::Sender<pb::IpcResponse>>>,
    events_tx: broadcast::Sender<pb::IpcEvent>,
    // Write side: we use an mpsc channel to serialize outgoing frames
    tx: mpsc::Sender<Bytes>,
}

impl IpcClient {
    pub async fn connect(cfg: IpcConfig) -> Result<Self, IpcError> {
        let endpoint = cfg.endpoint.as_deref().map(parse_endpoint).unwrap_or_else(default_endpoint);
        let (writer_tx, writer_rx) = mpsc::channel::<Bytes>(1024);
        let (events_tx, _events_rx) = broadcast::channel(1024);

        let inner = Arc::new(Inner {
            cfg,
            corr: AtomicU64::new(rand::thread_rng().gen()),
            pending: Mutex::new(HashMap::new()),
            events_tx,
            tx: writer_tx,
        });

        // Establish the connection and spawn reader/writer tasks
        Self::spawn_io(inner.clone(), endpoint, writer_rx).await?;
        Ok(Self { inner })
    }

    pub fn events(&self) -> broadcast::Receiver<pb::IpcEvent> {
        self.inner.events_tx.subscribe()
    }

    fn next_cid(&self) -> String {
        format!("{:016x}", self.inner.corr.fetch_add(1, Ordering::Relaxed))
    }

    pub async fn request(&self, req: pb::IpcRequest, timeout: Duration) -> Result<pb::IpcResponse, IpcError> {
        let cid = self.next_cid();
        let mut env = pb::IpcEnvelope { correlation_id: cid.clone(), kind: None };
        env.kind = Some(pb::ipc_envelope::Kind::Request(req));
        let bytes = codec::encode_envelope(&env)?;

        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().await.insert(cid.clone(), tx);
        self.inner.tx.send(bytes).await.map_err(|_| IpcError::Closed)?;

        match time::timeout(timeout, rx).await {
            Ok(Ok(resp)) => Ok(resp),
            Ok(Err(_canceled)) => Err(IpcError::Closed),
            Err(_elapsed) => {
                self.inner.pending.lock().await.remove(&cid);
                Err(IpcError::RequestTimeout)
            }
        }
    }

    pub async fn health(&self, timeout: Duration) -> Result<pb::HealthResponse, IpcError> {
        let req = pb::IpcRequest { payload: Some(pb::ipc_request::Payload::Health(pb::HealthRequest{})) };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Health(h)) => Ok(h),
            _ => Err(IpcError::Handshake("unexpected response type".into())),
        }
    }

    async fn spawn_io(inner: Arc<Inner>, endpoint: Endpoint, mut writer_rx: mpsc::Receiver<Bytes>) -> Result<(), IpcError> {
        // 1) connect
        let io = connect_endpoint(&endpoint, inner.cfg.connect_timeout).await?;
        let mut framed = framing::into_framed(io);

        // 2) handshake
        let hello = pb::IpcHello { ipc_version: 1, schema_hash: codec::schema_hash(), token: inner.cfg.token.clone() };
        let mut env = pb::IpcEnvelope { correlation_id: String::new(), kind: None };
        env.kind = Some(pb::ipc_envelope::Kind::Request(pb::IpcRequest { payload: Some(pb::ipc_request::Payload::Hello(hello)) }));
        let hello_bytes = codec::encode_envelope(&env)?;
        use futures::SinkExt;
        framed.send(hello_bytes).await.map_err(IpcError::Io)?;

        use futures::StreamExt;
        let welcome = time::timeout(inner.cfg.connect_timeout, async {
            while let Some(frame) = framed.next().await {
                let bytes = frame.map_err(IpcError::Io)?;
                let env = codec::decode_envelope(bytes)?;
                if let Some(pb::ipc_envelope::Kind::Response(resp)) = env.kind {
                    if let Some(pb::ipc_response::Payload::Welcome(w)) = resp.payload { return Ok::<_, IpcError>(w); }
                }
            }
            Err(IpcError::Handshake("no welcome".into()))
        }).await??;
        if !welcome.ok { return Err(IpcError::Handshake(welcome.error)); }

        // 3) spawn writer
        {
            let mut tx_framed = framed.clone();
            tokio::spawn(async move {
                while let Some(bytes) = writer_rx.recv().await {
                    if let Err(e) = tx_framed.send(bytes).await { let _ = e; break; }
                }
            });
        }

        // 4) spawn reader (responses/events)
        tokio::spawn(async move {
            while let Some(frame) = framed.next().await {
                let Ok(bytes) = frame else { break; };
                let Ok(env) = codec::decode_envelope(bytes) else { continue; };
                match env.kind {
                    Some(pb::ipc_envelope::Kind::Response(resp)) => {
                        let mut pending = inner.pending.lock().await;
                        if let Some(tx) = pending.remove(&resp.correlation_id) {
                            let _ = tx.send(resp);
                        }
                    }
                    Some(pb::ipc_envelope::Kind::Event(ev)) => { let _ = inner.events_tx.send(ev); }
                    _ => {}
                }
            }
            // TODO: signal Closed; consider reconnect loop if desired
        });

        Ok(())
    }
}

async fn connect_endpoint(endpoint: Endpoint, timeout: Duration) -> Result<impl AsyncRead + AsyncWrite + Unpin, IpcError> {
    use tokio::time::timeout as tokio_timeout;
    match endpoint {
        #[cfg(unix)] Endpoint::Unix(path) => {
            let fut = net::UnixStream::connect(path);
            Ok(tokio_timeout(timeout, fut).await.map_err(|_| IpcError::ConnectTimeout)??)
        }
        #[cfg(windows)] Endpoint::Pipe(name) => {
            use tokio::net::windows::named_pipe::ClientOptions;
            let fut = ClientOptions::new().open(&name);
            Ok(tokio_timeout(timeout, fut).await.map_err(|_| IpcError::ConnectTimeout)??)
        }
        Endpoint::Tcp(addr) => {
            let fut = net::TcpStream::connect(addr);
            Ok(tokio_timeout(timeout, fut).await.map_err(|_| IpcError::ConnectTimeout)??)
        }
    }
}
```

> Reconnection: this starter version connects once. In Step 3 (Logs/eventing), you may add a reconnect loop that preserves `events_tx` and fails in-flight requests.

---

## 6) Wire into the MCP service (call sites)
Replace gRPC client acquisition with an `IpcClient` instance owned by your service.

```rust
// server/src/mcp/service.rs (excerpt)
use crate::ipc::{client::IpcClient, path::IpcConfig};

pub struct McpService {
    ipc: IpcClient,
    // ... other fields
}

impl McpService {
    pub async fn new() -> anyhow::Result<Self> {
        let ipc = IpcClient::connect(IpcConfig::default()).await?;
        Ok(Self { ipc })
    }
}
```

Update the Health tool implementation:

```rust
// server/src/mcp/tools/health.rs (excerpt)
use crate::ipc::client::IpcClient;
use crate::generated::mcp::unity::v1 as pb;
use std::time::Duration;

pub async fn health(ipc: &IpcClient) -> anyhow::Result<pb::HealthResponse> {
    let resp = ipc.health(Duration::from_millis(1500)).await?;
    Ok(resp)
}
```

---

## 7) Tests
- **Unit** (no OS-specific server needed):
  - `codec.rs`: encode/decode roundtrip with a small `IpcEnvelope`.
  - `path.rs`: endpoint parsing and defaults (guard with platform `cfg`).
- **Integration** (optional now, mandatory in Step 2):
  - Spawn a minimal in-process IPC server (UDS on Unix, Pipe on Windows) that accepts a single connection, checks `IpcHello`, and replies with `IpcWelcome{ok:true}` and then a canned `HealthResponse`.

> Keep integration tests deterministic: use unique temp paths per test (e.g., append PID + random suffix), and delete UDS files after the run.

---

## 8) Error handling & timeouts
- Connect timeout (`IpcConfig.connect_timeout`), per-request timeout (`call_timeout`).
- On decode errors or peer close: drop the connection and error all in-flight requests.
- Consider exposing a `status()` method (Connected/Closed) and a `reconnect()` facility if Unity restarts often during development.

---

## 9) Definition of Done (Step 1)
- `server` builds and runs without gRPC.
- `IpcClient::connect()` completes the handshake against a stub Unity IPC server.
- `IpcClient::health()` returns `HealthResponse` via the request/response path.
- Events channel exists (even if unused yet); background read loop demultiplexes responses vs. events.

---

## 10) What’s next (Step 2 preview)
- Implement the Unity-side `EditorIpcServer` with the same framing and envelope types.
- Add a reconnect loop in `IpcClient` and wire `events()` to Logs.
- Replace temporary stubs in tools with concrete IPC requests per API (Assets, Build, Operations).

