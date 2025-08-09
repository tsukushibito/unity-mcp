//! Comprehensive test suite for validation module
//!
//! This test module covers security testing, validation testing, and integration testing

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::*;
use crate::grpc::{
    stream_request, DeleteAssetRequest, ImportAssetRequest, MoveAssetRequest, RefreshRequest,
};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_context() -> ValidationContext {
    ValidationContext {
        client_id: "test_client".to_string(),
        connection_id: "test_connection".to_string(),
        message_id: 1,
        timestamp: SystemTime::now(),
        client_info: Some(ClientInfo {
            user_agent: Some("Unity Editor/2023.3.0f1".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
            unity_version: Some("2023.3.0f1".to_string()),
        }),
    }
}

fn create_valid_import_request() -> StreamRequest {
    StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: "Assets/Scripts/TestScript.cs".to_string(),
        })),
    }
}

fn create_malicious_import_request() -> StreamRequest {
    StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: "Assets/../../../etc/passwd".to_string(),
        })),
    }
}

fn create_empty_path_request() -> StreamRequest {
    StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: "".to_string(),
        })),
    }
}

// ============================================================================
// Unit Tests - Asset Path Validator
// ============================================================================

#[test]
fn test_asset_path_validator_valid_paths() {
    let validator = AssetPathValidator::new();

    // Valid Unity asset paths
    assert!(validator
        .validate_asset_path("Assets/Scripts/Test.cs")
        .is_ok());
    assert!(validator
        .validate_asset_path("Assets/Textures/icon.png")
        .is_ok());
    assert!(validator
        .validate_asset_path("Assets/Models/character.fbx")
        .is_ok());
    assert!(validator
        .validate_asset_path("Assets/Audio/background.mp3")
        .is_ok());
    assert!(validator
        .validate_asset_path("Assets/Scenes/MainScene.unity")
        .is_ok());
}

#[test]
fn test_asset_path_validator_invalid_paths() {
    let validator = AssetPathValidator::new();

    // Empty path
    assert!(validator.validate_asset_path("").is_err());

    // Path not starting with Assets/
    assert!(validator.validate_asset_path("Scripts/Test.cs").is_err());
    assert!(validator.validate_asset_path("Test.cs").is_err());

    // Path traversal attempts
    assert!(validator
        .validate_asset_path("Assets/../Scripts/Test.cs")
        .is_err());
    assert!(validator
        .validate_asset_path("Assets/Scripts/../../etc/passwd")
        .is_err());
    assert!(validator
        .validate_asset_path("Assets/..\\Scripts\\Test.cs")
        .is_err());

    // Invalid characters
    assert!(validator
        .validate_asset_path("Assets/Scripts/<script>.cs")
        .is_err());
    assert!(validator
        .validate_asset_path("Assets/Scripts/test|file.cs")
        .is_err());
    assert!(validator
        .validate_asset_path("Assets/Scripts/test?.cs")
        .is_err());
    assert!(validator
        .validate_asset_path("Assets/Scripts/test*.cs")
        .is_err());
    assert!(validator
        .validate_asset_path("Assets/Scripts/test\".cs")
        .is_err());
}

#[test]
fn test_asset_path_validator_normalize_path() {
    let validator = AssetPathValidator::new();

    // Path normalization
    assert_eq!(
        validator
            .normalize_path("Assets\\Scripts\\Test.cs")
            .unwrap(),
        "Assets/Scripts/Test.cs"
    );

    // Remove consecutive slashes
    assert_eq!(
        validator
            .normalize_path("Assets//Scripts//Test.cs")
            .unwrap(),
        "Assets/Scripts/Test.cs"
    );

    // Add Assets prefix if missing
    assert_eq!(
        validator.normalize_path("Scripts/Test.cs").unwrap(),
        "Assets/Scripts/Test.cs"
    );

    // Already normalized path
    assert_eq!(
        validator.normalize_path("Assets/Scripts/Test.cs").unwrap(),
        "Assets/Scripts/Test.cs"
    );
}

// ============================================================================
// Unit Tests - Security Validator
// ============================================================================

