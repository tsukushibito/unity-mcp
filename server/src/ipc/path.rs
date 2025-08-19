use std::{env, path::PathBuf, time::Duration};

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
    pub call_timeout: Duration,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            endpoint: env::var("MCP_IPC_ENDPOINT").ok(),
            token: env::var("MCP_IPC_TOKEN").ok(),
            connect_timeout: Duration::from_millis(
                env::var("MCP_IPC_CONNECT_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2000),
            ),
            call_timeout: Duration::from_millis(
                env::var("MCP_IPC_CALL_TIMEOUT_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4000),
            ),
        }
    }
}

pub fn default_endpoint() -> Endpoint {
    if let Ok(raw) = env::var("MCP_IPC_ENDPOINT") {
        return parse_endpoint(&raw);
    }
    // OS-specific defaults
    cfg_if::cfg_if! {
        if #[cfg(unix)] {
            let dir = std::env::var("XDG_RUNTIME_DIR")
                .ok()
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
    if let Some(rest) = s.strip_prefix("unix://") {
        return Endpoint::Unix(PathBuf::from(rest));
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
        assert_eq!(config.connect_timeout, Duration::from_millis(2000));
        assert_eq!(config.call_timeout, Duration::from_millis(4000));
    }

    #[test]
    fn test_default_endpoint() {
        let endpoint = default_endpoint();
        match endpoint {
            #[cfg(unix)]
            Endpoint::Unix(_) => {}, // Expected on Unix
            #[cfg(windows)]
            Endpoint::Pipe(_) => {}, // Expected on Windows
            Endpoint::Tcp(_) => {}, // Expected on other platforms
            #[allow(unreachable_patterns)]
            _ => panic!("Unexpected endpoint type"),
        }
    }
}