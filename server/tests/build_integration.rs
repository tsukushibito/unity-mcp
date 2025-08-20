use server::ipc::{client::IpcClient, path::IpcConfig};
use server::mcp::tools::build::BuildTool;
use std::time::Duration;

#[tokio::test]
#[ignore] // Unity Editor が必要なため通常は無視
async fn test_build_player_integration() -> anyhow::Result<()> {
    // IPC 接続
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    // ヘルスチェック
    let health = client.health(Duration::from_secs(5)).await?;
    assert!(health.ready);

    // BuildTool 作成
    let build_tool = BuildTool::new(client);

    // Windows Standalone ビルド
    let result = build_tool.build_windows_standalone(
        "Builds/TestBuild/TestApp.exe".to_string(),
        vec![], // デフォルトシーン使用
        true,   // 開発ビルド
    ).await?;

    println!("Build result: status_code={}, message={}", result.status_code, result.message);
    println!("Output path: {}", result.output_path);
    println!("Build time: {}ms", result.build_time_ms);
    
    assert_eq!(result.status_code, 0, "Build should succeed");
    Ok(())
}

#[tokio::test]
#[ignore] // Unity Editor が必要なため通常は無視
async fn test_build_asset_bundles_integration() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    let build_tool = BuildTool::new(client);

    let result = build_tool.build_asset_bundles_default(
        "AssetBundles/Test".to_string(),
    ).await?;

    println!("AssetBundles build result: status_code={}, message={}", result.status_code, result.message);
    println!("Output directory: {}", result.output_directory);
    
    assert_eq!(result.status_code, 0, "AssetBundles build should succeed");
    Ok(())
}

#[tokio::test]
#[ignore] // Unity Editor が必要なため通常は無視
async fn test_build_android_integration() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    let build_tool = BuildTool::new(client);

    let result = build_tool.build_android(
        "Builds/Android/TestApp.apk".to_string(),
        vec![], // デフォルトシーン使用
        vec!["arm64-v8a".to_string()], // ARM64 ABI
        true,   // 開発ビルド
    ).await?;

    println!("Android build result: status_code={}, message={}", result.status_code, result.message);
    println!("Output path: {}", result.output_path);
    
    assert_eq!(result.status_code, 0, "Android build should succeed");
    Ok(())
}

#[tokio::test]
#[ignore] // Unity Editor が必要なため通常は無視
async fn test_build_macos_integration() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    let build_tool = BuildTool::new(client);

    let result = build_tool.build_macos_standalone(
        "Builds/macOS/TestApp.app".to_string(),
        vec![], // デフォルトシーン使用
        false,  // リリースビルド
    ).await?;

    println!("macOS build result: status_code={}, message={}", result.status_code, result.message);
    println!("Output path: {}", result.output_path);
    
    assert_eq!(result.status_code, 0, "macOS build should succeed");
    Ok(())
}

#[tokio::test]
async fn test_build_variants_creation() {
    let dev_variants = BuildTool::development_variants();
    assert!(dev_variants.development);
    assert!(!dev_variants.il2cpp);
    assert!(!dev_variants.strip_symbols);

    let release_variants = BuildTool::release_variants();
    assert!(!release_variants.development);
    assert!(release_variants.il2cpp);
    assert!(release_variants.strip_symbols);
}

#[tokio::test]
async fn test_build_tool_creation() {
    // Mock IPC client would be needed for full testing, but this tests the basic structure
    let config = IpcConfig::default();
    
    // This will fail to connect but we just want to test the BuildTool creation
    match IpcClient::connect(config).await {
        Ok(client) => {
            let _build_tool = BuildTool::new(client);
            // If we get here, connection succeeded (Unity Editor is running)
        }
        Err(_) => {
            // Expected when Unity Editor is not running
            println!("Unity Editor not running, skipping connection test");
        }
    }
}