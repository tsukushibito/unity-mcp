//! gRPC module for Unity MCP Server
//!
//! This module provides the gRPC server implementation for the Unity MCP service,
//! including protobuf-generated types and service implementations.

// Import the generated protobuf code
tonic::include_proto!("unity.mcp.v1");

// Re-export the generated types for easier access
pub use unity_mcp_service_client::UnityMcpServiceClient;
pub use unity_mcp_service_server::{UnityMcpService, UnityMcpServiceServer};

// Re-export commonly used error handling functions
pub use error::{
    internal_error_to_status, internal_server_error, mcp_error_to_status, no_error,
    not_found_error, validation_error,
};

// Re-export server configuration types
pub use server::{
    create_default_server, create_server_with_address, GrpcServerBuilder, GrpcServerConfig,
    DEFAULT_GRPC_HOST, DEFAULT_GRPC_PORT,
};

// Module declarations for sub-modules that will be created in subsequent tasks
pub mod error;
pub mod server;
pub mod service;