#[test]
fn test_security_validator_valid_requests() {
    let validator = SecurityValidator::new();

    let valid_import = ImportAssetRequest {
        asset_path: "Assets/Scripts/Test.cs".to_string(),
    };
    assert!(validator.validate_import_request(&valid_import).is_ok());

    let valid_move = MoveAssetRequest {
        src_path: "Assets/Scripts/Old.cs".to_string(),
        dst_path: "Assets/Scripts/New.cs".to_string(),
    };
    assert!(validator.validate_move_request(&valid_move).is_ok());

    let valid_delete = DeleteAssetRequest {
        asset_path: "Assets/Scripts/Temp.cs".to_string(),
    };
    assert!(validator.validate_delete_request(&valid_delete).is_ok());
}

#[test]
fn test_security_validator_malicious_requests() {
    let validator = SecurityValidator::new();

    // Path traversal in import
    let malicious_import = ImportAssetRequest {
        asset_path: "Assets/../../../etc/passwd".to_string(),
    };
    assert!(validator
        .validate_import_request(&malicious_import)
        .is_err());

    // Script injection attempts
    let script_injection = ImportAssetRequest {
        asset_path: "Assets/Scripts/javascript:alert('xss')".to_string(),
    };
    assert!(validator
        .validate_import_request(&script_injection)
        .is_err());

    // Invalid characters
    let invalid_chars = ImportAssetRequest {
        asset_path: "Assets/Scripts/test<script>.cs".to_string(),
    };
    assert!(validator.validate_import_request(&invalid_chars).is_err());

    // Control characters
    let control_chars = ImportAssetRequest {
        asset_path: "Assets/Scripts/test\x00.cs".to_string(),
    };
    assert!(validator.validate_import_request(&control_chars).is_err());
}

// ============================================================================
// Unit Tests - Rate Limiter
// ============================================================================

#[tokio::test]
async fn test_rate_limiter_allows_requests_within_limit() {
    let limiter = RateLimiter::new(3, Duration::from_secs(60));
    let client_id = "test_client";

    // Should allow up to the limit
    assert!(limiter.is_allowed(client_id).await);
    assert!(limiter.is_allowed(client_id).await);
    assert!(limiter.is_allowed(client_id).await);

    // Should reject when limit exceeded
    assert!(!limiter.is_allowed(client_id).await);
}

#[tokio::test]
async fn test_rate_limiter_different_clients() {
    let limiter = RateLimiter::new(2, Duration::from_secs(60));

    // Different clients should have separate limits
    assert!(limiter.is_allowed("client1").await);
    assert!(limiter.is_allowed("client2").await);
    assert!(limiter.is_allowed("client1").await);
    assert!(limiter.is_allowed("client2").await);

    // Each client should be limited independently
    assert!(!limiter.is_allowed("client1").await);
    assert!(!limiter.is_allowed("client2").await);
}

#[tokio::test]
async fn test_rate_limiter_window_reset() {
    let limiter = RateLimiter::new(1, Duration::from_millis(100));
    let client_id = "test_client";

    // Use up the limit
    assert!(limiter.is_allowed(client_id).await);
    assert!(!limiter.is_allowed(client_id).await);

    // Wait for window to reset
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should allow requests again
    assert!(limiter.is_allowed(client_id).await);
}

// ============================================================================
// Unit Tests - Individual Validators
// ============================================================================

#[test]
fn test_import_asset_validator() {
    let validator = ImportAssetStreamValidator::new();

    // Valid request
    let valid_req = ImportAssetRequest {
        asset_path: "Assets/Scripts/Test.cs".to_string(),
    };
    assert!(validator.validate(&valid_req).is_ok());

    // Empty path
    let empty_req = ImportAssetRequest {
        asset_path: "".to_string(),
    };
    assert!(matches!(
        validator.validate(&empty_req),
        Err(ImportAssetValidationError::EmptyAssetPath)
    ));

    // Invalid path
    let invalid_req = ImportAssetRequest {
        asset_path: "Scripts/Test.cs".to_string(),
    };
    assert!(matches!(
        validator.validate(&invalid_req),
        Err(ImportAssetValidationError::InvalidPath(_))
    ));
}

