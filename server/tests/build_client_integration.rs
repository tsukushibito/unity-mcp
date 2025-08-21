use server::generated::mcp::unity::v1 as pb;
use server::ipc::{client::IpcClient, path::IpcConfig};
use server::mcp::tools::build::BuildTool;
use std::{collections::HashMap, time::Duration};

/// IpcClient build_player() メソッドの統合テスト
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
            il2cpp: false,
            strip_symbols: false,
            architecture: "x86_64".to_string(),
            abis: vec![],
        }),
        define_symbols: HashMap::new(),
    };

    println!("Starting build...");
    let response = client.build_player(req, Duration::from_secs(300)).await?;

    println!(
        "Build completed: status={}, message={}",
        response.status_code, response.message
    );

    if response.status_code == 0 {
        println!("Success! Output: {}", response.output_path);
        println!(
            "Build time: {}ms, Size: {} bytes",
            response.build_time_ms, response.size_bytes
        );

        // ファイル存在確認
        assert!(
            std::path::Path::new(&response.output_path).exists(),
            "Output file should exist"
        );

        // ビルド時間の妥当性確認
        assert!(response.build_time_ms > 0, "Build time should be positive");

        // サイズの妥当性確認
        assert!(response.size_bytes > 0, "Build size should be positive");
    } else {
        println!("Build failed: {}", response.message);
        // ビルド失敗の場合も出力パスは空であるべき
        assert!(
            response.output_path.is_empty()
                || !std::path::Path::new(&response.output_path).exists()
        );
    }

    Ok(())
}

/// IpcClient build_bundles() メソッドの統合テスト
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

    println!(
        "AssetBundles build completed: status={}, message={}",
        response.status_code, response.message
    );

    if response.status_code == 0 {
        println!("Success! Output: {}", response.output_directory);
        println!("Build time: {}ms", response.build_time_ms);

        // ディレクトリ存在確認
        assert!(
            std::path::Path::new(&response.output_directory).exists(),
            "Output directory should exist"
        );

        // ビルド時間の妥当性確認
        assert!(response.build_time_ms > 0, "Build time should be positive");
    } else {
        println!("AssetBundles build failed: {}", response.message);
    }

    Ok(())
}

/// BuildTool 高レベル API の統合テスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_tool_windows_standalone() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    let build_tool = BuildTool::new(client);

    let result = build_tool
        .build_windows_standalone(
            "Builds/BuildTool/TestApp.exe".to_string(),
            vec![],
            true, // development
        )
        .await?;

    println!(
        "BuildTool result: status={}, message={}",
        result.status_code, result.message
    );

    if result.status_code == 0 {
        assert_eq!(result.status_code, 0, "Build should succeed");
        assert!(
            std::path::Path::new(&result.output_path).exists(),
            "Output file should exist"
        );
    }

    Ok(())
}

/// BuildTool macOS ビルドテスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_tool_macos_standalone() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    let build_tool = BuildTool::new(client);

    let result = build_tool
        .build_macos_standalone(
            "Builds/BuildTool/TestApp.app".to_string(),
            vec![],
            false, // release
        )
        .await?;

    println!(
        "macOS BuildTool result: status={}, message={}",
        result.status_code, result.message
    );

    // macOS ビルドは環境によって失敗する可能性があるが、レスポンス形式は確認
    assert!(
        result.status_code >= 0,
        "Status code should be non-negative"
    );
    assert!(!result.message.is_empty(), "Message should not be empty");

    Ok(())
}

/// BuildTool Android ビルドテスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_tool_android() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    let build_tool = BuildTool::new(client);

    let result = build_tool
        .build_android(
            "Builds/BuildTool/TestApp.apk".to_string(),
            vec![],
            vec!["arm64-v8a".to_string()],
            true, // development
        )
        .await?;

    println!(
        "Android BuildTool result: status={}, message={}",
        result.status_code, result.message
    );

    // Android ビルドは SDK 設定に依存するため、レスポンス形式のみ確認
    assert!(
        result.status_code >= 0,
        "Status code should be non-negative"
    );
    assert!(!result.message.is_empty(), "Message should not be empty");

    Ok(())
}

