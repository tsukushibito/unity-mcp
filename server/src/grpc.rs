//! gRPC module for Unity MCP Server
//!
//! This module provides the gRPC server implementation for the Unity MCP service,
//! including protobuf-generated types and service implementations.

// Import the generated protobuf code
tonic::include_proto!("unity.mcp.v1");

// Re-export the generated types for easier access

// Re-export commonly used error handling functions

// Re-export server configuration types

// Module declarations for sub-modules that will be created in subsequent tasks
pub mod error;
pub mod server;
pub mod service;
pub mod validation;
pub mod performance;