#[test]
fn test_move_asset_validator() {
    let validator = MoveAssetStreamValidator::new();

    // Valid request
    let valid_req = MoveAssetRequest {
        src_path: "Assets/Scripts/Old.cs".to_string(),
        dst_path: "Assets/Scripts/New.cs".to_string(),
    };
    assert!(validator.validate(&valid_req).is_ok());

    // Empty source path
    let empty_src = MoveAssetRequest {
        src_path: "".to_string(),
        dst_path: "Assets/Scripts/New.cs".to_string(),
    };
    assert!(matches!(
        validator.validate(&empty_src),
        Err(MoveAssetValidationError::EmptySourcePath)
    ));

    // Empty destination path
    let empty_dst = MoveAssetRequest {
        src_path: "Assets/Scripts/Old.cs".to_string(),
        dst_path: "".to_string(),
    };
    assert!(matches!(
        validator.validate(&empty_dst),
        Err(MoveAssetValidationError::EmptyDestinationPath)
    ));

    // Same source and destination
    let same_paths = MoveAssetRequest {
        src_path: "Assets/Scripts/Test.cs".to_string(),
        dst_path: "Assets/Scripts/Test.cs".to_string(),
    };
    assert!(matches!(
        validator.validate(&same_paths),
        Err(MoveAssetValidationError::SameSourceAndDestination)
    ));
}

#[test]
fn test_delete_asset_validator() {
    let validator = DeleteAssetStreamValidator::new();

    // Valid request
    let valid_req = DeleteAssetRequest {
        asset_path: "Assets/Scripts/Test.cs".to_string(),
    };
    assert!(validator.validate(&valid_req).is_ok());

    // Empty path
    let empty_req = DeleteAssetRequest {
        asset_path: "".to_string(),
    };
    assert!(matches!(
        validator.validate(&empty_req),
        Err(DeleteAssetValidationError::EmptyAssetPath)
    ));
}

#[test]
fn test_refresh_validator() {
    let validator = RefreshStreamValidator::new();

    // Refresh requests are always valid
    let req = RefreshRequest {};
    assert!(validator.validate(&req).is_ok());
}

// ============================================================================
// Integration Tests - Stream Validation Engine
// ============================================================================

#[tokio::test]
async fn test_stream_validation_engine_valid_request() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();
    let request = create_valid_import_request();

    let result = engine.validate_stream_request(&request, &context).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_stream_validation_engine_empty_message() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();
    let request = StreamRequest { message: None };

    let result = engine.validate_stream_request(&request, &context).await;
    assert!(matches!(result, Err(StreamValidationError::EmptyMessage)));
}

#[tokio::test]
async fn test_stream_validation_engine_invalid_path() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();
    let request = create_empty_path_request();

    let result = engine.validate_stream_request(&request, &context).await;
    assert!(matches!(
        result,
        Err(StreamValidationError::ImportAsset(
            ImportAssetValidationError::EmptyAssetPath
        ))
    ));
}

#[tokio::test]
async fn test_stream_validation_engine_sanitization() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();

    // Create request with path that needs normalization (but is still valid)
    let request = StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: "Assets\\Scripts\\Test.cs".to_string(),
        })),
    };

    // First validate (should pass since the core path is valid)
    assert!(engine
        .validate_stream_request(&request, &context)
        .await
        .is_ok());

    // Then sanitize
    let sanitized = engine
        .sanitize_stream_request(request, &context)
        .await
        .unwrap();

    // Check that path was normalized (backslashes should be converted to forward slashes)
    if let Some(stream_request::Message::ImportAsset(req)) = sanitized.message {
        assert_eq!(req.asset_path, "Assets/Scripts/Test.cs");
    } else {
        panic!("Expected ImportAsset message");
    }
}

// ============================================================================
// Security Tests
// ============================================================================

#[tokio::test]
async fn test_path_traversal_prevention() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();
    let request = create_malicious_import_request();

    let result = engine.validate_stream_request(&request, &context).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_injection_attack_prevention() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();

    // Script injection test
    let script_injection = StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: "Assets/Scripts/javascript:alert('xss')".to_string(),
        })),
    };

    let result = engine
        .validate_stream_request(&script_injection, &context)
        .await;
    assert!(result.is_err());

    // Data URI test
    let data_uri = StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: "Assets/Scripts/data:text/html,<script>alert('xss')</script>".to_string(),
        })),
    };

    let result = engine.validate_stream_request(&data_uri, &context).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rate_limiting_integration() {
    let engine = StreamValidationEngine::new();
    let mut context = create_test_context();
    let request = create_valid_import_request();

    // Perform many requests to trigger rate limiting
    for i in 0..150 {
        context.message_id = i;
        let result = engine.validate_stream_request(&request, &context).await;

        if i < 100 {
            // Should pass for first 100 requests
            assert!(result.is_ok(), "Request {} should have passed", i);
        } else {
            // Should fail for requests over the limit
            assert!(
                result.is_err(),
                "Request {} should have been rate limited",
                i
            );
            assert!(matches!(
                result,
                Err(StreamValidationError::Security(
                    SecurityValidationError::RateLimitExceeded { .. }
                ))
            ));
        }
    }
}

