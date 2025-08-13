use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // tonic 0.14.1推奨パターン: from_static + connect
    let _channel = tonic::transport::Channel::from_static("http://127.0.0.1:50051")
        .connect()
        .await;

    // Unity Bridge無しでも動作することを示す（接続エラーは無視）
    println!("gRPC client stubs compiled and binary runs.");
    Ok(())
}
