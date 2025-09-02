use crate::{generated::mcp::unity::v1 as pb, ipc::client::IpcClient, mcp::service::McpService};
use anyhow::Result;
use rmcp::{ErrorData as McpError, model::CallToolResult, model::Content};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Duration};

pub struct BuildTool {
    pub ipc: IpcClient,
}

impl BuildTool {
    pub fn new(ipc: IpcClient) -> Self {
        Self { ipc }
    }

    /// Build a Unity Player for the specified platform
    pub async fn build_player(
        &self,
        platform: pb::BuildPlatform,
        output: String,
        scenes: Vec<String>,
    ) -> Result<pb::BuildPlayerResponse> {
        let req = pb::BuildPlayerRequest {
            platform: platform as i32,
            output_path: output,
            scenes,
            variants: Some(pb::BuildVariants {
                architecture: String::new(),
                abis: vec![],
                development: false,
                il2cpp: false,
                strip_symbols: false,
            }),
            define_symbols: HashMap::new(),
        };

        let response = self
            .ipc
            .build_player(req, Duration::from_secs(1800))
            .await?;
        Ok(response)
    }

    /// Build a Unity Player with detailed configuration
    pub async fn build_player_detailed(
        &self,
        platform: pb::BuildPlatform,
        output: String,
        scenes: Vec<String>,
        variants: Option<pb::BuildVariants>,
        define_symbols: HashMap<String, String>,
    ) -> Result<pb::BuildPlayerResponse> {
        let req = pb::BuildPlayerRequest {
            platform: platform as i32,
            output_path: output,
            scenes,
            variants: Some(variants.unwrap_or_else(|| pb::BuildVariants {
                architecture: String::new(),
                abis: vec![],
                development: false,
                il2cpp: false,
                strip_symbols: false,
            })),
            define_symbols,
        };

        let response = self
            .ipc
            .build_player(req, Duration::from_secs(1800))
            .await?;
        Ok(response)
    }

    /// Build Unity AssetBundles
    pub async fn build_asset_bundles(
        &self,
        output_dir: String,
        deterministic: bool,
        chunk_based: bool,
        force_rebuild: bool,
    ) -> Result<pb::BuildAssetBundlesResponse> {
        let req = pb::BuildAssetBundlesRequest {
            output_directory: output_dir,
            deterministic,
            chunk_based,
            force_rebuild,
        };

        let response = self
            .ipc
            .build_bundles(req, Duration::from_secs(1800))
            .await?;
        Ok(response)
    }

    /// Build AssetBundles with default settings
    pub async fn build_asset_bundles_default(
        &self,
        output_dir: String,
    ) -> Result<pb::BuildAssetBundlesResponse> {
        self.build_asset_bundles(output_dir, true, false, false)
            .await
    }

    /// Create BuildVariants for development build
    pub fn development_variants() -> pb::BuildVariants {
        pb::BuildVariants {
            architecture: String::new(),
            abis: vec![],
            development: true,
            il2cpp: false,
            strip_symbols: false,
        }
    }

    /// Create BuildVariants for release build
    pub fn release_variants() -> pb::BuildVariants {
        pb::BuildVariants {
            architecture: String::new(),
            abis: vec![],
            development: false,
            il2cpp: true,
            strip_symbols: true,
        }
    }

    /// Helper to build Windows standalone
    pub async fn build_windows_standalone(
        &self,
        output: String,
        scenes: Vec<String>,
        development: bool,
    ) -> Result<pb::BuildPlayerResponse> {
        let variants = if development {
            Self::development_variants()
        } else {
            Self::release_variants()
        };

        self.build_player_detailed(
            pb::BuildPlatform::BpStandaloneWindows64,
            output,
            scenes,
            Some(variants),
            HashMap::new(),
        )
        .await
    }

    /// Helper to build Android APK
    pub async fn build_android(
        &self,
        output: String,
        scenes: Vec<String>,
        abis: Vec<String>,
        development: bool,
    ) -> Result<pb::BuildPlayerResponse> {
        let mut variants = if development {
            Self::development_variants()
        } else {
            Self::release_variants()
        };
        variants.abis = abis;

        self.build_player_detailed(
            pb::BuildPlatform::BpAndroid,
            output,
            scenes,
            Some(variants),
            HashMap::new(),
        )
        .await
    }

    /// Helper to build macOS standalone
    pub async fn build_macos_standalone(
        &self,
        output: String,
        scenes: Vec<String>,
        development: bool,
    ) -> Result<pb::BuildPlayerResponse> {
        let variants = if development {
            Self::development_variants()
        } else {
            Self::release_variants()
        };

        self.build_player_detailed(
            pb::BuildPlatform::BpStandaloneOsx,
            output,
            scenes,
            Some(variants),
            HashMap::new(),
        )
        .await
    }