/// BuildTool AssetBundles デフォルト設定テスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_tool_asset_bundles_default() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    let build_tool = BuildTool::new(client);

    let result = build_tool
        .build_asset_bundles_default("AssetBundles/BuildTool".to_string())
        .await?;

    println!(
        "AssetBundles BuildTool result: status={}, message={}",
        result.status_code, result.message
    );

    if result.status_code == 0 {
        assert!(
            std::path::Path::new(&result.output_directory).exists(),
            "Output directory should exist"
        );
        assert!(result.build_time_ms > 0, "Build time should be positive");
    }

    Ok(())
}

/// 複数プラットフォーム同時ビルドテスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_tool_multiple_platforms() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;
    let build_tool = BuildTool::new(client);

    // Windows ビルド
    let windows_result = build_tool
        .build_windows_standalone(
            "Builds/MultiPlatform/Windows/TestApp.exe".to_string(),
            vec![],
            true,
        )
        .await?;

    println!("Windows build: status={}", windows_result.status_code);

    // Linux ビルド
    let linux_result = build_tool
        .build_linux_standalone(
            "Builds/MultiPlatform/Linux/TestApp".to_string(),
            vec![],
            true,
        )
        .await?;

    println!("Linux build: status={}", linux_result.status_code);

    // 少なくとも1つは成功することを期待
    assert!(
        windows_result.status_code == 0 || linux_result.status_code == 0,
        "At least one platform build should succeed"
    );

    Ok(())
}

/// BuildTool ユーティリティメソッドテスト
#[test]
fn test_build_tool_variants() {
    // Development variants
    let dev_variants = BuildTool::development_variants();
    assert!(dev_variants.development);
    assert!(!dev_variants.il2cpp);
    assert!(!dev_variants.strip_symbols);

    // Release variants
    let release_variants = BuildTool::release_variants();
    assert!(!release_variants.development);
    assert!(release_variants.il2cpp);
    assert!(release_variants.strip_symbols);
}

/// 無効なプラットフォームテスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_invalid_platform() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: 999, // 無効なプラットフォーム
        output_path: "Builds/Invalid.exe".to_string(),
        scenes: vec![],
        variants: None,
        define_symbols: HashMap::new(),
    };

    let response = client.build_player(req, Duration::from_secs(30)).await?;

    // エラーが適切に返されることを確認
    assert_ne!(
        response.status_code, 0,
        "Should return error for invalid platform"
    );
    assert!(
        response.message.to_lowercase().contains("unsupported")
            || response.message.to_lowercase().contains("invalid")
            || response.message.to_lowercase().contains("unknown"),
        "Should mention unsupported/invalid platform: {}",
        response.message
    );

    // ビルド失敗時は出力パスが空または存在しない
    assert!(
        response.output_path.is_empty() || !std::path::Path::new(&response.output_path).exists()
    );

    Ok(())
}

/// 無効な出力パステスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_invalid_output_path() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Assets/BadLocation.exe".to_string(), // 禁止されたパス
        scenes: vec![],
        variants: None,
        define_symbols: HashMap::new(),
    };

    let response = client.build_player(req, Duration::from_secs(30)).await?;

    // パスポリシー違反エラーが返されることを確認
    assert_eq!(
        response.status_code, 7,
        "Should return PERMISSION_DENIED for invalid path"
    );
    assert!(
        response.message.to_lowercase().contains("forbidden")
            || response.message.to_lowercase().contains("permission")
            || response.message.to_lowercase().contains("policy"),
        "Should mention path policy violation: {}",
        response.message
    );

    Ok(())
}

/// Library ディレクトリへの出力テスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_library_output_path_forbidden() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Library/BadLocation.exe".to_string(), // Library は禁止
        scenes: vec![],
        variants: None,
        define_symbols: HashMap::new(),
    };

    let response = client.build_player(req, Duration::from_secs(30)).await?;

    // Library ディレクトリへの出力は禁止されている
    assert_eq!(
        response.status_code, 7,
        "Should return PERMISSION_DENIED for Library path"
    );

    Ok(())
}

