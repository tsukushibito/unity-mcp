//! Unity MCP Server Library
//!
//! This library provides a gRPC server implementation for Unity MCP (Model Context Protocol)
//! that bridges Unity Editor functionality with MCP clients.

pub mod grpc;
pub mod unity;

// Re-export commonly used types for easier access
pub use grpc::{UnityMcpService, UnityMcpServiceServer};