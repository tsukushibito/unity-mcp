pub mod grpc;

// Re-export generated protobuf modules
pub mod generated {
    pub mod mcp {
        pub mod unity {
            pub mod v1 {
                tonic::include_proto!("mcp.unity.v1");
            }
        }
    }
}
