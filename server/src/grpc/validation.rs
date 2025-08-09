//! Stream request validation module
//!
//! This module provides comprehensive validation and sanitization for all streaming requests,
//! ensuring security, data integrity, and proper request formatting.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use regex::Regex;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::error;

use crate::grpc::{
    stream_request, DeleteAssetRequest, ImportAssetRequest, MoveAssetRequest, RefreshRequest,
    StreamRequest,
};

// ============================================================================
// Core Validation Framework
// ============================================================================

/// Trait for validating different types of stream requests
pub trait StreamRequestValidator {
    type Request;
    type ValidationError: std::error::Error + Send + Sync + 'static;

    fn validate(&self, request: &Self::Request) -> Result<(), Self::ValidationError>;
    fn sanitize(&self, request: Self::Request) -> Result<Self::Request, Self::ValidationError>;
    fn get_validation_rules(&self) -> Vec<ValidationRule>;
}

/// Validation rule definition
#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub name: String,
    pub description: String,
    pub severity: ValidationSeverity,
    pub rule_type: ValidationType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationSeverity {
    Error,   // 検証失敗でリクエストを拒否
    Warning, // 警告を出すが処理続行
    Info,    // 情報記録のみ
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationType {
    Required,
    Format,
    Range,
    Security,
    Business,
}

/// Context information for validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub client_id: String,
    pub connection_id: String,
    pub message_id: u64,
    pub timestamp: SystemTime,
    pub client_info: Option<ClientInfo>,
}

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub unity_version: Option<String>,
}

// ============================================================================
// Validation Errors
// ============================================================================

/// Comprehensive validation errors
#[derive(Debug, Error)]
pub enum StreamValidationError {
    #[error("Empty message in stream request")]
    EmptyMessage,

    #[error("Import asset validation error: {0}")]
    ImportAsset(ImportAssetValidationError),

    #[error("Move asset validation error: {0}")]
    MoveAsset(MoveAssetValidationError),

    #[error("Delete asset validation error: {0}")]
    DeleteAsset(DeleteAssetValidationError),

    #[error("Refresh validation error: {0}")]
    Refresh(RefreshValidationError),

    #[error("Security validation error: {0}")]
    Security(SecurityValidationError),
}

#[derive(Debug, Error)]
pub enum ImportAssetValidationError {
    #[error("Asset path cannot be empty")]
    EmptyAssetPath,

    #[error("Invalid asset path: {0}")]
    InvalidPath(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Unsupported file type for path '{path}'. Allowed extensions: {allowed_extensions:?}")]
    UnsupportedFileType {
        path: String,
        allowed_extensions: Vec<&'static str>,
    },

    #[error("Invalid path format: {0}")]
    InvalidPathFormat(String),
}

#[derive(Debug, Error)]
pub enum MoveAssetValidationError {
    #[error("Source path cannot be empty")]
    EmptySourcePath,

    #[error("Destination path cannot be empty")]
    EmptyDestinationPath,

    #[error("Invalid source path: {0}")]
    InvalidSourcePath(String),

    #[error("Invalid destination path: {0}")]
    InvalidDestinationPath(String),

    #[error("Source and destination paths cannot be the same")]
    SameSourceAndDestination,

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[derive(Debug, Error)]
pub enum DeleteAssetValidationError {
    #[error("Asset path cannot be empty")]
    EmptyAssetPath,

