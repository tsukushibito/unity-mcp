pub mod assets;
pub mod build;
pub mod diagnostics;
pub mod health;
pub mod status;
pub mod tests;

use crate::mcp::service::McpService;
use crate::mcp::tools::build::{UnityBuildAssetBundlesRequest, UnityBuildPlayerRequest};
use crate::mcp::tools::diagnostics::UnityGetCompileDiagnosticsRequest;
use crate::mcp::tools::tests::{UnityGetTestResultsRequest, UnityRunTestsRequest};
use rmcp::{
    ErrorData as McpError, handler::server::tool::Parameters, model::CallToolResult, tool,
    tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[tool_router]
impl McpService {
    #[tool(description = "Unity Bridge connection status (always available)")]
    pub async fn unity_bridge_status(&self) -> Result<CallToolResult, McpError> {
        self.do_unity_bridge_status().await
    }

    #[tool(description = "Unity Bridge health check")]
    pub async fn unity_health(&self) -> Result<CallToolResult, McpError> {
        self.do_unity_health().await
    }

    #[tool(description = "Import Unity assets via Direct IPC")]
    pub async fn unity_assets_import(
        &self,
        Parameters(req): Parameters<UnityAssetsImportRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_assets_import(req.paths, req.recursive, req.auto_refresh, req.timeout_secs)
            .await
    }

    #[tool(description = "Move Unity asset via Direct IPC")]
    pub async fn unity_assets_move(
        &self,
        Parameters(req): Parameters<UnityAssetsMoveRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_assets_move(req.from_path, req.to_path, req.timeout_secs)
            .await
    }

    #[tool(description = "Delete Unity assets via Direct IPC")]
    pub async fn unity_assets_delete(
        &self,
        Parameters(req): Parameters<UnityAssetsDeleteRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_assets_delete(req.paths, req.soft, req.timeout_secs)
            .await
    }

    #[tool(description = "Refresh Unity AssetDatabase via Direct IPC")]
    pub async fn unity_assets_refresh(
        &self,
        Parameters(req): Parameters<UnityAssetsRefreshRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_assets_refresh(req.force, req.timeout_secs)
            .await
    }

    #[tool(description = "Convert Unity asset GUIDs to paths via Direct IPC")]
    pub async fn unity_assets_guid_to_path(
        &self,
        Parameters(req): Parameters<UnityAssetsGuidToPathRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_assets_guid_to_path(req.guids, req.timeout_secs)
            .await
    }

    #[tool(description = "Convert Unity asset paths to GUIDs via Direct IPC")]
    pub async fn unity_assets_path_to_guid(
        &self,
        Parameters(req): Parameters<UnityAssetsPathToGuidRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_assets_path_to_guid(req.paths, req.timeout_secs)
            .await
    }

    #[tool(description = "Build Unity player via Direct IPC")]
    pub async fn unity_build_player(
        &self,
        Parameters(req): Parameters<UnityBuildPlayerRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_build_player(req).await
    }

    #[tool(description = "Build Unity AssetBundles via Direct IPC")]
    pub async fn unity_build_asset_bundles(
        &self,
        Parameters(req): Parameters<UnityBuildAssetBundlesRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_build_asset_bundles(req).await
    }

    #[tool(description = "Get Unity C# compile diagnostics (errors, warnings, info)")]
    pub async fn unity_get_compile_diagnostics(
        &self,
        Parameters(req): Parameters<UnityGetCompileDiagnosticsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_get_compile_diagnostics(
            req.max_items,
            req.severity,
            req.changed_only,
            req.assembly,
        )
        .await
    }

    #[tool(description = "Run Unity tests (EditMode/PlayMode) and get results")]
    pub async fn unity_run_tests(
        &self,
        Parameters(req): Parameters<UnityRunTestsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_run_tests(
            req.mode,
            req.test_filter,
            req.categories,
            req.timeout_sec,
            req.max_items,
            req.include_passed,
        )
        .await
    }

    #[tool(description = "Get Unity test results from previous runs")]
    pub async fn unity_get_test_results(
        &self,
        Parameters(req): Parameters<UnityGetTestResultsRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.do_unity_get_test_results_ipc(req).await
    }
}

// Helper to expose router across modules while the generated
// associated function `tool_router()` remains private to this module.
pub(crate) fn make_tool_router() -> rmcp::handler::server::tool::ToolRouter<McpService> {
    McpService::tool_router()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityAssetsImportRequest {
    pub paths: Vec<String>,
    pub recursive: Option<bool>,
    pub auto_refresh: Option<bool>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityAssetsMoveRequest {
    pub from_path: String,
    pub to_path: String,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityAssetsDeleteRequest {
    pub paths: Vec<String>,
    pub soft: Option<bool>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityAssetsRefreshRequest {
    pub force: Option<bool>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityAssetsGuidToPathRequest {
    pub guids: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityAssetsPathToGuidRequest {
    pub paths: Vec<String>,
    pub timeout_secs: Option<u64>,
}

#[cfg(test)]
mod tool_tests {
    use super::*;

    #[test]
    fn tool_router_has_expected_routes() {
        let router = make_tool_router();
        assert!(router.has_route("unity_bridge_status"));
        assert!(router.has_route("unity_health"));
        assert!(router.has_route("unity_assets_import"));
        assert!(router.has_route("unity_assets_move"));
        assert!(router.has_route("unity_assets_delete"));
        assert!(router.has_route("unity_assets_refresh"));
        assert!(router.has_route("unity_assets_guid_to_path"));
        assert!(router.has_route("unity_assets_path_to_guid"));
        assert!(router.has_route("unity_build_player"));
        assert!(router.has_route("unity_build_asset_bundles"));
        assert!(router.has_route("unity_get_compile_diagnostics"));
        assert!(router.has_route("unity_run_tests"));
        assert!(router.has_route("unity_get_test_results"));
    }
}
