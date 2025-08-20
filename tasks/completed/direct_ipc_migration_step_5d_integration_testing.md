# Step 5D — 統合テスト・検証: Build Tool End-to-End 動作確認

**目的:** Step 5A-5C で実装した Build 機能の統合テストを実行し、Step 5 全体の完了を確認する。

**前提条件:** Step 5A (Protocol), 5B (Unity), 5C (Rust) 完了済み

**所要時間:** 2-3時間（テスト実行 + バグ修正 + ドキュメント更新）

---

## テスト対象

1. **Protocol 統合確認** - メッセージ送受信の正常性
2. **Unity Build 機能** - Player/AssetBundles ビルドの実行
3. **Rust Client 機能** - IpcClient + BuildTool の動作
4. **エラーハンドリング** - 異常系の適切な処理
5. **進捗ストリーミング** - OperationEvent の配信

---

## 1) 環境準備

### 1.1 Unity Editor 起動
```bash
# Unity Editor を起動し、bridge プロジェクトを開く
# EditorIpcServer が自動的に開始されることを確認
```

### 1.2 Rust サーバービルド確認
```bash
cd server/
cargo build
# コンパイルエラーがないことを確認
```

---

## 2) Protocol 統合テスト

### 2.1 メッセージ生成確認テスト
```rust
// server/tests/protocol_integration.rs (新規作成)
use unity_mcp_server::generated::mcp::unity::v1 as pb;

#[test]
fn test_build_request_creation() {
    // BuildPlayerRequest 作成
    let player_req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/TestApp.exe".to_string(),
        scenes: vec!["Assets/Scenes/Main.unity".to_string()],
        variants: Some(pb::BuildVariants {
            development: true,
            il2cpp: false,
            strip_symbols: false,
            architecture: "x86_64".to_string(),
            abis: vec![],
        }),
        define_symbols: Default::default(),
    };

    // BuildRequest への包装
    let build_req = pb::BuildRequest {
        payload: Some(pb::build_request::Payload::Player(player_req)),
    };

    // IpcRequest への包装
    let ipc_req = pb::IpcRequest {
        payload: Some(pb::ipc_request::Payload::Build(build_req)),
    };

    // Serialization 確認
    assert!(matches!(ipc_req.payload, Some(pb::ipc_request::Payload::Build(_))));
}

#[test]
fn test_build_response_parsing() {
    // BuildPlayerResponse 作成
    let player_resp = pb::BuildPlayerResponse {
        status_code: 0,
        message: "OK".to_string(),
        output_path: "Builds/TestApp.exe".to_string(),
        build_time_ms: 30000,
        size_bytes: 50 * 1024 * 1024, // 50MB
        warnings: vec![],
    };

    // BuildResponse への包装
    let build_resp = pb::BuildResponse {
        payload: Some(pb::build_response::Payload::Player(player_resp)),
    };

    // IpcResponse への包装
    let ipc_resp = pb::IpcResponse {
        correlation_id: "test-123".to_string(),
        payload: Some(pb::ipc_response::Payload::Build(build_resp)),
    };

    // Parsing 確認
    if let Some(pb::ipc_response::Payload::Build(build)) = ipc_resp.payload {
        if let Some(pb::build_response::Payload::Player(player)) = build.payload {
            assert_eq!(player.status_code, 0);
            assert_eq!(player.message, "OK");
        } else {
            panic!("Expected Player response");
        }
    } else {
        panic!("Expected Build response");
    }
}
```

### 2.2 テスト実行
```bash
cd server/
cargo test protocol_integration
```

---

## 3) Unity Build 機能テスト

### 3.1 手動テスト (Unity Editor Console で実行)
```csharp
// Unity Editor Console で実行するテストコード
using Mcp.Unity.V1.Ipc;
using Pb = Mcp.Unity.V1;

// Build Player テスト
var playerReq = new Pb.BuildPlayerRequest
{
    Platform = Pb.BuildPlatform.BpStandaloneWindows64,
    OutputPath = "Builds/ManualTest/TestApp.exe",
    Scenes = { }, // デフォルトシーン使用
    Variants = new Pb.BuildVariants { Development = true }
};

var buildReq = new Pb.BuildRequest { Player = playerReq };
var response = BuildHandler.Handle(buildReq);

UnityEngine.Debug.Log($"Build result: {response.Player.StatusCode} - {response.Player.Message}");
if (response.Player.StatusCode == 0)
{
    UnityEngine.Debug.Log($"Output: {response.Player.OutputPath}");
    UnityEngine.Debug.Log($"Size: {response.Player.SizeBytes} bytes");
    UnityEngine.Debug.Log($"Time: {response.Player.BuildTimeMs} ms");
}
```