    #[error("Invalid asset path: {0}")]
    InvalidPath(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[derive(Debug, Error)]
pub enum RefreshValidationError {
    #[error("Security violation: {0}")]
    SecurityViolation(String),
}

#[derive(Debug, Error)]
pub enum SecurityValidationError {
    #[error("Rate limit exceeded for client {client_id}")]
    RateLimitExceeded { client_id: String },

    #[error("Message too large: {size} bytes (max: {max_size} bytes)")]
    MessageTooLarge { size: usize, max_size: usize },

    #[error("Path too long: '{path}' (max length: {max_length})")]
    PathTooLong { path: String, max_length: usize },

    #[error("Malicious pattern detected in path '{path}': {pattern}")]
    MaliciousPattern { path: String, pattern: String },
}

// ============================================================================
// Main Validation Engine
// ============================================================================

/// Main validation engine for stream requests
pub struct StreamValidationEngine {
    import_validator: ImportAssetStreamValidator,
    move_validator: MoveAssetStreamValidator,
    delete_validator: DeleteAssetStreamValidator,
    refresh_validator: RefreshStreamValidator,
    security_rules: SecurityValidationRules,
    performance_monitor: ValidationPerformanceMonitor,
}

impl StreamValidationEngine {
    pub fn new() -> Self {
        Self {
            import_validator: ImportAssetStreamValidator::new(),
            move_validator: MoveAssetStreamValidator::new(),
            delete_validator: DeleteAssetStreamValidator::new(),
            refresh_validator: RefreshStreamValidator::new(),
            security_rules: SecurityValidationRules::new(),
            performance_monitor: ValidationPerformanceMonitor::new(),
        }
    }

    pub async fn validate_stream_request(
        &self,
        stream_request: &StreamRequest,
        context: &ValidationContext,
    ) -> Result<(), StreamValidationError> {
        let start_time = Instant::now();

        // 基本構造検証
        self.validate_basic_structure(stream_request)?;

        // セキュリティ検証
        self.security_rules
            .validate(stream_request, context)
            .await
            .map_err(StreamValidationError::Security)?;

        // メッセージタイプ別検証
        let result = match &stream_request.message {
            Some(stream_request::Message::ImportAsset(req)) => self
                .import_validator
                .validate(req)
                .map_err(StreamValidationError::ImportAsset),
            Some(stream_request::Message::MoveAsset(req)) => self
                .move_validator
                .validate(req)
                .map_err(StreamValidationError::MoveAsset),
            Some(stream_request::Message::DeleteAsset(req)) => self
                .delete_validator
                .validate(req)
                .map_err(StreamValidationError::DeleteAsset),
            Some(stream_request::Message::Refresh(req)) => self
                .refresh_validator
                .validate(req)
                .map_err(StreamValidationError::Refresh),
            None => Err(StreamValidationError::EmptyMessage),
        };

        // パフォーマンス監視
        let validation_time = start_time.elapsed();
        self.performance_monitor
            .record_validation(validation_time, &result);

        result
    }

    pub async fn sanitize_stream_request(
        &self,
        mut stream_request: StreamRequest,
        _context: &ValidationContext,
    ) -> Result<StreamRequest, StreamValidationError> {
        // メッセージタイプ別サニタイゼーション
        if let Some(ref mut message) = stream_request.message {
            match message {
                stream_request::Message::ImportAsset(ref mut req) => {
                    *req = self
                        .import_validator
                        .sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::ImportAsset)?;
                }
                stream_request::Message::MoveAsset(ref mut req) => {
                    *req = self
                        .move_validator
                        .sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::MoveAsset)?;
                }
                stream_request::Message::DeleteAsset(ref mut req) => {
                    *req = self
                        .delete_validator
                        .sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::DeleteAsset)?;
                }
                stream_request::Message::Refresh(ref mut req) => {
                    *req = self
                        .refresh_validator
                        .sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::Refresh)?;
                }
            }
        }

        Ok(stream_request)
    }

    fn validate_basic_structure(
        &self,
        stream_request: &StreamRequest,
    ) -> Result<(), StreamValidationError> {
        if stream_request.message.is_none() {
            return Err(StreamValidationError::EmptyMessage);
        }
        Ok(())
    }
}

impl Default for StreamValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Asset Path Validator (Shared utility)
// ============================================================================

pub struct AssetPathValidator {
    max_path_length: usize,
}

impl AssetPathValidator {
    pub fn new() -> Self {
        Self {
            max_path_length: 260,
        }
    }