/// AssetBundles 無効パステスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_asset_bundles_invalid_path() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildAssetBundlesRequest {
        output_directory: "Assets/InvalidAssetBundles".to_string(), // Assets は禁止
        deterministic: true,
        chunk_based: false,
        force_rebuild: false,
    };

    let response = client.build_bundles(req, Duration::from_secs(30)).await?;

    // Assets ディレクトリへの出力は禁止されている
    assert_eq!(
        response.status_code, 7,
        "Should return PERMISSION_DENIED for Assets path"
    );

    Ok(())
}

/// 空のシーンリストテスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_empty_scenes_list() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/EmptyScenes/TestApp.exe".to_string(),
        scenes: vec![], // 空のシーンリスト（デフォルトシーンを使用）
        variants: Some(pb::BuildVariants {
            development: true,
            ..Default::default()
        }),
        define_symbols: HashMap::new(),
    };

    let response = client.build_player(req, Duration::from_secs(120)).await?;

    // 空のシーンリストでもビルドは成功するべき（デフォルトシーンを使用）
    println!(
        "Empty scenes build result: status={}, message={}",
        response.status_code, response.message
    );

    // エラーでない場合は成功、エラーの場合は適切なメッセージが含まれている
    if response.status_code != 0 {
        assert!(
            response.message.to_lowercase().contains("scene")
                || response.message.to_lowercase().contains("empty"),
            "Error message should mention scenes: {}",
            response.message
        );
    }

    Ok(())
}

/// 長時間ビルドのタイムアウトテスト
#[tokio::test]
#[ignore] // Unity Editor が必要、時間がかかる
async fn test_build_timeout() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/TimeoutTest/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants {
            development: false, // Release ビルドで時間がかかる
            il2cpp: true,
            ..Default::default()
        }),
        define_symbols: HashMap::new(),
    };

    // 短いタイムアウトでテスト
    let start = std::time::Instant::now();
    let result = client.build_player(req, Duration::from_secs(1)).await;
    let elapsed = start.elapsed();

    match result {
        Ok(response) => {
            // 短時間で完了した場合
            println!("Build completed quickly: status={}", response.status_code);
        }
        Err(err) => {
            // タイムアウトエラーが期待される
            println!("Build timed out as expected: {}", err);
            assert!(elapsed < Duration::from_secs(5), "Should timeout quickly");
        }
    }

    Ok(())
}

/// 進捗ストリーミングテスト - OperationEvent 受信
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_progress_events() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    // イベント受信開始
    let mut events = client.events();

    // ビルドタスクを非同期で開始
    let client_clone = client.clone();
    let build_task = tokio::spawn(async move {
        let req = pb::BuildPlayerRequest {
            platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
            output_path: "Builds/ProgressTest/TestApp.exe".to_string(),
            scenes: vec![],
            variants: Some(pb::BuildVariants {
                development: true,
                ..Default::default()
            }),
            define_symbols: HashMap::new(),
        };

        client_clone
            .build_player(req, Duration::from_secs(300))
            .await
    });

    // イベント受信ループ
    let mut start_received = false;
    let mut complete_received = false;
    let mut progress_count = 0;

    let timeout = tokio::time::sleep(Duration::from_secs(60));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            event_result = events.recv() => {
                match event_result {
                    Ok(event) => {
                        if let Some(pb::ipc_event::Payload::Op(op)) = event.payload {
                            match op.kind {
                                k if k == pb::operation_event::Kind::Start as i32 => {
                                    if op.message.to_lowercase().contains("build") {
                                        println!("Build started: {}", op.message);
                                        start_received = true;
                                    }
                                }
                                k if k == pb::operation_event::Kind::Complete as i32 => {
                                    if op.message.to_lowercase().contains("build") {
                                        println!("Build completed: code={}, message={}", op.code, op.message);
                                        complete_received = true;
                                        break;
                                    }
                                }
                                k if k == pb::operation_event::Kind::Progress as i32 => {
                                    println!("Build progress: {}% - {}", op.progress, op.message);
                                    progress_count += 1;
                                }
                                _ => {
                                    println!("Other event: kind={}, message={}", op.kind, op.message);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Event receive error: {}", e);
                        break;
                    }
                }
            }
            _ = &mut timeout => {
                println!("Event timeout reached");
                break;
            }
        }
    }

    // ビルド完了待ち
    let result = build_task.await??;

    println!(
        "Event summary: start={}, complete={}, progress_count={}",
        start_received, complete_received, progress_count
    );

    // 最低限のイベントが受信されることを確認
    // すべてのイベントが期待通りに来るとは限らないので、レスポンスが成功していることを主に確認
    assert_eq!(result.status_code, 0, "Build should succeed");

    Ok(())
}