### 3.2 AssetBundles テスト
```csharp
// AssetBundles ビルドテスト
var bundlesReq = new Pb.BuildAssetBundlesRequest
{
    OutputDirectory = "AssetBundles/ManualTest",
    Deterministic = true,
    ChunkBased = false,
    ForceRebuild = true
};

var buildReq = new Pb.BuildRequest { Bundles = bundlesReq };
var response = BuildHandler.Handle(buildReq);

UnityEngine.Debug.Log($"AssetBundles result: {response.Bundles.StatusCode} - {response.Bundles.Message}");
if (response.Bundles.StatusCode == 0)
{
    UnityEngine.Debug.Log($"Output: {response.Bundles.OutputDirectory}");
    UnityEngine.Debug.Log($"Time: {response.Bundles.BuildTimeMs} ms");
}
```

---

## 4) Rust Client 統合テスト

### 4.1 IpcClient Build メソッドテスト
```rust
// server/tests/build_client_integration.rs (新規作成)
use unity_mcp_server::ipc::{client::IpcClient, path::IpcConfig};
use unity_mcp_server::generated::mcp::unity::v1 as pb;
use std::time::Duration;

#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_ipc_client_build_player() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    // Health check
    let health = client.health(Duration::from_secs(5)).await?;
    assert!(health.ready, "Unity Editor should be ready");

    // Build Player request
    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/TestIntegration/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants {
            development: true,
            ..Default::default()
        }),
        define_symbols: Default::default(),
    };

    println!("Starting build...");
    let response = client.build_player(req, Duration::from_secs(300)).await?;

    println!("Build completed: status={}, message={}", response.status_code, response.message);
    
    if response.status_code == 0 {
        println!("Success! Output: {}", response.output_path);
        println!("Build time: {}ms, Size: {} bytes", response.build_time_ms, response.size_bytes);
        
        // ファイル存在確認
        assert!(std::path::Path::new(&response.output_path).exists(), "Output file should exist");
    } else {
        println!("Build failed: {}", response.message);
    }

    Ok(())
}

#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_ipc_client_build_bundles() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildAssetBundlesRequest {
        output_directory: "AssetBundles/TestIntegration".to_string(),
        deterministic: true,
        chunk_based: false,
        force_rebuild: false,
    };

    println!("Starting AssetBundles build...");
    let response = client.build_bundles(req, Duration::from_secs(120)).await?;

    println!("AssetBundles build completed: status={}, message={}", response.status_code, response.message);
    
    if response.status_code == 0 {
        println!("Success! Output: {}", response.output_directory);
        println!("Build time: {}ms", response.build_time_ms);
        
        // ディレクトリ存在確認
        assert!(std::path::Path::new(&response.output_directory).exists(), "Output directory should exist");
    }

    Ok(())
}
```

### 4.2 BuildTool テスト
```rust
// BuildTool の高レベル API テスト
#[tokio::test]
#[ignore]
async fn test_build_tool_windows_standalone() -> anyhow::Result<()> {
    use unity_mcp_server::mcp::tools::BuildTool;
    
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    let build_tool = BuildTool::new(client);

    let result = build_tool.build_windows_standalone(
        "Builds/BuildTool/TestApp.exe".to_string(),
        vec![],
        true, // development
    ).await?;

    println!("BuildTool result: status={}, message={}", result.status_code, result.message);
    assert_eq!(result.status_code, 0, "Build should succeed");
    
    Ok(())
}
```

---

## 5) エラーハンドリングテスト

### 5.1 無効入力テスト
```rust
#[tokio::test]
#[ignore]
async fn test_invalid_platform() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: 999, // 無効なプラットフォーム
        output_path: "Builds/Invalid.exe".to_string(),
        scenes: vec![],
        variants: None,
        define_symbols: Default::default(),
    };

    let response = client.build_player(req, Duration::from_secs(30)).await?;
    
    // エラーが適切に返されることを確認
    assert_ne!(response.status_code, 0, "Should return error for invalid platform");
    assert!(response.message.contains("unsupported"), "Should mention unsupported platform");
    
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_invalid_output_path() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Assets/BadLocation.exe".to_string(), // 禁止されたパス
        scenes: vec![],
        variants: None,
        define_symbols: Default::default(),
    };

    let response = client.build_player(req, Duration::from_secs(30)).await?;
    
    // パスポリシー違反エラーが返されることを確認
    assert_eq!(response.status_code, 7, "Should return PERMISSION_DENIED for invalid path");
    assert!(response.message.contains("forbidden"), "Should mention path policy violation");
    
    Ok(())
}
```

