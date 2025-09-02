use rmcp::handler::server::tool::Parameters;
use server::mcp::service::{McpService, UnityBuildPlayerRequest};

/// MCP サービスのビルドエンドポイント統合テスト
#[tokio::test]
#[ignore] // Unity Editor が必要なため通常は無視
async fn test_unity_build_player_tool() -> anyhow::Result<()> {
    let service = McpService::new().await?;

    // Unity 接続がない場合はスキップ
    if service.require_ipc().await.is_err() {
        println!("Skipping test: Unity Bridge not available");
        return Ok(());
    }

    let req = UnityBuildPlayerRequest {
        platform: "BP_STANDALONE_LINUX64".to_string(),
        output_path: "Builds/Tests/McpPlayer".to_string(),
        scenes: None,
        development: Some(true),
        timeout_secs: Some(60),
    };

    let result = service.unity_build_player(Parameters(req)).await?;
    println!("Build tool result: {:?}", result);
    Ok(())
}
