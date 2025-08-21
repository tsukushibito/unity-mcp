use crate::{generated::mcp::unity::v1 as pb, ipc::client::IpcClient};
use anyhow::Result;
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
