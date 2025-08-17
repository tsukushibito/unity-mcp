// Re-export the prost/tonic generated modules for tests and other crates.
pub mod generated {
    pub mod mcp {
        pub mod unity {
            pub mod v1 {
                tonic::include_proto!("mcp.unity.v1");
            }
        }
    }
}

// Re-export ChannelManager and config so tests can use them as `server::grpc::...`
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
