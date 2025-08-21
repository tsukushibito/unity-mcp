use std::{env, time::Duration};

#[derive(Clone, Debug)]
pub struct BridgeConfig {
    pub host: String,
    pub port: u16,
    pub health_timeout_ms: u64,
}

impl BridgeConfig {
    pub fn load() -> Self {
        let host = env::var("MCP_BRIDGE_HOST")
            .or_else(|_| env::var("UNITY_BRIDGE_HOST"))
            .unwrap_or_else(|_| "127.0.0.1".into());
        let port = env::var("MCP_BRIDGE_PORT")
            .or_else(|_| env::var("UNITY_BRIDGE_PORT"))
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(50051);
        let health_timeout_ms = env::var("UNITY_HEALTH_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(2000);
        Self {
            host,
            port,
            health_timeout_ms,
        }
    }

    /// Create BridgeConfig with explicit values for testing
    pub fn with_values(host: String, port: u16, health_timeout_ms: u64) -> Self {
        Self {
            host,
            port,
            health_timeout_ms,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub bridge: BridgeConfig,
}

impl ServerConfig {
    pub fn load() -> Self {
        Self {
            bridge: BridgeConfig::load(),
        }
    }

    /// Create ServerConfig with explicit bridge configuration for testing
    pub fn with_bridge_config(bridge: BridgeConfig) -> Self {
        Self { bridge }
    }

    pub fn health_timeout(&self) -> Duration {
        Duration::from_millis(self.bridge.health_timeout_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_config_loads_bridge_config() {
        let server_config = ServerConfig::load();

        // Should contain bridge config with defaults
        assert_eq!(server_config.bridge.host, "127.0.0.1"); // default
        assert_eq!(server_config.bridge.port, 50051); // default
        assert_eq!(server_config.bridge.health_timeout_ms, 2000); // default
    }

    #[test]
    fn server_config_health_timeout_returns_duration() {
        let server_config = ServerConfig::load();
        let timeout = server_config.health_timeout();

        // Default timeout should be 2000ms
        assert_eq!(timeout, Duration::from_millis(2000));
    }
}