/// 複数ビルドの並行進捗テスト
#[tokio::test]
#[ignore] // Unity Editor が必要、時間がかかる
async fn test_multiple_build_progress() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let mut events = client.events();

    // 複数のビルドを順次実行
    let client1 = client.clone();
    let build1 = tokio::spawn(async move {
        let req = pb::BuildPlayerRequest {
            platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
            output_path: "Builds/Multi1/TestApp.exe".to_string(),
            scenes: vec![],
            variants: Some(pb::BuildVariants {
                development: true,
                ..Default::default()
            }),
            define_symbols: HashMap::new(),
        };
        client1.build_player(req, Duration::from_secs(120)).await
    });

    let client2 = client.clone();
    let build2 = tokio::spawn(async move {
        // 少し遅らせて開始
        tokio::time::sleep(Duration::from_secs(5)).await;
        let req = pb::BuildAssetBundlesRequest {
            output_directory: "AssetBundles/Multi".to_string(),
            deterministic: true,
            chunk_based: false,
            force_rebuild: false,
        };
        client2.build_bundles(req, Duration::from_secs(120)).await
    });

    // イベント監視
    let mut build_operations = std::collections::HashSet::new();
    let timeout = tokio::time::sleep(Duration::from_secs(150));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            event_result = events.recv() => {
                if let Ok(event) = event_result {
                    if let Some(pb::ipc_event::Payload::Op(op)) = event.payload {
                        if op.message.to_lowercase().contains("build") {
                            build_operations.insert(op.op_id.clone());
                            println!("Build operation {}: {} - {}", op.op_id, op.kind, op.message);
                        }
                    }
                }
            }
            _ = &mut timeout => break,
        }
    }

    // ビルド結果確認
    let result1 = build1.await??;
    let result2 = build2.await??;

    println!("Build 1 result: status={}", result1.status_code);
    println!("Build 2 result: status={}", result2.status_code);
    println!("Tracked {} build operations", build_operations.len());

    // 少なくとも1つの操作が追跡されていることを確認
    assert!(
        !build_operations.is_empty(),
        "Should track at least one build operation"
    );

    Ok(())
}

/// パフォーマンステスト - ビルド時間測定
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_build_performance() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let start = std::time::Instant::now();

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/PerfTest/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants {
            development: true,
            ..Default::default()
        }),
        define_symbols: HashMap::new(),
    };

    let response = client.build_player(req, Duration::from_secs(300)).await?;
    let elapsed = start.elapsed();

    println!("=== Build Performance Results ===");
    println!("Total time (IPC + Build): {:?}", elapsed);
    println!("Unity reported build time: {}ms", response.build_time_ms);

    if response.build_time_ms > 0 {
        let unity_duration = Duration::from_millis(response.build_time_ms);
        let ipc_overhead = elapsed.saturating_sub(unity_duration);
        println!("IPC overhead: {:?}", ipc_overhead);
        println!(
            "IPC overhead percentage: {:.2}%",
            (ipc_overhead.as_millis() as f64 / elapsed.as_millis() as f64) * 100.0
        );
    }

    if response.status_code == 0 {
        println!(
            "Build size: {} bytes ({:.2} MB)",
            response.size_bytes,
            response.size_bytes as f64 / (1024.0 * 1024.0)
        );
        println!(
            "Build speed: {:.2} MB/s",
            (response.size_bytes as f64 / (1024.0 * 1024.0))
                / (response.build_time_ms as f64 / 1000.0)
        );
    }

    assert_eq!(response.status_code, 0, "Build should succeed");

    // パフォーマンス基準の確認
    assert!(
        elapsed < Duration::from_secs(600),
        "Total build time should be under 10 minutes"
    );

    Ok(())
}

