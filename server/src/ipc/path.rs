#[cfg(unix)]
use std::path::PathBuf;
use std::{env, time::Duration};

#[derive(Debug, Clone)]
pub enum Endpoint {
    #[cfg(unix)]
    Unix(PathBuf),
    #[cfg(windows)]
    Pipe(String),
    Tcp(String), // host:port (dev fallback)
}

#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub endpoint: Option<String>, // raw string like "unix:///...", "pipe://...", "tcp://host:port"
    pub token: Option<String>,
    pub connect_timeout: Duration,
    pub handshake_timeout: Duration, // T01: hello送信後の応答待ち
    pub total_handshake_timeout: Duration, // T01: 全体制限時間
    pub call_timeout: Duration,
    pub max_reconnect_attempts: Option<u32>, // Phase 3: 再接続試行回数制限
}

impl Default for IpcConfig {
    fn default() -> Self {
        let is_ci = env::var("CI").is_ok();
        Self {
            endpoint: env::var("MCP_IPC_ENDPOINT").ok(),
            token: env::var("MCP_IPC_TOKEN").ok(),
            // T01 準拠のタイムアウト設定
            connect_timeout: Duration::from_millis(
                env::var("MCP_IPC_CONNECT_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(if is_ci { 5000 } else { 2000 }),
            ),
            handshake_timeout: Duration::from_millis(
                env::var("MCP_IPC_HANDSHAKE_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2000),
            ),
            total_handshake_timeout: Duration::from_millis(
                env::var("MCP_IPC_TOTAL_HANDSHAKE_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(if is_ci { 8000 } else { 3000 }),
            ),
            call_timeout: Duration::from_millis(
                env::var("MCP_IPC_CALL_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4000),
            ),
            max_reconnect_attempts: env::var("MCP_IPC_MAX_RECONNECT_ATTEMPTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(Some(10)), // Default to 10 attempts
        }
    }
}

pub fn default_endpoint() -> Endpoint {
    if let Ok(raw) = env::var("MCP_IPC_ENDPOINT") {
        return parse_endpoint(&raw);
    }
    // Use TCP as default for all platforms to match Unity bridge
    Endpoint::Tcp("127.0.0.1:7777".to_string())
}

pub fn parse_endpoint(s: &str) -> Endpoint {
    #[cfg(unix)]
    {
        if let Some(rest) = s.strip_prefix("unix://") {
            return Endpoint::Unix(PathBuf::from(rest));
        }
    }
    #[cfg(windows)]
    {
        if let Some(rest) = s.strip_prefix("pipe://") {
            return Endpoint::Pipe(rest.to_string());
        }
    }
    if let Some(rest) = s.strip_prefix("tcp://") {
        return Endpoint::Tcp(rest.to_string());
    }
    // Fallback: bare strings are treated as TCP host:port
    Endpoint::Tcp(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_endpoint() {
        let tcp = parse_endpoint("tcp://127.0.0.1:8080");
        matches!(tcp, Endpoint::Tcp(addr) if addr == "127.0.0.1:8080");

        let bare = parse_endpoint("localhost:3000");
        matches!(bare, Endpoint::Tcp(addr) if addr == "localhost:3000");

        #[cfg(unix)]
        {
            let unix = parse_endpoint("unix:///tmp/test.sock");
            matches!(unix, Endpoint::Unix(path) if path == PathBuf::from("/tmp/test.sock"));
        }

        #[cfg(windows)]
        {
            let pipe = parse_endpoint("pipe://test-pipe");
            matches!(pipe, Endpoint::Pipe(name) if name == "test-pipe");
        }
    }

    #[test]
    fn test_ipc_config_default() {
        let config = IpcConfig::default();
        // CI環境でない場合のデフォルト値をテスト
        if std::env::var("CI").is_err() {
            assert_eq!(config.connect_timeout, Duration::from_millis(2000));
            assert_eq!(config.total_handshake_timeout, Duration::from_millis(3000));
        }
        assert_eq!(config.handshake_timeout, Duration::from_millis(2000));
        assert_eq!(config.call_timeout, Duration::from_millis(4000));
    }

    #[test]
    fn test_default_endpoint() {
        let endpoint = default_endpoint();
        match endpoint {
            Endpoint::Tcp(addr) => {
                assert_eq!(addr, "127.0.0.1:7777");
            }
            #[allow(unreachable_patterns)]
            _ => panic!("Expected TCP endpoint, got {:?}", endpoint),
        }
    }
}
