use bytes::Bytes;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net,
    sync::{Mutex, broadcast, mpsc, oneshot},
    time,
};

// Trait for stream types that can be used with IPC
trait IpcStream: AsyncRead + AsyncWrite + Unpin + Send {}

// Implement for common stream types
impl<T> IpcStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

use super::{
    codec,
    features::{FeatureFlag, FeatureSet},
    framing,
    path::{Endpoint, IpcConfig, default_endpoint, parse_endpoint},
};
use crate::generated::mcp::unity::v1 as pb;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("connect timeout")]
    ConnectTimeout,
    #[error("handshake failed: {0}")]
    Handshake(String),
    #[error("authentication failed: {0}")]
    Authentication(String),
    #[error("version incompatible: {0}")]
    VersionIncompatible(String),
    #[error("schema mismatch: {0}")]
    SchemaMismatch(String),
    #[error("server unavailable: {0}")]
    ServerUnavailable(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),
    #[error("failed precondition: {0}")]
    FailedPrecondition(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("codec: {0}")]
    Codec(#[from] super::codec::CodecError),
    #[error("request timeout")]
    RequestTimeout,
    #[error("closed")]
    Closed,
}

#[derive(Clone, Debug)]
pub struct IpcClient {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    cfg: IpcConfig,
    corr: AtomicU64,
    #[allow(dead_code)] // Used in spawn_io but not visible to derive
    pending: Mutex<HashMap<String, oneshot::Sender<pb::IpcResponse>>>,
    events_tx: broadcast::Sender<pb::IpcEvent>,
    // Write side: we use an mpsc channel to serialize outgoing frames
    tx: Mutex<mpsc::Sender<Bytes>>,
    negotiated_features: Mutex<FeatureSet>,
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
            tx: Mutex::new(writer_tx),
            negotiated_features: Mutex::new(FeatureSet::new()),
        });

        // Spawn reconnection supervisor task
        Self::spawn_supervisor(inner.clone(), endpoint, writer_rx).await?;
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
        // Clone sender under lock so we don't hold the mutex across await
        let tx_clone = { self.inner.tx.lock().await.clone() };
        tx_clone.send(bytes).await.map_err(|_| IpcError::Closed)?;

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

    pub async fn has_feature(&self, feature: FeatureFlag) -> bool {
        let features = self.inner.negotiated_features.lock().await;
        features.contains(&feature)
    }

    pub async fn get_negotiated_features(&self) -> FeatureSet {
        self.inner.negotiated_features.lock().await.clone()
    }

    pub async fn assets_import(
        &self,
        paths: Vec<String>,
        recursive: bool,
        auto_refresh: bool,
        timeout: Duration,
    ) -> Result<pb::ImportAssetResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "assets.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::Import(
                    pb::ImportAssetRequest {
                        paths,
                        recursive,
                        auto_refresh,
                    },
                )),
            })),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse {
                status_code: 0,
                payload: Some(pb::assets_response::Payload::Import(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!(
                "assets import failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn assets_move(
        &self,
        from_path: String,
        to_path: String,
        timeout: Duration,
    ) -> Result<pb::MoveAssetResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "assets.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::Move(pb::MoveAssetRequest {
                    from_path,
                    to_path,
                })),
            })),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse {
                status_code: 0,
                payload: Some(pb::assets_response::Payload::Move(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!(
                "assets move failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn assets_delete(
        &self,
        paths: Vec<String>,
        soft: bool,
        timeout: Duration,
    ) -> Result<pb::DeleteAssetResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "assets.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::Delete(
                    pb::DeleteAssetRequest { paths, soft },
                )),
            })),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse {
                status_code: 0,
                payload: Some(pb::assets_response::Payload::Delete(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!(
                "assets delete failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn assets_refresh(
        &self,
        force: bool,
        timeout: Duration,
    ) -> Result<pb::RefreshResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "assets.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::Refresh(pb::RefreshRequest {
                    force,
                })),
            })),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse {
                status_code: 0,
                payload: Some(pb::assets_response::Payload::Refresh(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!(
                "assets refresh failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn assets_guid_to_path(
        &self,
        guids: Vec<String>,
        timeout: Duration,
    ) -> Result<pb::GuidToPathResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "assets.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::G2p(pb::GuidToPathRequest {
                    guids,
                })),
            })),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse {
                status_code: 0,
                payload: Some(pb::assets_response::Payload::G2p(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!(
                "assets g2p failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn assets_path_to_guid(
        &self,
        paths: Vec<String>,
        timeout: Duration,
    ) -> Result<pb::PathToGuidResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "assets.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::P2g(pb::PathToGuidRequest {
                    paths,
                })),
            })),
        };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse {
                status_code: 0,
                payload: Some(pb::assets_response::Payload::P2g(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!(
                "assets p2g failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn prefab_create(
        &self,
        game_object_path: String,
        prefab_path: String,
        timeout: Duration,
    ) -> Result<pb::CreatePrefabResponse, IpcError> {
        if !self.has_feature(FeatureFlag::PrefabsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "prefabs.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Prefab(pb::PrefabRequest {
                payload: Some(pb::prefab_request::Payload::Create(
                    pb::CreatePrefabRequest {
                        game_object_path,
                        prefab_path,
                    },
                )),
            })),
        };

        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Prefab(pb::PrefabResponse {
                status_code: 0,
                payload: Some(pb::prefab_response::Payload::Create(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Prefab(res)) => Err(IpcError::Handshake(format!(
                "prefab create failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn prefab_update(
        &self,
        game_object_path: String,
        prefab_path: String,
        timeout: Duration,
    ) -> Result<pb::UpdatePrefabResponse, IpcError> {
        if !self.has_feature(FeatureFlag::PrefabsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "prefabs.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Prefab(pb::PrefabRequest {
                payload: Some(pb::prefab_request::Payload::Update(
                    pb::UpdatePrefabRequest {
                        game_object_path,
                        prefab_path,
                    },
                )),
            })),
        };

        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Prefab(pb::PrefabResponse {
                status_code: 0,
                payload: Some(pb::prefab_response::Payload::Update(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Prefab(res)) => Err(IpcError::Handshake(format!(
                "prefab update failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn prefab_apply_overrides(
        &self,
        instance_path: String,
        timeout: Duration,
    ) -> Result<pb::ApplyPrefabOverridesResponse, IpcError> {
        if !self.has_feature(FeatureFlag::PrefabsBasic).await {
            return Err(IpcError::UnsupportedFeature(
                "prefabs.basic feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Prefab(pb::PrefabRequest {
                payload: Some(pb::prefab_request::Payload::ApplyOverrides(
                    pb::ApplyPrefabOverridesRequest { instance_path },
                )),
            })),
        };

        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Prefab(pb::PrefabResponse {
                status_code: 0,
                payload: Some(pb::prefab_response::Payload::ApplyOverrides(r)),
                ..
            })) => Ok(r),
            Some(pb::ipc_response::Payload::Prefab(res)) => Err(IpcError::Handshake(format!(
                "prefab apply overrides failed: {}",
                res.message
            ))),
            _ => Err(IpcError::Handshake("unexpected response".into())),
        }
    }

    pub async fn build_player(
        &self,
        req: pb::BuildPlayerRequest,
        timeout: Duration,
    ) -> Result<pb::BuildPlayerResponse, IpcError> {
        // Check if build.min feature is negotiated
        if !self.has_feature(FeatureFlag::BuildMin).await {
            return Err(IpcError::UnsupportedFeature(
                "build.min feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Build(pb::BuildRequest {
                payload: Some(pb::build_request::Payload::Player(req)),
            })),
        };

        let resp = self.request(req, timeout).await?;

        match resp.payload {
            Some(pb::ipc_response::Payload::Build(pb::BuildResponse {
                payload: Some(pb::build_response::Payload::Player(r)),
            })) => Ok(r),
            _ => Err(IpcError::Handshake("unexpected build response".into())),
        }
    }

    pub async fn build_bundles(
        &self,
        req: pb::BuildAssetBundlesRequest,
        timeout: Duration,
    ) -> Result<pb::BuildAssetBundlesResponse, IpcError> {
        // Check if build.min feature is negotiated
        if !self.has_feature(FeatureFlag::BuildMin).await {
            return Err(IpcError::UnsupportedFeature(
                "build.min feature not negotiated".into(),
            ));
        }

        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Build(pb::BuildRequest {
                payload: Some(pb::build_request::Payload::Bundles(req)),
            })),
        };

        let resp = self.request(req, timeout).await?;

        match resp.payload {
            Some(pb::ipc_response::Payload::Build(pb::BuildResponse {
                payload: Some(pb::build_response::Payload::Bundles(r)),
            })) => Ok(r),
            _ => Err(IpcError::Handshake("unexpected build response".into())),
        }
    }

    async fn spawn_supervisor(
        inner: Arc<Inner>,
        endpoint: Endpoint,
        writer_rx: mpsc::Receiver<Bytes>,
    ) -> Result<(), IpcError> {
        // Initial connection attempt
        Self::spawn_io(inner.clone(), endpoint.clone(), writer_rx).await?;

        // Spawn supervisor task for reconnection
        let inner_clone = inner.clone();
        tokio::spawn(async move {
            let mut backoff_ms = 200u64;
            const MAX_BACKOFF_MS: u64 = 5000;

            loop {
                // Wait for connection to be lost (indicated by writer channel closure)
                tokio::time::sleep(Duration::from_millis(1000)).await;

                // Check if writer channel is closed (connection likely lost)
                let is_closed = { inner_clone.tx.lock().await.is_closed() };
                if is_closed {
                    tracing::warn!("IPC connection lost, attempting reconnect...");

                    // Clear all pending requests
                    {
                        let mut pending = inner_clone.pending.lock().await;
                        for (_, tx) in pending.drain() {
                            let _ = tx.send(pb::IpcResponse {
                                correlation_id: String::new(),
                                payload: None,
                            });
                        }
                    }

                    // Reconnection loop with exponential backoff
                    loop {
                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                        // Create new writer channel for reconnection
                        let (new_writer_tx, new_writer_rx) = mpsc::channel::<Bytes>(1024);

                        match Self::spawn_io(inner_clone.clone(), endpoint.clone(), new_writer_rx)
                            .await
                        {
                            Ok(()) => {
                                tracing::info!("IPC reconnection successful");
                                // Reset backoff on successful connection
                                backoff_ms = 200;

                                // Update the writer channel in inner so future sends go to the new connection
                                {
                                    let mut guard = inner_clone.tx.lock().await;
                                    *guard = new_writer_tx;
                                }
                                break;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "IPC reconnection failed: {}, retrying in {}ms",
                                    e,
                                    backoff_ms
                                );
                                // Exponential backoff with jitter
                                backoff_ms = std::cmp::min(backoff_ms * 2, MAX_BACKOFF_MS);
                                let jitter = rand::random::<u64>() % (backoff_ms / 4);
                                backoff_ms += jitter;
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn spawn_io(
        inner: Arc<Inner>,
        endpoint: Endpoint,
        mut writer_rx: mpsc::Receiver<Bytes>,
    ) -> Result<(), IpcError> {
        // 1) connect
        let io = connect_endpoint(&endpoint, inner.cfg.connect_timeout).await?;
        let mut framed = framing::into_framed(io);

        // 2) T01 handshake
        let desired_features = FeatureSet::supported_by_client();
        let hello = pb::IpcHello {
            token: inner.cfg.token.clone().unwrap_or_default(),
            ipc_version: "1.0".to_string(),
            features: desired_features.to_strings(),
            schema_hash: codec::schema_hash(),
            client_name: "unity-mcp-rs".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            meta: create_default_meta(),
        };
        let control = pb::IpcControl {
            kind: Some(pb::ipc_control::Kind::Hello(hello)),
        };
        let hello_bytes = codec::encode_control(&control)?;
        use futures::{SinkExt, StreamExt};
        framed.send(hello_bytes).await.map_err(IpcError::Io)?;

        // 3) Read welcome/reject response with timeout
        let welcome = time::timeout(Duration::from_secs(2), async {
            if let Some(frame) = framed.next().await {
                let bytes = frame.map_err(IpcError::Io)?;
                let control = codec::decode_control(bytes.freeze())?;
                Self::handle_handshake_response(control).await
            } else {
                Err(IpcError::Handshake("no welcome response".into()))
            }
        })
        .await
        .map_err(|_| IpcError::ConnectTimeout)??;

        // Process negotiated features
        let negotiated = FeatureSet::from_strings(&welcome.accepted_features);
        {
            let mut features = inner.negotiated_features.lock().await;
            *features = negotiated.clone();
        }

        // 4) Log successful handshake
        tracing::info!(
            "T01 Handshake OK: version={}, features={:?}, session={}, server={} {}",
            welcome.ipc_version,
            negotiated.to_strings(),
            welcome.session_id,
            welcome.server_name,
            welcome.server_version
        );

        // 5) spawn writer and reader
        let (writer, reader) = framed.split();
        tokio::spawn(async move {
            let mut writer = writer;
            while let Some(bytes) = writer_rx.recv().await {
                if let Err(_e) = writer.send(bytes).await {
                    break;
                }
            }
        });

        // 6) spawn reader (responses/events)
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

    async fn handle_handshake_response(
        control: pb::IpcControl,
    ) -> Result<pb::IpcWelcome, IpcError> {
        match control.kind {
            Some(pb::ipc_control::Kind::Welcome(w)) => Ok(w),
            Some(pb::ipc_control::Kind::Reject(r)) => {
                use pb::ipc_reject::Code;

                match r.code() {
                    Code::Unauthenticated => Err(IpcError::Authentication(r.message)),
                    Code::OutOfRange => Err(IpcError::VersionIncompatible(r.message)),
                    Code::FailedPrecondition => {
                        if r.message.contains("schema") {
                            Err(IpcError::SchemaMismatch(r.message))
                        } else {
                            Err(IpcError::FailedPrecondition(r.message))
                        }
                    }
                    Code::Unavailable => Err(IpcError::ServerUnavailable(r.message)),
                    Code::PermissionDenied => Err(IpcError::PermissionDenied(r.message)),
                    Code::Internal => {
                        Err(IpcError::Handshake(format!("server error: {}", r.message)))
                    }
                }
            }
            _ => Err(IpcError::Handshake("unexpected control response".into())),
        }
    }

    pub async fn connect_with_retry(cfg: IpcConfig) -> Result<Self, IpcError> {
        let mut backoff_ms = 250u64;
        const MAX_BACKOFF_MS: u64 = 5000;
        let max_attempts = cfg.max_reconnect_attempts.unwrap_or(10);

        for attempt in 1..=max_attempts {
            match Self::connect(cfg.clone()).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    let should_retry = match &e {
                        IpcError::ServerUnavailable(_) => true,
                        IpcError::ConnectTimeout => true,
                        IpcError::Io(_) => true,
                        IpcError::Authentication(_) => false, // Don't retry auth errors
                        IpcError::VersionIncompatible(_) => false, // Don't retry version errors
                        IpcError::SchemaMismatch(_) => false, // Don't retry schema errors
                        IpcError::UnsupportedFeature(_) => false, // Don't retry feature errors
                        IpcError::PermissionDenied(_) => false, // Don't retry permission errors
                        IpcError::FailedPrecondition(_) => false, // Don't retry precondition errors
                        _ => false,
                    };

                    if !should_retry || attempt == max_attempts {
                        return Err(e);
                    }

                    tracing::warn!(
                        "Connection attempt {} failed: {}. Retrying in {}ms",
                        attempt,
                        e,
                        backoff_ms
                    );

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                    // Exponential backoff with jitter
                    let jitter = rand::random::<u64>() % (backoff_ms / 4);
                    backoff_ms = std::cmp::min(backoff_ms * 2, MAX_BACKOFF_MS) + jitter;
                }
            }
        }

        unreachable!()
    }
}

fn create_default_meta() -> std::collections::HashMap<String, String> {
    let mut meta = std::collections::HashMap::new();
    meta.insert("os".to_string(), std::env::consts::OS.to_string());
    meta.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    meta
}

// normalize_project_root: obsolete (field removed from T01)

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
            let stream = ClientOptions::new().open(name).map_err(IpcError::Io)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::path::IpcConfig;

    #[test]
    fn test_next_cid_generates_unique_ids() {
        let cfg = IpcConfig::default();
        let inner = Arc::new(Inner {
            cfg,
            corr: AtomicU64::new(100),
            pending: Mutex::new(HashMap::new()),
            events_tx: broadcast::channel(1).0,
            tx: Mutex::new(mpsc::channel(1).0),
            negotiated_features: Mutex::new(FeatureSet::new()),
        });
        let client = IpcClient { inner };

        let cid1 = client.next_cid();
        let cid2 = client.next_cid();
        let cid3 = client.next_cid();

        assert_ne!(cid1, cid2);
        assert_ne!(cid2, cid3);
        assert_ne!(cid1, cid3);
    }

    #[test]
    fn test_next_cid_format() {
        let cfg = IpcConfig::default();
        let inner = Arc::new(Inner {
            cfg,
            corr: AtomicU64::new(0x123456789abcdef0),
            pending: Mutex::new(HashMap::new()),
            events_tx: broadcast::channel(1).0,
            tx: Mutex::new(mpsc::channel(1).0),
            negotiated_features: Mutex::new(FeatureSet::new()),
        });
        let client = IpcClient { inner };

        let cid = client.next_cid();
        assert_eq!(cid, "123456789abcdef0");
    }

    #[test]
    fn test_ipc_error_display() {
        let err = IpcError::ConnectTimeout;
        assert_eq!(err.to_string(), "connect timeout");

        let err = IpcError::Handshake("test error".to_string());
        assert_eq!(err.to_string(), "handshake failed: test error");

        let err = IpcError::RequestTimeout;
        assert_eq!(err.to_string(), "request timeout");

        let err = IpcError::Closed;
        assert_eq!(err.to_string(), "closed");
    }

    #[test]
    fn test_events_channel() {
        let cfg = IpcConfig::default();
        let inner = Arc::new(Inner {
            cfg,
            corr: AtomicU64::new(0),
            pending: Mutex::new(HashMap::new()),
            events_tx: broadcast::channel(1).0,
            tx: Mutex::new(mpsc::channel(1).0),
            negotiated_features: Mutex::new(FeatureSet::new()),
        });
        let client = IpcClient { inner };

        // Should be able to get event receiver
        let _rx = client.events();
        let _rx2 = client.events(); // Multiple receivers should work
    }
}