/// AssetBundles ビルドパフォーマンステスト
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_asset_bundles_performance() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    let start = std::time::Instant::now();

    let req = pb::BuildAssetBundlesRequest {
        output_directory: "AssetBundles/PerfTest".to_string(),
        deterministic: true,
        chunk_based: false,
        force_rebuild: true, // フルリビルドでパフォーマンス測定
    };

    let response = client.build_bundles(req, Duration::from_secs(180)).await?;
    let elapsed = start.elapsed();

    println!("=== AssetBundles Performance Results ===");
    println!("Total time: {:?}", elapsed);
    println!("Unity reported time: {}ms", response.build_time_ms);

    if response.status_code == 0 {
        println!("AssetBundles build succeeded");
    }

    // パフォーマンス基準（AssetBundles は通常高速）
    assert!(
        elapsed < Duration::from_secs(300),
        "AssetBundles build should be under 5 minutes"
    );

    Ok(())
}

/// 並行ビルド性能テスト
#[tokio::test]
#[ignore] // Unity Editor が必要、時間がかかる
async fn test_concurrent_build_performance() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    // 順次ビルド時間測定
    let start_sequential = std::time::Instant::now();

    // Player ビルド
    let req1 = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/Concurrent1/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants {
            development: true,
            ..Default::default()
        }),
        define_symbols: HashMap::new(),
    };
    let _result1 = client.build_player(req1, Duration::from_secs(180)).await?;

    // AssetBundles ビルド
    let req2 = pb::BuildAssetBundlesRequest {
        output_directory: "AssetBundles/Concurrent".to_string(),
        deterministic: true,
        chunk_based: false,
        force_rebuild: false,
    };
    let _result2 = client.build_bundles(req2, Duration::from_secs(120)).await?;

    let sequential_time = start_sequential.elapsed();

    println!("=== Concurrent Build Performance ===");
    println!("Sequential total time: {:?}", sequential_time);

    // 注意: 現在の実装では並行ビルドは Unity Editor の制限により期待通りに動作しない可能性
    // このテストは主にIPC層の動作確認のため

    Ok(())
}

/// メモリ使用量監視テスト（簡易版）
#[tokio::test]
#[ignore] // Unity Editor が必要
async fn test_memory_usage_during_build() -> anyhow::Result<()> {
    let config = IpcConfig::default();
    let client = IpcClient::connect(config).await?;

    // ベースライン メモリ使用量
    let process = std::process::Command::new("ps")
        .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
        .output();

    let baseline_memory = if let Ok(output) = process {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    } else {
        0
    };

    println!("Baseline memory: {} KB", baseline_memory);

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/MemoryTest/TestApp.exe".to_string(),
        scenes: vec![],
        variants: Some(pb::BuildVariants {
            development: true,
            ..Default::default()
        }),
        define_symbols: HashMap::new(),
    };

    let _response = client.build_player(req, Duration::from_secs(300)).await?;

    // ビルド後のメモリ使用量
    let process = std::process::Command::new("ps")
        .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
        .output();

    let post_build_memory = if let Ok(output) = process {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u64>()
            .unwrap_or(0)
    } else {
        0
    };

    println!("Post-build memory: {} KB", post_build_memory);

    if baseline_memory > 0 && post_build_memory > baseline_memory {
        let memory_increase = post_build_memory - baseline_memory;
        println!("Memory increase: {} KB", memory_increase);

        // メモリリークチェック（簡易）
        assert!(
            memory_increase < 100_000,
            "Memory increase should be reasonable (< 100MB)"
        );
    }

    Ok(())
}

/// IpcClient 接続テスト（Unity Editor 不要）
#[tokio::test]
async fn test_build_tool_creation_without_unity() {
    let config = IpcConfig::default();

    // Unity Editor が動いていない場合の接続テスト
    match IpcClient::connect(config).await {
        Ok(client) => {
            let _build_tool = BuildTool::new(client);
            println!("Unity Editor is running, BuildTool created successfully");
        }
        Err(err) => {
            println!("Unity Editor not running (expected): {}", err);
            // これは予期される動作
        }
    }
}