---

## 6) 進捗ストリーミングテスト

### 6.1 OperationEvent 受信テスト
```rust
#[tokio::test]
#[ignore]
async fn test_build_progress_events() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    // イベント受信開始
    let mut events = client.events();
    
    // ビルド開始
    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/ProgressTest/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants { development: true, ..Default::default() }),
        define_symbols: Default::default(),
    };

    let build_task = tokio::spawn(async move {
        client.build_player(req, Duration::from_secs(300)).await
    });

    // イベント受信
    let mut start_received = false;
    let mut complete_received = false;
    
    while let Ok(event) = events.recv().await {
        if let Some(pb::ipc_event::Payload::Op(op)) = event.payload {
            match op.kind() {
                pb::operation_event::Kind::Start if op.message.contains("BuildPlayer") => {
                    println!("Build started: {}", op.message);
                    start_received = true;
                }
                pb::operation_event::Kind::Complete if op.message.contains("BuildPlayer") => {
                    println!("Build completed: code={}, message={}", op.code, op.message);
                    complete_received = true;
                    break;
                }
                pb::operation_event::Kind::Progress => {
                    println!("Build progress: {}% - {}", op.progress, op.message);
                }
                _ => {}
            }
        }
    }

    // ビルド完了待ち
    let result = build_task.await??;
    
    assert!(start_received, "Should receive START event");
    assert!(complete_received, "Should receive COMPLETE event");
    assert_eq!(result.status_code, 0, "Build should succeed");
    
    Ok(())
}
```

---

## 7) 総合テスト実行

### 7.1 テスト実行手順
```bash
# 1. Unity Editor 起動 (bridge プロジェクト)
# 2. Rust テスト実行
cd server/

# Protocol テスト (Unity Editor 不要)
cargo test protocol_integration

# 統合テスト (Unity Editor 必要)
cargo test build_client_integration -- --ignored

# 全テスト (時間がかかる)
cargo test build -- --ignored --nocapture
```

### 7.2 手動検証項目
- [ ] Unity Editor Console でエラーが出ていない
- [ ] Builds/ ディレクトリに出力ファイルが作成されている
- [ ] AssetBundles/ ディレクトリに Bundle ファイルが作成されている
- [ ] OperationEvent が適切に配信されている
- [ ] エラーケースで適切なステータスコードが返される

---

## 8) パフォーマンス確認

### 8.1 ビルド時間測定
```rust
#[tokio::test]
#[ignore]
async fn test_build_performance() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    
    let start = std::time::Instant::now();
    
    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/PerfTest/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants { development: true, ..Default::default() }),
        define_symbols: Default::default(),
    };

    let response = client.build_player(req, Duration::from_secs(300)).await?;
    let elapsed = start.elapsed();
    
    println!("Total time (IPC + Build): {:?}", elapsed);
    println!("Unity reported build time: {}ms", response.build_time_ms);
    println!("IPC overhead: {:?}", elapsed - Duration::from_millis(response.build_time_ms));
    
    assert_eq!(response.status_code, 0);
    Ok(())
}
```

---

## 9) Definition of Done (Step 5D)

### 9.1 必須確認項目
- [ ] Protocol 統合テストが全て成功する
- [ ] Unity Build 機能テストが成功する  
- [ ] Rust Client 統合テストが成功する
- [ ] エラーハンドリングが正しく動作する
- [ ] 進捗ストリーミングが正常に機能する

### 9.2 出力成果物確認
- [ ] Player ビルドで実行可能ファイルが生成される
- [ ] AssetBundles ビルドで Bundle ファイルが生成される
- [ ] ビルド時間とサイズが適切に報告される
- [ ] ログに異常がない

### 9.3 ドキュメント更新
- [ ] テスト結果をドキュメント化
- [ ] 既知の制限事項があれば記録
- [ ] 次のステップ (Step 6) への準備確認

---

## 10) Step 5 全体完了確認

Step 5A, 5B, 5C, 5D が全て完了し、以下が実現されていることを確認：

- [ ] Protocol Buffer 定義が完全実装されている
- [ ] Unity 側で Player/AssetBundles ビルドが実行できる
- [ ] Rust 側でビルド要求・結果取得ができる
- [ ] IPC 経由でビルド進捗がストリーミングされる
- [ ] セキュリティポリシーが適切に実装されている
- [ ] エラーケースが適切に処理される

**Step 5 完了後、Step 6 (gRPC 成果物削除・最終統合) に進む。**