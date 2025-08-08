fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["../proto/unity_mcp.proto"], &["../proto"])?;

    // Tell cargo to rerun this build script if the proto file changes
    println!("cargo:rerun-if-changed=../proto/unity_mcp.proto");

    Ok(())
}
