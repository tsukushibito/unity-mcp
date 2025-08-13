use anyhow::Result;
use tonic::codegen::InterceptedService;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::transport::{Channel, Endpoint};
use tonic::{Request, Status};

use crate::generated::mcp::unity::v1::editor_control_client::EditorControlClient;
use crate::grpc::config::GrpcConfig;

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
        Ok(Self {
            channel,
            token: cfg.token.clone(),
        })
    }

    /// Clone of the underlying Channel (cheap; Channels are internally ref-counted).
    #[inline]
    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    /// Inject `Authorization: Bearer <token>` into a `Request<T>` if a token is present.
    /// If the token already starts with "Bearer ", it is used as-is.
    pub fn with_meta<T>(&self, mut req: Request<T>) -> Request<T> {
        if let Some(tok) = self
            .token
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
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

    /// Client with an interceptor that adds Authorization headers when token is configured.
    /// When no token is present, the interceptor acts as a no-op.
    pub fn editor_control_client(
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
            if tt.is_empty() {
                None
            } else {
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
        if let Some(h) = &self.header
            && let Ok(v) = MetadataValue::try_from(h.as_str())
        {
            req.metadata_mut().insert("authorization", v);
        }
        Ok(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[test]
    fn test_auth_interceptor_new_with_token() {
        let interceptor = AuthInterceptor::new(Some("test-token".to_string()));
        assert_eq!(interceptor.header, Some("Bearer test-token".to_string()));
    }

    #[test]
    fn test_auth_interceptor_new_with_bearer_token() {
        let interceptor = AuthInterceptor::new(Some("Bearer test-token".to_string()));
        assert_eq!(interceptor.header, Some("Bearer test-token".to_string()));
    }

    #[test]
    fn test_auth_interceptor_new_with_empty_token() {
        let interceptor = AuthInterceptor::new(Some("".to_string()));
        assert_eq!(interceptor.header, None);
    }

    #[test]
    fn test_auth_interceptor_new_with_none() {
        let interceptor = AuthInterceptor::new(None);
        assert_eq!(interceptor.header, None);
    }

    #[tokio::test]
    async fn test_channel_manager_with_meta_adds_auth_header() {
        let cm = ChannelManager {
            channel: tonic::transport::Channel::from_static("http://127.0.0.1:50051")
                .connect_lazy(),
            token: Some("test-token".to_string()),
        };

        let req = Request::new(());
        let req_with_meta = cm.with_meta(req);

        let auth_header = req_with_meta.metadata().get("authorization");
        assert!(auth_header.is_some());
        assert_eq!(auth_header.unwrap().to_str().unwrap(), "Bearer test-token");
    }

    #[tokio::test]
    async fn test_channel_manager_with_meta_no_token() {
        let cm = ChannelManager {
            channel: tonic::transport::Channel::from_static("http://127.0.0.1:50051")
                .connect_lazy(),
            token: None,
        };

        let req = Request::new(());
        let req_with_meta = cm.with_meta(req);

        let auth_header = req_with_meta.metadata().get("authorization");
        assert!(auth_header.is_none());
    }

    #[tokio::test]
    async fn test_channel_manager_with_meta_bearer_prefix() {
        let cm = ChannelManager {
            channel: tonic::transport::Channel::from_static("http://127.0.0.1:50051")
                .connect_lazy(),
            token: Some("Bearer existing-token".to_string()),
        };

        let req = Request::new(());
        let req_with_meta = cm.with_meta(req);

        let auth_header = req_with_meta.metadata().get("authorization");
        assert!(auth_header.is_some());
        assert_eq!(
            auth_header.unwrap().to_str().unwrap(),
            "Bearer existing-token"
        );
    }
}