#[tokio::test]
async fn test_message_size_limits() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();

    // Create oversized request
    let large_path = "A".repeat(100_000); // Much larger than 64KB limit
    let oversized_request = StreamRequest {
        message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
            asset_path: format!("Assets/Scripts/{}.cs", large_path),
        })),
    };

    let result = engine
        .validate_stream_request(&oversized_request, &context)
        .await;
    assert!(matches!(
        result,
        Err(StreamValidationError::Security(
            SecurityValidationError::MessageTooLarge { .. }
        ))
    ));
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_validation_performance() {
    let engine = StreamValidationEngine::new();
    let context = create_test_context();
    let request = create_valid_import_request();

    let start = std::time::Instant::now();

    // Perform 1000 validations
    for _ in 0..1000 {
        let _ = engine.validate_stream_request(&request, &context).await;
    }

    let duration = start.elapsed();

    // Should complete 1000 validations in under 1 second
    assert!(
        duration < Duration::from_secs(1),
        "1000 validations took {:?}, expected under 1s",
        duration
    );

    // Average should be under 1ms per validation
    let avg_per_validation = duration / 1000;
    assert!(
        avg_per_validation < Duration::from_millis(1),
        "Average validation time {:?}, expected under 1ms",
        avg_per_validation
    );
}

#[test]
fn test_validation_performance_monitor() {
    let monitor = ValidationPerformanceMonitor::new();

    // Record some validations
    monitor.record_validation(Duration::from_millis(5), &Ok(()));
    monitor.record_validation(Duration::from_millis(3), &Ok(()));
    monitor.record_validation(
        Duration::from_millis(10),
        &Err(StreamValidationError::EmptyMessage),
    );

    let stats = monitor.get_performance_stats();

    assert_eq!(stats.total_validations, 3);
    assert!(stats.average_validation_time > Duration::from_millis(5));
    assert!(stats.average_validation_time < Duration::from_millis(7));
    assert_eq!(stats.error_counts.len(), 1);
    assert_eq!(stats.error_counts.get("EmptyMessage").unwrap_or(&0), &1);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_validation() {
    let engine = Arc::new(StreamValidationEngine::new());
    let context = create_test_context();

    let mut handles = vec![];

    // Launch 10 concurrent validation tasks
    for i in 0..10 {
        let engine_clone = engine.clone();
        let context_clone = context.clone();
        let mut request = create_valid_import_request();

        // Modify each request slightly
        if let Some(stream_request::Message::ImportAsset(ref mut req)) = request.message {
            req.asset_path = format!("Assets/Scripts/Test{}.cs", i);
        }

        let handle = tokio::spawn(async move {
            engine_clone
                .validate_stream_request(&request, &context_clone)
                .await
        });

        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[test]
fn test_validation_rules_metadata() {
    let import_validator = ImportAssetStreamValidator::new();
    let rules = import_validator.get_validation_rules();

    assert!(!rules.is_empty());

    // Check that we have required field validation
    let has_required_rule = rules
        .iter()
        .any(|rule| rule.rule_type == ValidationType::Required);
    assert!(has_required_rule);

    // Check that we have format validation
    let has_format_rule = rules
        .iter()
        .any(|rule| rule.rule_type == ValidationType::Format);
    assert!(has_format_rule);
}

#[tokio::test]
async fn test_validation_context_fields() {
    let engine = StreamValidationEngine::new();
    let mut context = create_test_context();
    let request = create_valid_import_request();

    // Test with minimal context
    context.client_info = None;
    assert!(engine
        .validate_stream_request(&request, &context)
        .await
        .is_ok());

    // Test with full context
    context.client_info = Some(ClientInfo {
        user_agent: Some("Unity Editor/2023.3.0f1".to_string()),
        ip_address: Some("192.168.1.100".to_string()),
        unity_version: Some("2023.3.0f1".to_string()),
    });
    assert!(engine
        .validate_stream_request(&request, &context)
        .await
        .is_ok());
}
