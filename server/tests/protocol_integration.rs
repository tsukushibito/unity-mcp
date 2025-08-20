use server::generated::mcp::unity::v1 as pb;
use std::collections::HashMap;

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
        define_symbols: HashMap::new(),
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
    
    // oneof 包装の確認
    if let Some(pb::ipc_request::Payload::Build(build)) = ipc_req.payload {
        assert!(matches!(build.payload, Some(pb::build_request::Payload::Player(_))));
    } else {
        panic!("Expected Build request payload");
    }
}

#[test]
fn test_build_asset_bundles_request_creation() {
    // BuildAssetBundlesRequest 作成
    let bundles_req = pb::BuildAssetBundlesRequest {
        output_directory: "AssetBundles/Test".to_string(),
        deterministic: true,
        chunk_based: false,
        force_rebuild: true,
    };

    // BuildRequest への包装
    let build_req = pb::BuildRequest {
        payload: Some(pb::build_request::Payload::Bundles(bundles_req)),
    };

    // IpcRequest への包装
    let ipc_req = pb::IpcRequest {
        payload: Some(pb::ipc_request::Payload::Build(build_req)),
    };

    // oneof 包装の確認
    if let Some(pb::ipc_request::Payload::Build(build)) = ipc_req.payload {
        if let Some(pb::build_request::Payload::Bundles(bundles)) = build.payload {
            assert_eq!(bundles.output_directory, "AssetBundles/Test");
            assert!(bundles.deterministic);
            assert!(!bundles.chunk_based);
            assert!(bundles.force_rebuild);
        } else {
            panic!("Expected Bundles request");
        }
    } else {
        panic!("Expected Build request payload");
    }
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
        warnings: vec!["Minor optimization warning".to_string()],
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
            assert_eq!(player.output_path, "Builds/TestApp.exe");
            assert_eq!(player.build_time_ms, 30000);
            assert_eq!(player.size_bytes, 50 * 1024 * 1024);
            assert_eq!(player.warnings.len(), 1);
        } else {
            panic!("Expected Player response");
        }
    } else {
        panic!("Expected Build response");
    }
}

#[test]
fn test_build_asset_bundles_response_parsing() {
    // BuildAssetBundlesResponse 作成
    let bundles_resp = pb::BuildAssetBundlesResponse {
        status_code: 0,
        message: "AssetBundles built successfully".to_string(),
        output_directory: "AssetBundles/Test".to_string(),
        build_time_ms: 15000,
    };

    // BuildResponse への包装
    let build_resp = pb::BuildResponse {
        payload: Some(pb::build_response::Payload::Bundles(bundles_resp)),
    };

    // IpcResponse への包装
    let ipc_resp = pb::IpcResponse {
        correlation_id: "bundles-456".to_string(),
        payload: Some(pb::ipc_response::Payload::Build(build_resp)),
    };

    // Parsing 確認
    if let Some(pb::ipc_response::Payload::Build(build)) = ipc_resp.payload {
        if let Some(pb::build_response::Payload::Bundles(bundles)) = build.payload {
            assert_eq!(bundles.status_code, 0);
            assert_eq!(bundles.message, "AssetBundles built successfully");
            assert_eq!(bundles.output_directory, "AssetBundles/Test");
            assert_eq!(bundles.build_time_ms, 15000);
        } else {
            panic!("Expected Bundles response");
        }
    } else {
        panic!("Expected Build response");
    }
}

#[test]
fn test_build_variants_creation() {
    // Development variants
    let dev_variants = pb::BuildVariants {
        development: true,
        il2cpp: false,
        strip_symbols: false,
        architecture: "x86_64".to_string(),
        abis: vec![],
    };

    assert!(dev_variants.development);
    assert!(!dev_variants.il2cpp);
    assert!(!dev_variants.strip_symbols);

    // Release variants
    let release_variants = pb::BuildVariants {
        development: false,
        il2cpp: true,
        strip_symbols: true,
        architecture: "x86_64".to_string(),
        abis: vec![],
    };

    assert!(!release_variants.development);
    assert!(release_variants.il2cpp);
    assert!(release_variants.strip_symbols);

    // Android variants with ABIs
    let android_variants = pb::BuildVariants {
        development: false,
        il2cpp: true,
        strip_symbols: true,
        architecture: "arm64".to_string(),
        abis: vec!["arm64-v8a".to_string(), "armeabi-v7a".to_string()],
    };

    assert_eq!(android_variants.architecture, "arm64");
    assert_eq!(android_variants.abis.len(), 2);
    assert!(android_variants.abis.contains(&"arm64-v8a".to_string()));
}

#[test]
fn test_build_platform_enum() {
    // Platform enum values 確認
    assert_eq!(pb::BuildPlatform::BpUnspecified as i32, 0);
    assert_eq!(pb::BuildPlatform::BpStandaloneWindows64 as i32, 1);
    assert_eq!(pb::BuildPlatform::BpStandaloneOsx as i32, 2);
    assert_eq!(pb::BuildPlatform::BpStandaloneLinux64 as i32, 3);
    assert_eq!(pb::BuildPlatform::BpAndroid as i32, 10);
    assert_eq!(pb::BuildPlatform::BpIos as i32, 11);

    // From i32 conversion 確認
    assert_eq!(pb::BuildPlatform::try_from(1).unwrap(), pb::BuildPlatform::BpStandaloneWindows64);
    assert_eq!(pb::BuildPlatform::try_from(10).unwrap(), pb::BuildPlatform::BpAndroid);
    assert!(pb::BuildPlatform::try_from(999).is_err());
}

#[test]
fn test_error_response_creation() {
    // エラーレスポンス作成
    let error_resp = pb::BuildPlayerResponse {
        status_code: 7, // PERMISSION_DENIED
        message: "Output path is forbidden by security policy".to_string(),
        output_path: String::new(),
        build_time_ms: 0,
        size_bytes: 0,
        warnings: vec![],
    };

    assert_ne!(error_resp.status_code, 0);
    assert!(error_resp.message.contains("forbidden"));
    assert!(error_resp.output_path.is_empty());
}

#[test]
fn test_define_symbols_handling() {
    let mut define_symbols = HashMap::new();
    define_symbols.insert("DEVELOPMENT_BUILD".to_string(), "1".to_string());
    define_symbols.insert("ENABLE_LOGGING".to_string(), "true".to_string());

    let req = pb::BuildPlayerRequest {
        platform: pb::BuildPlatform::BpStandaloneWindows64 as i32,
        output_path: "Builds/TestApp.exe".to_string(),
        scenes: vec![],
        variants: None,
        define_symbols,
    };

    assert_eq!(req.define_symbols.len(), 2);
    assert_eq!(req.define_symbols.get("DEVELOPMENT_BUILD"), Some(&"1".to_string()));
    assert_eq!(req.define_symbols.get("ENABLE_LOGGING"), Some(&"true".to_string()));
}