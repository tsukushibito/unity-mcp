# T2 — ChannelManager: Fixes & Final Implementation

This document corrects pitfalls in the original plan and provides a production‑ready `ChannelManager` for tonic 0.14 (L0 policy). It manages a single reusable `Channel`, supports token‑based auth via **either** per‑request metadata or an **interceptor** client, and configures **endpoint‑level** timeouts (no deprecated `Request::set_timeout`).

---

## Key Fixes

1. **Endpoint timeouts vs. connect timeouts**
   We set **both** `connect_timeout` (TCP/TLS handshake) and `timeout` (per‑request layer). This avoids hangs when the server is down.

2. **Token injection options**
   Keep your requested `with_meta(req)` for explicit per‑call control **and** expose an `editor_control_client_intercepted()` that always adds `Authorization` headers automatically. The latter returns a typed client using `InterceptedService`.

3. **Scheme normalization & TLS**
   `GrpcConfig::endpoint()` already returns a valid URI (e.g., `http://…`/`https://…`). With tonic’s `transport,tls-webpki-roots` features, `https://` endpoints are supported without extra code. (If you later need custom TLS, you can extend `GrpcConfig::endpoint()`.)

4. **One‑time proto include**
   The client constructors use the re‑exported generated modules (no duplicate includes per file).

---

## File: `server/src/grpc/channel.rs`

```rust
use anyhow::Result;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};
use tonic::codegen::InterceptedService;

use crate::config::GrpcConfig;
use crate::generated::mcp::unity::v1::editor_control::editor_control_client::EditorControlClient;

/// Manages a reusable gRPC channel and optional auth token.
pub struct ChannelManager {
    channel: Channel,
    token: Option<String>,
}

impl ChannelManager {
    /// Connect to the gRPC server using `GrpcConfig`.
    /// - Sets both `connect_timeout` and per-request `timeout` using the config value.
    pub async fn connect(cfg: &GrpcConfig) -> Result<Self> {
        let timeout = cfg.timeout();
        let endpoint: Endpoint = cfg
            .endpoint()? // already normalized (http/https)
            .connect_timeout(timeout)
            .timeout(timeout);

        let channel = endpoint.connect().await?;
        Ok(Self { channel, token: cfg.token.clone() })
    }

    /// Clone of the underlying Channel (cheap; Channels are internally ref-counted).
    #[inline]
    pub fn channel(&self) -> Channel { self.channel.clone() }

    /// Inject `Authorization: Bearer <token>` into a `Request<T>` if a token is present.
    /// If the token already starts with "Bearer ", it is used as-is.
    pub fn with_meta<T>(&self, mut req: Request<T>) -> Request<T> {
        if let Some(tok) = self.token.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            let header = if tok.to_ascii_lowercase().starts_with("bearer ") {
                tok.to_string()
            } else {
                format!("Bearer {}", tok)
            };
            if let Ok(val) = MetadataValue::try_from(header.as_str()) {
                req.metadata_mut().insert("authorization", val);
            }
        }
        req
    }

    /// Plain client (no automatic auth headers). Use together with `with_meta(...)` when needed.
    pub fn editor_control_client(&self) -> EditorControlClient<Channel> {
        EditorControlClient::new(self.channel())
    }

    /// Client with an interceptor that always adds Authorization headers (if token configured).
    /// Convenient when *all* calls should carry auth without wrapping each `Request`.
    pub fn editor_control_client_intercepted(
        &self,
    ) -> EditorControlClient<InterceptedService<Channel, AuthInterceptor>> {
        let interceptor = AuthInterceptor::new(self.token.clone());
        EditorControlClient::with_interceptor(self.channel(), interceptor)
    }
}

/// Lightweight auth interceptor; stores a prebuilt header string when possible.
#[derive(Clone, Default)]
pub struct AuthInterceptor {
    header: Option<String>,
}

impl AuthInterceptor {
    pub fn new(token: Option<String>) -> Self {
        let header = token.and_then(|t| {
            let tt = t.trim();
            if tt.is_empty() { None } else {
                Some(if tt.to_ascii_lowercase().starts_with("bearer ") {
                    tt.to_string()
                } else {
                    format!("Bearer {}", tt)
                })
            }
        });
        Self { header }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        if let Some(h) = &self.header {
            if let Ok(v) = MetadataValue::try_from(h.as_str()) {
                req.metadata_mut().insert("authorization", v);
            }
        }
        Ok(req)
    }
}
```

---

## Usage Examples

### A) 明示メタデータ（1回だけ付けたい）

```rust
use tonic::Request;
use server::grpc::channel::ChannelManager;
use server::grpc::config::GrpcConfig;
use server::generated::mcp::unity::v1::editor_control::{HealthRequest, HealthResponse};

let cfg = GrpcConfig::from_env();
let cm = ChannelManager::connect(&cfg).await?;
let mut client = cm.editor_control_client();

let req = cm.with_meta(Request::new(HealthRequest {}));
let resp = client.health(req).await?;
assert!(resp.into_inner().ready);
```

### B) 常に認証を付与（全呼び出し）

```rust
let cfg = GrpcConfig::from_env();
let cm = ChannelManager::connect(&cfg).await?;
let mut client = cm.editor_control_client_intercepted();
let resp = client.health(HealthRequest {}).await?;
```

---

## Integration Notes

* Works with the **T4 smoke test** out of the box. T4 が Request を直接渡す形（Aパターン）なら `with_meta` を使ってください。トークン不要ならそのまま `T4` のテストは通ります。
* `GrpcConfig` の `default_timeout_secs` は **接続**・**各リクエスト**の両方に適用（単一値でOK）。将来、接続と呼び出しで値を分けたくなったら `GrpcConfig` を拡張してください。

---

## Acceptance Checklist

* ✅ ビルド通過（tonic 0.14 / transport + tls-webpki-roots）。
* ✅ 接続確立・クライアント生成（EditorControl）。
* ✅ トークンメタデータ注入：`with_meta` と `intercepted` の2経路で可動。
* ✅ **リクエストレベルのタイムアウト**はエンドポイントでレイヤー設定。`Request::set_timeout` 不使用。
* ✅ 生成protoの参照は一箇所（再エクスポート経由）。