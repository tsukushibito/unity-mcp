use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net,
    sync::{broadcast, mpsc, oneshot, Mutex},
    time,
};
use bytes::Bytes;
use thiserror::Error;

// Trait for stream types that can be used with IPC
trait IpcStream: AsyncRead + AsyncWrite + Unpin + Send {}

// Implement for common stream types
impl<T> IpcStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

use crate::generated::mcp::unity::v1 as pb;
use super::{
    codec,
    framing,
    path::{default_endpoint, parse_endpoint, Endpoint, IpcConfig},
};

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("connect timeout")]
    ConnectTimeout,
    #[error("handshake failed: {0}")]
    Handshake(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("codec: {0}")]
    Codec(#[from] super::codec::CodecError),
    #[error("request timeout")]
    RequestTimeout,
    #[error("closed")]
    Closed,
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
        let endpoint = cfg
            .endpoint
            .as_deref()
            .map(parse_endpoint)
            .unwrap_or_else(default_endpoint);
        let (writer_tx, writer_rx) = mpsc::channel::<Bytes>(1024);
        let (events_tx, _events_rx) = broadcast::channel(1024);

        let inner = Arc::new(Inner {
            cfg,
            corr: AtomicU64::new(rand::random()),
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

    pub async fn request(
        &self,
        req: pb::IpcRequest,
        timeout: Duration,
    ) -> Result<pb::IpcResponse, IpcError> {
        let cid = self.next_cid();
        let mut env = pb::IpcEnvelope {
            correlation_id: cid.clone(),
            kind: None,
        };
        env.kind = Some(pb::ipc_envelope::Kind::Request(req));
        let bytes = codec::encode_envelope(&env)?;

        let (tx, rx) = oneshot::channel();
        self.inner.pending.lock().await.insert(cid.clone(), tx);
        self.inner
            .tx
            .send(bytes)
            .await
            .map_err(|_| IpcError::Closed)?;

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
        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Health(pb::HealthRequest {})),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Health(h)) => Ok(h),
            _ => Err(IpcError::Handshake("unexpected response type".into())),
        }
    }

    async fn spawn_io(
        inner: Arc<Inner>,
        endpoint: Endpoint,
        mut writer_rx: mpsc::Receiver<Bytes>,
    ) -> Result<(), IpcError> {
        // 1) connect
        let io = connect_endpoint(&endpoint, inner.cfg.connect_timeout).await?;
        let mut framed = framing::into_framed(io);

        // 2) handshake
        let hello = pb::IpcHello {
            ipc_version: 1,
            schema_hash: codec::schema_hash(),
            token: inner.cfg.token.clone().unwrap_or_default(),
        };
        let mut env = pb::IpcEnvelope {
            correlation_id: String::new(),
            kind: None,
        };
        env.kind = Some(pb::ipc_envelope::Kind::Request(pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Hello(hello)),
        }));
        let hello_bytes = codec::encode_envelope(&env)?;
        use futures::{SinkExt, StreamExt};
        framed.send(hello_bytes).await.map_err(IpcError::Io)?;
        
        // Get handshake response first
        let welcome = time::timeout(inner.cfg.connect_timeout, async {
            while let Some(frame) = framed.next().await {
                let bytes = frame.map_err(IpcError::Io)?;
                let env = codec::decode_envelope(bytes.freeze())?;
                if let Some(pb::ipc_envelope::Kind::Response(resp)) = env.kind {
                    if let Some(pb::ipc_response::Payload::Welcome(w)) = resp.payload {
                        return Ok::<_, IpcError>(w);
                    }
                }
            }
            Err(IpcError::Handshake("no welcome".into()))
        })
        .await
        .map_err(|_| IpcError::ConnectTimeout)??;
        if !welcome.ok {
            return Err(IpcError::Handshake(welcome.error));
        }

        // 3) spawn writer and reader
        let (reader, writer) = framed.split();
        tokio::spawn(async move {
            let mut writer = writer;
            while let Some(bytes) = writer_rx.recv().await {
                if let Err(_e) = writer.send(bytes).await {
                    break;
                }
            }
        });

        // 4) spawn reader (responses/events)
        tokio::spawn(async move {
            let mut reader = reader;
            while let Some(frame) = reader.next().await {
                let Ok(bytes) = frame else {
                    break;
                };
                let Ok(env) = codec::decode_envelope(bytes.freeze()) else {
                    continue;
                };
                match env.kind {
                    Some(pb::ipc_envelope::Kind::Response(resp)) => {
                        let mut pending = inner.pending.lock().await;
                        if let Some(tx) = pending.remove(&resp.correlation_id) {
                            let _ = tx.send(resp);
                        }
                    }
                    Some(pb::ipc_envelope::Kind::Event(ev)) => {
                        let _ = inner.events_tx.send(ev);
                    }
                    _ => {}
                }
            }
            // TODO: signal Closed; consider reconnect loop if desired
        });

        Ok(())
    }
}

async fn connect_endpoint(
    endpoint: &Endpoint,
    timeout: Duration,
) -> Result<Box<dyn IpcStream>, IpcError> {
    use tokio::time::timeout as tokio_timeout;
    match endpoint {
        #[cfg(unix)]
        Endpoint::Unix(path) => {
            let fut = net::UnixStream::connect(path);
            let stream = tokio_timeout(timeout, fut)
                .await
                .map_err(|_| IpcError::ConnectTimeout)??;
            Ok(Box::new(stream))
        }
        #[cfg(windows)]
        Endpoint::Pipe(name) => {
            use tokio::net::windows::named_pipe::ClientOptions;
            let fut = ClientOptions::new().open(name);
            let stream = tokio_timeout(timeout, fut)
                .await
                .map_err(|_| IpcError::ConnectTimeout)??;
            Ok(Box::new(stream))
        }
        Endpoint::Tcp(addr) => {
            let fut = net::TcpStream::connect(addr);
            let stream = tokio_timeout(timeout, fut)
                .await
                .map_err(|_| IpcError::ConnectTimeout)??;
            Ok(Box::new(stream))
        }
    }
}