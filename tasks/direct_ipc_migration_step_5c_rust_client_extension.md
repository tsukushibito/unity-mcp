# Step 5C — Rust IpcClient 拡張: Build メソッドとMCPツール実装

**目的:** Rust 側 `IpcClient` に Build 機能用の convenience メソッドを追加し、MCP ツールファサードを実装する。

**前提条件:** Step 5A (Protocol 更新) & Step 5B (Unity Handler) 完了済み

**所要時間:** 2-3時間（Client拡張 + MCPツール + 統合確認）

---

## 実装対象

1. **IpcClient 拡張** - `build_player()`, `build_bundles()` メソッド追加
2. **MCP Build Tool** - ツールファサード実装
3. **統合テスト** - End-to-End 動作確認

---

## 1) IpcClient 拡張実装

### 1.1 ファイル更新場所
```
server/src/ipc/client.rs
```

### 1.2 build_player メソッド追加
```rust
// server/src/ipc/client.rs (既存ファイルに追加)
use crate::generated::mcp::unity::v1 as pb;
use std::time::Duration;

impl IpcClient {
    pub async fn build_player(
        &self, 
        req: pb::BuildPlayerRequest, 
        timeout: Duration
    ) -> Result<pb::BuildPlayerResponse, IpcError> {
        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Build(pb::BuildRequest {
                payload: Some(pb::build_request::Payload::Player(req))
            }))
        };
        
        let resp = self.request(req, timeout).await?;
        
        match resp.payload {
            Some(pb::ipc_response::Payload::Build(pb::BuildResponse {
                payload: Some(pb::build_response::Payload::Player(r))
            })) => Ok(r),
            _ => Err(IpcError::Handshake("unexpected build response".into()))
        }
    }

    pub async fn build_bundles(
        &self, 
        req: pb::BuildAssetBundlesRequest, 
        timeout: Duration
    ) -> Result<pb::BuildAssetBundlesResponse, IpcError> {
        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Build(pb::BuildRequest {
                payload: Some(pb::build_request::Payload::Bundles(req))
            }))
        };
        
        let resp = self.request(req, timeout).await?;
        
        match resp.payload {
            Some(pb::ipc_response::Payload::Build(pb::BuildResponse {
                payload: Some(pb::build_response::Payload::Bundles(r))
            })) => Ok(r),
            _ => Err(IpcError::Handshake("unexpected build response".into()))
        }
    }
}
```

---

## 2) MCP Build Tool 実装

### 2.1 ファイル作成場所
```
server/src/mcp/tools/build.rs
```

### 2.2 BuildTool 構造体
```rust
// server/src/mcp/tools/build.rs
use crate::{ipc::client::IpcClient, generated::mcp::unity::v1 as pb};
use anyhow::Result;
use std::time::Duration;

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
            define_symbols: Default::default(),
        };

        let response = self.ipc.build_player(req, Duration::from_secs(1800)).await?;
        Ok(response)
    }

    /// Build a Unity Player with detailed configuration
    pub async fn build_player_detailed(
        &self,
        platform: pb::BuildPlatform,
        output: String,
        scenes: Vec<String>,
        variants: Option<pb::BuildVariants>,
        define_symbols: std::collections::HashMap<String, String>,
    ) -> Result<pb::BuildPlayerResponse> {
        let req = pb::BuildPlayerRequest {
            platform: platform as i32,
            output_path: output,
            scenes,
            variants: variants.unwrap_or_else(|| pb::BuildVariants {
                architecture: String::new(),
                abis: vec![],
                development: false,
                il2cpp: false,
                strip_symbols: false,
            }),
            define_symbols,
        };

        let response = self.ipc.build_player(req, Duration::from_secs(1800)).await?;
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

        let response = self.ipc.build_bundles(req, Duration::from_secs(1800)).await?;
        Ok(response)
    }

    /// Build AssetBundles with default settings
    pub async fn build_asset_bundles_default(
        &self,
        output_dir: String,
    ) -> Result<pb::BuildAssetBundlesResponse> {
        self.build_asset_bundles(output_dir, true, false, false).await
    }
}
```

### 2.3 プラットフォーム ヘルパー関数
```rust
// server/src/mcp/tools/build.rs (続き)
impl BuildTool {
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
            Default::default(),
        ).await
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
            Default::default(),
        ).await
    }
}
```

---

## 3) mod.rs への統合

### 3.1 ファイル更新場所
```
server/src/mcp/tools/mod.rs
```

### 3.2 build モジュール追加
```rust
// server/src/mcp/tools/mod.rs (既存ファイルに追加)
pub mod build;

pub use build::BuildTool;
```

---

## 4) 統合テスト実装

### 4.1 ファイル作成場所
```
server/tests/build_integration.rs
```

### 4.2 基本テスト
```rust
// server/tests/build_integration.rs
use unity_mcp_server::ipc::{client::IpcClient, path::IpcConfig};
use unity_mcp_server::mcp::tools::BuildTool;
use unity_mcp_server::generated::mcp::unity::v1 as pb;
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
```

---

## 5) 使用例とドキュメント

### 5.1 README 更新例
```rust
// Usage example
use unity_mcp_server::ipc::{client::IpcClient, path::IpcConfig};
use unity_mcp_server::mcp::tools::BuildTool;
use unity_mcp_server::generated::mcp::unity::v1 as pb;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect to Unity Editor
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    // Create build tool
    let build_tool = BuildTool::new(client);
    
    // Build Windows standalone
    let result = build_tool.build_windows_standalone(
        "Builds/MyGame.exe".to_string(),
        vec!["Assets/Scenes/MainMenu.unity".to_string()],
        false, // Release build
    ).await?;
    
    if result.status_code == 0 {
        println!("Build successful: {}", result.output_path);
        println!("Build time: {}ms, Size: {} bytes", 
                 result.build_time_ms, result.size_bytes);
    } else {
        eprintln!("Build failed: {}", result.message);
    }
    
    Ok(())
}
```

---

## 6) 検証項目

### 6.1 コンパイル確認
- [ ] `server/src/ipc/client.rs` の変更がエラーなくコンパイルできる
- [ ] `server/src/mcp/tools/build.rs` がエラーなくコンパイルできる
- [ ] `cargo test --lib` が成功する

### 6.2 型安全性確認
- [ ] `BuildPlatform` enum が正しく使用されている
- [ ] Request/Response の型マッピングが正確である
- [ ] Error handling が適切である

### 6.3 統合テスト確認 (Unity Editor 起動時)
- [ ] IPC 接続が成功する
- [ ] Build Player リクエストが処理される
- [ ] Build AssetBundles リクエストが処理される
- [ ] エラーケースが適切に処理される

---

## 7) Definition of Done (Step 5C)

- [ ] IpcClient に `build_player()`, `build_bundles()` メソッドが追加されている
- [ ] BuildTool ファサードが完全実装されている
- [ ] 統合テストが実装されている
- [ ] 全コードがエラーなくコンパイルできる
- [ ] Step 5D (最終テスト・検証) に進める状態である

---

## 8) 次のステップ

Step 5C 完了後は **Step 5D (統合テスト・検証)** に進む。