    pub fn validate_asset_path(&self, path: &str) -> Result<(), String> {
        if path.trim().is_empty() {
            return Err("Path cannot be empty".to_string());
        }

        // Unity Asset path format check - accept both forward and backslash formats
        if !path.starts_with("Assets/") && !path.starts_with("Assets\\") {
            return Err("Path must start with 'Assets/' or 'Assets\\'".to_string());
        }

        // Path traversal check
        if path.contains("../") || path.contains("..\\") {
            return Err("Path traversal not allowed".to_string());
        }

        // Invalid characters check
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        if path.chars().any(|c| invalid_chars.contains(&c)) {
            return Err("Path contains invalid characters".to_string());
        }

        // Length check
        if path.len() > self.max_path_length {
            return Err(format!(
                "Path exceeds maximum length of {} characters",
                self.max_path_length
            ));
        }

        Ok(())
    }

    pub fn normalize_path(&self, path: &str) -> Result<String, String> {
        // パス区切り文字の統一
        let normalized = path.replace('\\', "/");

        // 連続スラッシュの除去
        let normalized = normalized
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("/");

        // 先頭にAssets/を確保
        let normalized = if normalized.starts_with("Assets/") {
            normalized
        } else {
            format!("Assets/{}", normalized.trim_start_matches('/'))
        };

        Ok(normalized)
    }
}

impl Default for AssetPathValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Security Validator (Shared utility)
// ============================================================================

pub struct SecurityValidator {
    blocked_patterns: Vec<Regex>,
}

impl SecurityValidator {
    pub fn new() -> Self {
        let blocked_patterns = vec![
            Regex::new(r"\.\.[\\/]").unwrap(),               // Path traversal
            Regex::new(r"[<>:|?*]").unwrap(), // Invalid filename chars (not including backslash)
            Regex::new(r"(?i)script:|javascript:").unwrap(), // Script injection
            Regex::new(r"(?i)data:|vbscript:").unwrap(), // Data/VBScript URIs
            Regex::new(r"[\x00-\x1F\x7F]").unwrap(), // Control characters
        ];

        Self { blocked_patterns }
    }

    pub fn validate_import_request(&self, request: &ImportAssetRequest) -> Result<(), String> {
        self.validate_path(&request.asset_path)?;
        Ok(())
    }

    pub fn validate_move_request(&self, request: &MoveAssetRequest) -> Result<(), String> {
        self.validate_path(&request.src_path)?;
        self.validate_path(&request.dst_path)?;
        Ok(())
    }

    pub fn validate_delete_request(&self, request: &DeleteAssetRequest) -> Result<(), String> {
        self.validate_path(&request.asset_path)?;
        Ok(())
    }

    fn validate_path(&self, path: &str) -> Result<(), String> {
        for pattern in &self.blocked_patterns {
            if pattern.is_match(path) {
                return Err("Security violation: path matches blocked pattern".to_string());
            }
        }
        Ok(())
    }
}

impl Default for SecurityValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Rate Limiter
// ============================================================================