    /// Helper to build Linux standalone
    pub async fn build_linux_standalone(
        &self,
        output: String,
        scenes: Vec<String>,
        development: bool,
    ) -> Result<pb::BuildPlayerResponse> {
        let variants = if development {
            Self::development_variants()
        } else {
            Self::release_variants()
        };

        self.build_player_detailed(
            pb::BuildPlatform::BpStandaloneLinux64,
            output,
            scenes,
            Some(variants),
            HashMap::new(),
        )
        .await
    }

    /// Helper to build iOS
    pub async fn build_ios(
        &self,
        output: String,
        scenes: Vec<String>,
        development: bool,
    ) -> Result<pb::BuildPlayerResponse> {
        let variants = if development {
            Self::development_variants()
        } else {
            Self::release_variants()
        };

        self.build_player_detailed(
            pb::BuildPlatform::BpIos,
            output,
            scenes,
            Some(variants),
            HashMap::new(),
        )
        .await
    }
}

const DEFAULT_BUILD_TIMEOUT_SECS: u64 = 1800;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityBuildPlayerRequest {
    pub platform: String,
    #[serde(rename = "outputPath")]
    pub output_path: String,
    pub scenes: Option<Vec<String>>,
    pub development: Option<bool>,
    #[serde(rename = "timeoutSecs")]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityBuildAssetBundlesRequest {
    #[serde(rename = "outputDirectory")]
    pub output_directory: String,
    pub deterministic: Option<bool>,
    #[serde(rename = "chunkBased")]
    pub chunk_based: Option<bool>,
    #[serde(rename = "forceRebuild")]
    pub force_rebuild: Option<bool>,
    #[serde(rename = "timeoutSecs")]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPlayerOutput {
    #[serde(rename = "statusCode")]
    pub status_code: i32,
    pub message: String,
    #[serde(rename = "outputPath")]
    pub output_path: String,
    #[serde(rename = "buildTimeMs")]
    pub build_time_ms: u64,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildAssetBundlesOutput {
    #[serde(rename = "statusCode")]
    pub status_code: i32,
    pub message: String,
    #[serde(rename = "outputDirectory")]
    pub output_directory: String,
    #[serde(rename = "buildTimeMs")]
    pub build_time_ms: u64,
}

impl McpService {
    pub(super) async fn do_unity_build_player(
        &self,
        req: UnityBuildPlayerRequest,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(req.timeout_secs.unwrap_or(DEFAULT_BUILD_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;

        let platform = pb::BuildPlatform::from_str_name(&req.platform).ok_or_else(|| {
            McpError::invalid_params(format!("invalid platform: {}", req.platform), None)
        })?;

        let pb_req = pb::BuildPlayerRequest {
            platform: platform as i32,
            output_path: req.output_path,
            scenes: req.scenes.unwrap_or_default(),
            variants: Some(if req.development.unwrap_or(false) {
                BuildTool::development_variants()
            } else {
                BuildTool::release_variants()
            }),
            define_symbols: HashMap::new(),
        };

        let resp = ipc.build_player(pb_req, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Build player IPC error: {}", e), None)
        })?;

        let output = BuildPlayerOutput {
            status_code: resp.status_code,
            message: resp.message,
            output_path: resp.output_path,
            build_time_ms: resp.build_time_ms,
            size_bytes: resp.size_bytes,
            warnings: resp.warnings,
        };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }

    pub(super) async fn do_unity_build_asset_bundles(
        &self,
        req: UnityBuildAssetBundlesRequest,
    ) -> Result<CallToolResult, McpError> {
        let timeout = Duration::from_secs(req.timeout_secs.unwrap_or(DEFAULT_BUILD_TIMEOUT_SECS));
        let ipc = self.require_ipc().await?;

        let pb_req = pb::BuildAssetBundlesRequest {
            output_directory: req.output_directory,
            deterministic: req.deterministic.unwrap_or(true),
            chunk_based: req.chunk_based.unwrap_or(false),
            force_rebuild: req.force_rebuild.unwrap_or(false),
        };

        let resp = ipc.build_bundles(pb_req, timeout).await.map_err(|e| {
            McpError::internal_error(format!("Build bundles IPC error: {}", e), None)
        })?;

        let output = BuildAssetBundlesOutput {
            status_code: resp.status_code,
            message: resp.message,
            output_directory: resp.output_directory,
            build_time_ms: resp.build_time_ms,
        };

        let content = serde_json::to_string(&output)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e), None))?;

        Ok(CallToolResult::success(vec![Content::text(content)]))
    }
}
