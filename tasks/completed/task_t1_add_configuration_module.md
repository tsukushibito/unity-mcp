# T1 — Configuration Module (gRPC client): Fixes & Final Implementation

This document amends the original plan and provides a production‑ready `GrpcConfig` with robust env parsing, safe defaults, and testable construction without touching process env. It also aligns module paths with the T4 smoke test by re‑exporting under `server::grpc::config` while keeping the requested file at `server/src/config.rs`.

---

## Key Fixes

1. **Address normalization**
   If users set `MCP_BRIDGE_ADDR` to `127.0.0.1:50051`, `tonic::Endpoint` requires a URI with scheme. We normalize automatically to `http://127.0.0.1:50051` (scheme preserved if present).

2. **Token semantics**
   Empty or whitespace tokens are treated as `None` to avoid sending an empty `Authorization` header later.

3. **Timeout safety**
   `MCP_BRIDGE_TIMEOUT` is parsed as seconds. Invalid values are ignored (default retained). A `timeout()` convenience returns `Duration`.

4. **Testability without global env**
   `from_env()` delegates to a generic `from_reader` and a test‑friendly `from_map(...)` so unit tests don’t mutate process env or require serial execution.

5. **Path alignment**
   The file lives at `server/src/config.rs` as requested. A thin re‑export at `server::grpc::config` maintains consistency with other modules and the T4 test skeleton.

---

## File: `server/src/config.rs`

```rust
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
            (GrpcConfig::ENV_ADDR.to_string(), "127.0.0.1:50051".into()),
            (GrpcConfig::ENV_TOKEN.to_string(), "abc".into()),
            (GrpcConfig::ENV_TIMEOUT.to_string(), "5".into()),
        ]);
        assert_eq!(cfg.addr, "http://127.0.0.1:50051");
        assert_eq!(cfg.token.as_deref(), Some("abc"));
        assert_eq!(cfg.default_timeout_secs, 5);
    }

    #[test]
    fn empty_token_is_none_and_bad_timeout_is_ignored() {
        let cfg = GrpcConfig::from_map([
            (GrpcConfig::ENV_TOKEN.to_string(), "   ".into()),
            (GrpcConfig::ENV_TIMEOUT.to_string(), "NaN".into()),
        ]);
        assert_eq!(cfg.token, None);
        assert_eq!(cfg.default_timeout_secs, 30);
    }

    #[test]
    fn endpoint_parses_with_https() {
        let cfg = GrpcConfig::from_map([(GrpcConfig::ENV_ADDR.to_string(), "https://localhost:7443".into())]);
        assert!(cfg.endpoint().is_ok());
    }
}
```

---

## Re‑export under `server::grpc::config`

If you already have a `server/src/grpc/mod.rs`, add this thin shim so callers (and the T4 test) can import `server::grpc::config::GrpcConfig` without changing the file location:

**`server/src/grpc/mod.rs`** (append)

```rust
pub mod config {
    pub use crate::config::*; // re-export the same types/functions
}
```

And ensure lib exports both modules:

**`server/src/lib.rs`** (excerpt)

```rust
pub mod config; // actual implementation lives here
pub mod grpc;   // existing grpc module; now also re-exports config
```

---

## Usage Examples

```rust
use server::grpc::config::GrpcConfig;

let cfg = GrpcConfig::from_env();
let ep = cfg.endpoint()?; // tonic::transport::Endpoint
let timeout = cfg.timeout();
```

---

## Acceptance Checklist

* ✅ Builds with no additional dependencies.
* ✅ Unit tests cover defaults, overrides, invalid inputs, and endpoint parsing.
* ✅ Works whether env vars are present or not.
* ✅ Address normalization ensures a valid URI for `tonic::Endpoint`.
* ✅ Available at both `server::config::GrpcConfig` and `server::grpc::config::GrpcConfig`.