pub struct RateLimiter {
    requests_per_window: usize,
    window_duration: Duration,
    client_windows: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new(requests_per_window: usize, window_duration: Duration) -> Self {
        Self {
            requests_per_window,
            window_duration,
            client_windows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn is_allowed(&self, client_id: &str) -> bool {
        let now = Instant::now();
        let mut windows = self.client_windows.write().await;

        let client_requests = windows
            .entry(client_id.to_string())
            .or_insert_with(Vec::new);

        // 古いリクエストを削除
        client_requests.retain(|&timestamp| now.duration_since(timestamp) < self.window_duration);

        // リクエスト数チェック
        if client_requests.len() >= self.requests_per_window {
            false
        } else {
            client_requests.push(now);
            true
        }
    }
}

// ============================================================================
// Performance Monitor
// ============================================================================

/// Performance monitoring for validation
pub struct ValidationPerformanceMonitor {
    validation_times: Arc<Mutex<Vec<Duration>>>,
    error_counts: Arc<Mutex<HashMap<String, u64>>>,
}

impl ValidationPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            validation_times: Arc::new(Mutex::new(Vec::new())),
            error_counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn record_validation(
        &self,
        duration: Duration,
        result: &Result<(), StreamValidationError>,
    ) {
        // パフォーマンス記録
        if let Ok(mut times) = self.validation_times.lock() {
            times.push(duration);

            // 直近1000件のみ保持
            if times.len() > 1000 {
                let excess = times.len() - 1000;
                times.drain(..excess);
            }
        }

        // エラー統計記録
        if let Err(error) = result {
            let error_type = format!("{:?}", error);
            if let Ok(mut counts) = self.error_counts.lock() {
                *counts.entry(error_type).or_insert(0) += 1;
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_performance_stats(&self) -> ValidationPerformanceStats {
        let (total_validations, avg_time) = {
            let times = self.validation_times.lock().unwrap();
            let total_validations = times.len();
            let avg_time = if total_validations > 0 {
                times.iter().sum::<Duration>() / total_validations as u32
            } else {
                Duration::default()
            };
            (total_validations, avg_time)
        };

        let error_counts = self.error_counts.lock().unwrap();

        ValidationPerformanceStats {
            total_validations,
            average_validation_time: avg_time,
            error_counts: error_counts.clone(),
        }
    }
}

impl Default for ValidationPerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ValidationPerformanceStats {
    pub total_validations: usize,
    pub average_validation_time: Duration,
    pub error_counts: HashMap<String, u64>,
}

// ============================================================================
// Security Validation Rules
// ============================================================================

/// Security-focused validation rules
pub struct SecurityValidationRules {
    rate_limiter: Arc<RateLimiter>,
    blocked_patterns: Vec<Regex>,
    max_path_length: usize,
    max_message_size: usize,
}

impl SecurityValidationRules {
    pub fn new() -> Self {
        let blocked_patterns = vec![
            Regex::new(r"\.\.[\\/]").unwrap(),               // Path traversal
            Regex::new(r"[<>:|?*]").unwrap(),                // Invalid filename chars
            Regex::new(r"(?i)script:|javascript:").unwrap(), // Script injection
            Regex::new(r"(?i)data:|vbscript:").unwrap(),     // Data/VBScript URIs
        ];

        Self {
            rate_limiter: Arc::new(RateLimiter::new(100, Duration::from_secs(60))),
            blocked_patterns,
            max_path_length: 260,
            max_message_size: 64 * 1024, // 64KB
        }
    }

    pub async fn validate(
        &self,
        stream_request: &StreamRequest,
        context: &ValidationContext,
    ) -> Result<(), SecurityValidationError> {
        // レート制限チェック
        if !self.rate_limiter.is_allowed(&context.client_id).await {
            return Err(SecurityValidationError::RateLimitExceeded {
                client_id: context.client_id.clone(),
            });
        }

        // メッセージサイズチェック
        let message_size = self.estimate_message_size(stream_request);
        if message_size > self.max_message_size {
            return Err(SecurityValidationError::MessageTooLarge {
                size: message_size,
                max_size: self.max_message_size,
            });
        }

        // パターンベースセキュリティチェック
        self.validate_security_patterns(stream_request)?;

        Ok(())
    }

    fn validate_security_patterns(
        &self,
        stream_request: &StreamRequest,
    ) -> Result<(), SecurityValidationError> {
        if let Some(ref message) = stream_request.message {
            let paths_to_check = self.extract_paths_from_message(message);

            for path in paths_to_check {
                // パス長チェック
                if path.len() > self.max_path_length {
                    return Err(SecurityValidationError::PathTooLong {
                        path: path.clone(),
                        max_length: self.max_path_length,
                    });
                }

                // 悪意のあるパターンチェック
                for pattern in &self.blocked_patterns {
                    if pattern.is_match(&path) {
                        return Err(SecurityValidationError::MaliciousPattern {
                            path: path.clone(),
                            pattern: pattern.as_str().to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_paths_from_message(&self, message: &stream_request::Message) -> Vec<String> {
        match message {
            stream_request::Message::ImportAsset(req) => {
                vec![req.asset_path.clone()]
            }
            stream_request::Message::MoveAsset(req) => {
                vec![req.src_path.clone(), req.dst_path.clone()]
            }
            stream_request::Message::DeleteAsset(req) => {
                vec![req.asset_path.clone()]
            }
            stream_request::Message::Refresh(_) => {
                vec![]
            }
        }
    }

    fn estimate_message_size(&self, stream_request: &StreamRequest) -> usize {
        // プロトコルバッファのサイズ概算
        match &stream_request.message {
            Some(stream_request::Message::ImportAsset(req)) => {
                req.asset_path.len() + 32 // 基本オーバーヘッド
            }
            Some(stream_request::Message::MoveAsset(req)) => {
                req.src_path.len() + req.dst_path.len() + 32
            }
            Some(stream_request::Message::DeleteAsset(req)) => req.asset_path.len() + 32,
            Some(stream_request::Message::Refresh(_)) => {
                32 // 基本オーバーヘッドのみ
            }
            None => 16,
        }
    }
}

impl Default for SecurityValidationRules {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Individual Operation Validators
// ============================================================================

// ImportAssetStreamValidator
pub struct ImportAssetStreamValidator {
    path_validator: AssetPathValidator,
    security_validator: SecurityValidator,
}

impl ImportAssetStreamValidator {
    pub fn new() -> Self {
        Self {
            path_validator: AssetPathValidator::new(),
            security_validator: SecurityValidator::new(),
        }
    }
}

impl StreamRequestValidator for ImportAssetStreamValidator {
    type Request = ImportAssetRequest;
    type ValidationError = ImportAssetValidationError;

    fn validate(&self, request: &Self::Request) -> Result<(), Self::ValidationError> {
        // 必須フィールド検証
        if request.asset_path.trim().is_empty() {
            return Err(ImportAssetValidationError::EmptyAssetPath);
        }

        // パス検証
        self.path_validator
            .validate_asset_path(&request.asset_path)
            .map_err(ImportAssetValidationError::InvalidPath)?;

        // セキュリティ検証
        self.security_validator
            .validate_import_request(request)
            .map_err(ImportAssetValidationError::SecurityViolation)?;

        Ok(())
    }

    fn sanitize(&self, mut request: Self::Request) -> Result<Self::Request, Self::ValidationError> {
        // パス正規化
        request.asset_path = self
            .path_validator
            .normalize_path(&request.asset_path)
            .map_err(ImportAssetValidationError::InvalidPath)?;

        // 不要な空白除去
        request.asset_path = request.asset_path.trim().to_string();

        Ok(request)
    }

    fn get_validation_rules(&self) -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                name: "asset_path_required".to_string(),
                description: "Asset path must not be empty".to_string(),
                severity: ValidationSeverity::Error,
                rule_type: ValidationType::Required,
            },
            ValidationRule {
                name: "asset_path_format".to_string(),
                description: "Asset path must follow Unity format (Assets/...)".to_string(),
                severity: ValidationSeverity::Error,
                rule_type: ValidationType::Format,
            },
        ]
    }
}

impl Default for ImportAssetStreamValidator {
    fn default() -> Self {
        Self::new()
    }
}

// MoveAssetStreamValidator
pub struct MoveAssetStreamValidator {
    path_validator: AssetPathValidator,
    security_validator: SecurityValidator,
}

impl MoveAssetStreamValidator {
    pub fn new() -> Self {
        Self {
            path_validator: AssetPathValidator::new(),
            security_validator: SecurityValidator::new(),
        }
    }
}

impl StreamRequestValidator for MoveAssetStreamValidator {
    type Request = MoveAssetRequest;
    type ValidationError = MoveAssetValidationError;

    fn validate(&self, request: &Self::Request) -> Result<(), Self::ValidationError> {
        if request.src_path.trim().is_empty() {
            return Err(MoveAssetValidationError::EmptySourcePath);
        }
        if request.dst_path.trim().is_empty() {
            return Err(MoveAssetValidationError::EmptyDestinationPath);
        }
        if request.src_path == request.dst_path {
            return Err(MoveAssetValidationError::SameSourceAndDestination);
        }

        self.path_validator
            .validate_asset_path(&request.src_path)
            .map_err(MoveAssetValidationError::InvalidSourcePath)?;
        self.path_validator
            .validate_asset_path(&request.dst_path)
            .map_err(MoveAssetValidationError::InvalidDestinationPath)?;

        self.security_validator
            .validate_move_request(request)
            .map_err(MoveAssetValidationError::SecurityViolation)?;

        Ok(())
    }

    fn sanitize(&self, mut request: Self::Request) -> Result<Self::Request, Self::ValidationError> {
        request.src_path = self
            .path_validator
            .normalize_path(&request.src_path)
            .map_err(MoveAssetValidationError::InvalidSourcePath)?;
        request.dst_path = self
            .path_validator
            .normalize_path(&request.dst_path)
            .map_err(MoveAssetValidationError::InvalidDestinationPath)?;

        Ok(request)
    }

    fn get_validation_rules(&self) -> Vec<ValidationRule> {
        vec![
            ValidationRule {
                name: "src_path_required".to_string(),
                description: "Source path must not be empty".to_string(),
                severity: ValidationSeverity::Error,
                rule_type: ValidationType::Required,
            },
            ValidationRule {
                name: "dst_path_required".to_string(),
                description: "Destination path must not be empty".to_string(),
                severity: ValidationSeverity::Error,
                rule_type: ValidationType::Required,
            },
        ]
    }
}

impl Default for MoveAssetStreamValidator {
    fn default() -> Self {
        Self::new()
    }
}

// DeleteAssetStreamValidator
pub struct DeleteAssetStreamValidator {
    path_validator: AssetPathValidator,
    security_validator: SecurityValidator,
}

impl DeleteAssetStreamValidator {
    pub fn new() -> Self {
        Self {
            path_validator: AssetPathValidator::new(),
            security_validator: SecurityValidator::new(),
        }
    }
}

impl StreamRequestValidator for DeleteAssetStreamValidator {
    type Request = DeleteAssetRequest;
    type ValidationError = DeleteAssetValidationError;

    fn validate(&self, request: &Self::Request) -> Result<(), Self::ValidationError> {
        if request.asset_path.trim().is_empty() {
            return Err(DeleteAssetValidationError::EmptyAssetPath);
        }

        self.path_validator
            .validate_asset_path(&request.asset_path)
            .map_err(DeleteAssetValidationError::InvalidPath)?;

        self.security_validator
            .validate_delete_request(request)
            .map_err(DeleteAssetValidationError::SecurityViolation)?;

        Ok(())
    }

    fn sanitize(&self, mut request: Self::Request) -> Result<Self::Request, Self::ValidationError> {
        request.asset_path = self
            .path_validator
            .normalize_path(&request.asset_path)
            .map_err(DeleteAssetValidationError::InvalidPath)?;

        Ok(request)
    }

    fn get_validation_rules(&self) -> Vec<ValidationRule> {
        vec![ValidationRule {
            name: "asset_path_required".to_string(),
            description: "Asset path must not be empty".to_string(),
            severity: ValidationSeverity::Error,
            rule_type: ValidationType::Required,
        }]
    }
}

impl Default for DeleteAssetStreamValidator {
    fn default() -> Self {
        Self::new()
    }
}

// RefreshStreamValidator
pub struct RefreshStreamValidator;

impl RefreshStreamValidator {
    pub fn new() -> Self {
        Self
    }
}

impl StreamRequestValidator for RefreshStreamValidator {
    type Request = RefreshRequest;
    type ValidationError = RefreshValidationError;

    fn validate(&self, _request: &Self::Request) -> Result<(), Self::ValidationError> {
        // RefreshRequestには特別な検証は不要
        Ok(())
    }

    fn sanitize(&self, request: Self::Request) -> Result<Self::Request, Self::ValidationError> {
        // RefreshRequestには特別なサニタイゼーションは不要
        Ok(request)
    }

    fn get_validation_rules(&self) -> Vec<ValidationRule> {
        vec![ValidationRule {
            name: "refresh_allowed".to_string(),
            description: "Refresh operation is always allowed".to_string(),
            severity: ValidationSeverity::Info,
            rule_type: ValidationType::Business,
        }]
    }
}

impl Default for RefreshStreamValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

// Re-export test utilities for external use
#[cfg(test)]
pub use tests::*;
