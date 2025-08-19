// Re-export the prost generated modules for tests and other crates.
#![allow(clippy::derive_partial_eq_without_eq)]
pub mod generated {
    pub mod mcp {
        pub mod unity {
            pub mod v1 {
                include!("generated/mcp.unity.v1.rs");
            }
        }
    }
}

// Re-export ChannelManager and config so tests can use them as `server::grpc::...`
#[cfg(feature = "transport-grpc")]
pub mod grpc {
    pub mod channel;
    pub mod config; // re-exports from unified config
}

// MCP-related modules
pub mod mcp;
pub mod mcp_types;
pub mod observability;

// Unified configuration module
pub mod config;
