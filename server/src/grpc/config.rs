use std::{env, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrpcConfig {
    pub addr: String,
    pub token: Option<String>,
    pub default_timeout_secs: u64,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            addr: "http://localhost:8080".to_string(),
            token: None,
            default_timeout_secs: 30,
        }
    }
}

impl GrpcConfig {
    pub const ENV_ADDR: &str = "MCP_BRIDGE_ADDR";
    pub const ENV_TOKEN: &str = "MCP_BRIDGE_TOKEN";
    pub const ENV_TIMEOUT: &str = "MCP_BRIDGE_TIMEOUT"; // seconds

    /// Construct from real process environment variables.
    pub fn from_env() -> Self {
        Self::from_reader(|k| env::var(k).ok())
    }

    /// Construct from an arbitrary key/value source (for tests).
    pub fn from_map<I, K, V>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        use std::collections::HashMap;
        let map: HashMap<String, String> = iter
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        Self::from_reader(|k| map.get(k).cloned())
    }

    fn from_reader<F>(get: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut cfg = Self::default();

        if let Some(addr) = get(Self::ENV_ADDR) {
            cfg.addr = normalize_addr(&addr);
        }

        if let Some(token_raw) = get(Self::ENV_TOKEN) {
            let t = token_raw.trim();
            cfg.token = if t.is_empty() { None } else { Some(t.to_string()) };
        }

        if let Some(timeout_raw) = get(Self::ENV_TIMEOUT) {
            if let Ok(secs) = timeout_raw.trim().parse::<u64>() {
                cfg.default_timeout_secs = secs;
            }
        }

        cfg
    }

    /// Convenience as `std::time::Duration`.
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.default_timeout_secs)
    }

    /// Convert to `tonic::transport::Endpoint`.
    pub fn endpoint(&self) -> Result<tonic::transport::Endpoint, tonic::transport::Error> {
        tonic::transport::Endpoint::from_shared(self.addr.clone())
    }
}

fn normalize_addr(s: &str) -> String {
    let t = s.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else {
        format!("http://{}", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_applied() {
        let cfg = GrpcConfig::from_map(std::iter::empty::<(String, String)>());
        assert_eq!(cfg.addr, "http://localhost:8080");
        assert_eq!(cfg.token, None);
        assert_eq!(cfg.default_timeout_secs, 30);
        assert_eq!(cfg.timeout(), Duration::from_secs(30));
    }

    #[test]
    fn overrides_work_and_addr_is_normalized() {
        let cfg = GrpcConfig::from_map([
            (GrpcConfig::ENV_ADDR.to_string(), "127.0.0.1:50051".to_string()),
            (GrpcConfig::ENV_TOKEN.to_string(), "abc".to_string()),
            (GrpcConfig::ENV_TIMEOUT.to_string(), "5".to_string()),
        ]);
        assert_eq!(cfg.addr, "http://127.0.0.1:50051");
        assert_eq!(cfg.token.as_deref(), Some("abc"));
        assert_eq!(cfg.default_timeout_secs, 5);
    }

    #[test]
    fn empty_token_is_none_and_bad_timeout_is_ignored() {
        let cfg = GrpcConfig::from_map([
            (GrpcConfig::ENV_TOKEN.to_string(), "   ".to_string()),
            (GrpcConfig::ENV_TIMEOUT.to_string(), "NaN".to_string()),
        ]);
        assert_eq!(cfg.token, None);
        assert_eq!(cfg.default_timeout_secs, 30);
    }

    #[test]
    fn endpoint_parses_with_https() {
        let cfg = GrpcConfig::from_map([(GrpcConfig::ENV_ADDR.to_string(), "https://localhost:7443".to_string())]);
        assert!(cfg.endpoint().is_ok());
    }
}