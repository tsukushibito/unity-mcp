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

// IPC module for Unity bridge communication
pub mod ipc;

// MCP-related modules
pub mod mcp;
pub mod mcp_types;
pub mod observability;

// Unified configuration module
pub mod config;
