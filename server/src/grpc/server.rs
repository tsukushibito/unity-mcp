//! gRPC server configuration and setup
//!
//! This module provides the configuration and setup logic for the Tonic-based gRPC server,
//! including binding address, port configuration, and server lifecycle management.

use anyhow::Result;
use std::net::SocketAddr;
use tonic::transport::Server;
use tracing::info;

/// Default gRPC server port
pub const DEFAULT_GRPC_PORT: u16 = 50051;

/// Default gRPC server host
pub const DEFAULT_GRPC_HOST: &str = "127.0.0.1";

/// gRPC server configuration
#[derive(Debug, Clone)]
pub struct GrpcServerConfig {
    /// Host address to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
    /// Maximum message size in bytes (4MB default)
    pub max_message_size: usize,
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_GRPC_HOST.to_string(),
            port: DEFAULT_GRPC_PORT,
            max_message_size: 4 * 1024 * 1024, // 4MB
        }
    }
}

impl GrpcServerConfig {
    /// Create a new server configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the host address
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set the port
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }


    /// Set the maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Get the socket address for binding
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        let addr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid socket address {}: {}", self.host, e))?;
        Ok(addr)
    }
}

/// gRPC server builder that provides a fluent interface for configuring and starting the server
pub struct GrpcServerBuilder {
    config: GrpcServerConfig,
}

impl GrpcServerBuilder {
    /// Create a new server builder with default configuration
    pub fn new() -> Self {
        Self {
            config: GrpcServerConfig::default(),
        }
    }

    /// Create a server builder with custom configuration
    pub fn with_config(config: GrpcServerConfig) -> Self {
        Self { config }
    }

    /// Set the binding host
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Set the binding port
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }


    /// Set maximum message size
    pub fn max_message_size(mut self, size: usize) -> Self {
        self.config.max_message_size = size;
        self
    }

    /// Create a configured Tonic server instance
    ///
    /// This method creates the base server configuration but does not add services.
    /// Services will be added in subsequent tasks.
    pub fn build(self) -> Result<(Server, SocketAddr)> {
        let addr = self.config.socket_addr()?;

        info!(
            host = %self.config.host,
            port = %self.config.port,
            max_message_size = %self.config.max_message_size,
            "Building gRPC server"
        );

        // Note: tonic 0.14 does not support setting message size limits on the transport Server.
        // Apply per-service limits when adding services instead:
        //   MyServiceServer::new(impl)
        //       .max_decoding_message_size(self.config.max_message_size)
        //       .max_encoding_message_size(self.config.max_message_size)
        let server = Server::builder();


        Ok((server, addr))
    }
}

impl Default for GrpcServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility function to create a server with default configuration
pub fn create_default_server() -> Result<(Server, SocketAddr)> {
    GrpcServerBuilder::new().build()
}

/// Utility function to create a server with custom address
pub fn create_server_with_address(host: &str, port: u16) -> Result<(Server, SocketAddr)> {
    GrpcServerBuilder::new().host(host).port(port).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.host, DEFAULT_GRPC_HOST);
        assert_eq!(config.port, DEFAULT_GRPC_PORT);
        assert_eq!(config.max_message_size, 4 * 1024 * 1024);
    }

    #[test]
    fn test_config_builder() {
        let config = GrpcServerConfig::new()
            .with_host("0.0.0.0")
            .with_port(8080)
            .with_max_message_size(8 * 1024 * 1024);

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.max_message_size, 8 * 1024 * 1024);
    }

    #[test]
    fn test_socket_addr() {
        let config = GrpcServerConfig::default();
        let addr = config.socket_addr().unwrap();
        assert_eq!(addr.port(), DEFAULT_GRPC_PORT);
        assert_eq!(addr.ip().to_string(), DEFAULT_GRPC_HOST);
    }

    #[test]
    fn test_server_builder() {
        let builder = GrpcServerBuilder::new()
            .host("localhost")
            .port(9090)
            .max_message_size(1024);

        assert_eq!(builder.config.host, "localhost");
        assert_eq!(builder.config.port, 9090);
        assert_eq!(builder.config.max_message_size, 1024);
    }
}
