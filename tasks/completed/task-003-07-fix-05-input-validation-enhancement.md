# Task 3.7 Fix 05: 入力検証機能強化

## 概要
Task 3.7のストリーミングサービス実装で特定された入力検証不足を修正します。ストリーミングメッセージの構造的検証とサニタイゼーション機能を追加し、セキュリティとデータ整合性を向上させます。

## 優先度
**🟡 重要優先度** - セキュリティとデータ整合性に重要な影響

## 実装時間見積もり
**2-3時間** - 集中作業時間

## 受け入れ基準

### セキュリティ要件
- [ ] ストリーミングメッセージの包括的検証
- [ ] 入力データのサニタイゼーション機能
- [ ] 不正入力に対する適切な拒否処理
- [ ] セキュリティ脆弱性の排除

### データ整合性要件
- [ ] 構造的データ検証の実装
- [ ] 型安全性の確保
- [ ] 境界値チェックの実装
- [ ] データ形式の標準化

### パフォーマンス要件
- [ ] 検証処理による性能劣化の最小化
- [ ] 効率的な検証アルゴリズムの実装
- [ ] キャッシュ機能の活用

## 技術的詳細

### 現在の問題
現在、ストリーミングメッセージの検証は個別のRPCメソッド内でのみ行われており、ストリーミング特有の検証が不足しています。

### 検証アーキテクチャ

#### 1. 統一検証フレームワーク
```rust
/// Trait for validating different types of stream requests
pub trait StreamRequestValidator {
    type Request;
    type ValidationError;

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
    Error,    // 検証失敗でリクエストを拒否
    Warning,  // 警告を出すが処理続行
    Info,     // 情報記録のみ
}

#[derive(Debug, Clone)]
pub enum ValidationType {
    Required,
    Format,
    Range,
    Security,
    Business,
}
```

#### 2. ストリーミング専用検証エンジン
```rust
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
        let start_time = std::time::Instant::now();
        
        // 基本構造検証
        self.validate_basic_structure(stream_request)?;
        
        // セキュリティ検証
        self.security_rules.validate(stream_request, context).await?;
        
        // メッセージタイプ別検証
        let result = match &stream_request.message {
            Some(stream_request::Message::ImportAsset(req)) => {
                self.import_validator.validate(req)
                    .map_err(StreamValidationError::ImportAsset)
            }
            Some(stream_request::Message::MoveAsset(req)) => {
                self.move_validator.validate(req)
                    .map_err(StreamValidationError::MoveAsset)
            }
            Some(stream_request::Message::DeleteAsset(req)) => {
                self.delete_validator.validate(req)
                    .map_err(StreamValidationError::DeleteAsset)
            }
            Some(stream_request::Message::Refresh(req)) => {
                self.refresh_validator.validate(req)
                    .map_err(StreamValidationError::Refresh)
            }
            None => Err(StreamValidationError::EmptyMessage),
        };

        // パフォーマンス監視
        let validation_time = start_time.elapsed();
        self.performance_monitor.record_validation(validation_time, &result);
        
        result
    }

    pub async fn sanitize_stream_request(
        &self,
        mut stream_request: StreamRequest,
        context: &ValidationContext,
    ) -> Result<StreamRequest, StreamValidationError> {
        // メッセージタイプ別サニタイゼーション
        if let Some(ref mut message) = stream_request.message {
            match message {
                stream_request::Message::ImportAsset(ref mut req) => {
                    *req = self.import_validator.sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::ImportAsset)?;
                }
                stream_request::Message::MoveAsset(ref mut req) => {
                    *req = self.move_validator.sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::MoveAsset)?;
                }
                stream_request::Message::DeleteAsset(ref mut req) => {
                    *req = self.delete_validator.sanitize(std::mem::take(req))
                        .map_err(StreamValidationError::DeleteAsset)?;
                }
                stream_request::Message::Refresh(ref mut req) => {
                    *req = self.refresh_validator.sanitize(std::mem::take(req))
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
```

#### 3. 個別操作用検証器
```rust
/// Validator for ImportAsset stream requests
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
        self.path_validator.validate_asset_path(&request.asset_path)
            .map_err(ImportAssetValidationError::InvalidPath)?;

        // セキュリティ検証
        self.security_validator.validate_import_request(request)
            .map_err(ImportAssetValidationError::SecurityViolation)?;

        // ファイル拡張子検証
        self.validate_file_extension(&request.asset_path)?;

        // パス正規化検証
        self.validate_path_normalization(&request.asset_path)?;

        Ok(())
    }

    fn sanitize(&self, mut request: Self::Request) -> Result<Self::Request, Self::ValidationError> {
        // パス正規化
        request.asset_path = self.path_validator.normalize_path(&request.asset_path)
            .map_err(ImportAssetValidationError::InvalidPath)?;

        // 不要な空白除去
        request.asset_path = request.asset_path.trim().to_string();

        // パス区切り文字の統一
        request.asset_path = request.asset_path.replace('\\', "/");

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
            ValidationRule {
                name: "asset_path_security".to_string(),
                description: "Asset path must not contain path traversal patterns".to_string(),
                severity: ValidationSeverity::Error,
                rule_type: ValidationType::Security,
            },
            ValidationRule {
                name: "file_extension".to_string(),
                description: "Asset must have a valid file extension".to_string(),
                severity: ValidationSeverity::Warning,
                rule_type: ValidationType::Business,
            },
        ]
    }
}

impl ImportAssetStreamValidator {
    fn validate_file_extension(&self, asset_path: &str) -> Result<(), ImportAssetValidationError> {
        let allowed_extensions = [
            ".cs", ".js", ".boo",                    // Scripts
            ".png", ".jpg", ".jpeg", ".gif", ".bmp", // Textures
            ".fbx", ".obj", ".dae", ".3ds",          // Models
            ".prefab", ".unity",                     // Unity assets
            ".txt", ".json", ".xml", ".yaml",        // Data files
            ".wav", ".mp3", ".ogg",                  // Audio
            ".mp4", ".mov", ".avi",                  // Video
        ];

        let path_lower = asset_path.to_lowercase();
        let has_valid_extension = allowed_extensions
            .iter()
            .any(|ext| path_lower.ends_with(ext));

        if !has_valid_extension {
            return Err(ImportAssetValidationError::UnsupportedFileType {
                path: asset_path.to_string(),
                allowed_extensions: allowed_extensions.to_vec(),
            });
        }

        Ok(())
    }

    fn validate_path_normalization(&self, asset_path: &str) -> Result<(), ImportAssetValidationError> {
        // 連続スラッシュの検証
        if asset_path.contains("//") {
            return Err(ImportAssetValidationError::InvalidPathFormat(
                "Path contains consecutive slashes".to_string()
            ));
        }

        // 末尾スラッシュの検証
        if asset_path.ends_with('/') {
            return Err(ImportAssetValidationError::InvalidPathFormat(
                "Path should not end with slash".to_string()
            ));
        }

        // Unicode制御文字の検証
        if asset_path.chars().any(|c| c.is_control()) {
            return Err(ImportAssetValidationError::InvalidPathFormat(
                "Path contains control characters".to_string()
            ));
        }

        Ok(())
    }
}
```

#### 4. セキュリティ専用検証ルール
```rust
/// Security-focused validation rules
pub struct SecurityValidationRules {
    rate_limiter: Arc<RateLimiter>,
    blocked_patterns: Vec<regex::Regex>,
    max_path_length: usize,
    max_message_size: usize,
}

impl SecurityValidationRules {
    pub fn new() -> Self {
        let blocked_patterns = vec![
            regex::Regex::new(r"\.\.[\\/]").unwrap(),           // Path traversal
            regex::Regex::new(r"[<>:\"|?*]").unwrap(),          // Invalid filename chars
            regex::Regex::new(r"(?i)script:|javascript:").unwrap(), // Script injection
            regex::Regex::new(r"(?i)data:|vbscript:").unwrap(), // Data/VBScript URIs
        ];

        Self {
            rate_limiter: Arc::new(RateLimiter::new(100, std::time::Duration::from_secs(60))),
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
        // 実際のシリアライゼーションサイズを概算
        match &stream_request.message {
            Some(stream_request::Message::ImportAsset(req)) => {
                req.asset_path.len() + 32 // 基本オーバーヘッド
            }
            Some(stream_request::Message::MoveAsset(req)) => {
                req.src_path.len() + req.dst_path.len() + 32
            }
            Some(stream_request::Message::DeleteAsset(req)) => {
                req.asset_path.len() + 32
            }
            Some(stream_request::Message::Refresh(_)) => {
                32 // 基本オーバーヘッドのみ
            }
            None => 16,
        }
    }
}
```

#### 5. 検証コンテキストと結果
```rust
/// Context information for validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub client_id: String,
    pub connection_id: String,
    pub message_id: u64,
    pub timestamp: std::time::SystemTime,
    pub client_info: Option<ClientInfo>,
}

/// Comprehensive validation errors
#[derive(Debug, thiserror::Error)]
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

#[derive(Debug, thiserror::Error)]
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

/// Performance monitoring for validation
pub struct ValidationPerformanceMonitor {
    validation_times: Arc<Mutex<Vec<std::time::Duration>>>,
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
        duration: std::time::Duration,
        result: &Result<(), StreamValidationError>,
    ) {
        // パフォーマンス記録
        if let Ok(mut times) = self.validation_times.lock() {
            times.push(duration);
            
            // 直近1000件のみ保持
            if times.len() > 1000 {
                times.drain(..times.len() - 1000);
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

    pub fn get_performance_stats(&self) -> ValidationPerformanceStats {
        let times = self.validation_times.lock().unwrap();
        let error_counts = self.error_counts.lock().unwrap();
        
        let total_validations = times.len();
        let avg_time = if total_validations > 0 {
            times.iter().sum::<std::time::Duration>() / total_validations as u32
        } else {
            std::time::Duration::default()
        };
        
        ValidationPerformanceStats {
            total_validations,
            average_validation_time: avg_time,
            error_counts: error_counts.clone(),
        }
    }
}
```

## 統合と使用法

### StreamMessageProcessorへの統合
```rust
impl StreamMessageProcessor {
    pub async fn handle_stream_request_with_validation(
        &self,
        stream_request: StreamRequest,
        message_id: u64,
    ) -> StreamResponse {
        let context = ValidationContext {
            client_id: "unknown".to_string(), // 実装時にクライアント識別子を設定
            connection_id: self.connection_id.clone(),
            message_id,
            timestamp: std::time::SystemTime::now(),
            client_info: None,
        };

        // 検証実行
        match self.validation_engine.validate_stream_request(&stream_request, &context).await {
            Ok(_) => {
                // 検証成功 - サニタイゼーション実行
                match self.validation_engine.sanitize_stream_request(stream_request, &context).await {
                    Ok(sanitized_request) => {
                        // 正常処理続行
                        self.dispatch_message(sanitized_request.message.unwrap(), message_id).await
                    }
                    Err(sanitize_error) => {
                        // サニタイゼーションエラー
                        self.create_validation_error_response(sanitize_error, message_id)
                    }
                }
            }
            Err(validation_error) => {
                // 検証失敗
                self.create_validation_error_response(validation_error, message_id)
            }
        }
    }

    fn create_validation_error_response(
        &self,
        error: StreamValidationError,
        message_id: u64,
    ) -> StreamResponse {
        warn!(
            connection_id = %self.connection_id,
            message_id = message_id,
            error = %error,
            "Stream request validation failed"
        );

        UnityMcpServiceImpl::create_error_response(
            StreamErrorType::ValidationError,
            &format!("Request validation failed: {}", error),
            &format!("Connection: {} | Message: {} | Validation error details: {:?}",
                     self.connection_id, message_id, error),
            None,
        )
    }
}
```

## テスト方針

### 単体テスト
```rust
#[tokio::test]
async fn test_import_asset_validation() {
    // ImportAsset検証テスト
}

#[tokio::test]
async fn test_security_validation_rules() {
    // セキュリティルール検証テスト
}

#[tokio::test]
async fn test_input_sanitization() {
    // 入力サニタイゼーションテスト
}
```

### セキュリティテスト
```rust
#[tokio::test]
async fn test_path_traversal_prevention() {
    // パストラバーサル攻撃防止テスト
}

#[tokio::test]
async fn test_injection_attack_prevention() {
    // インジェクション攻撃防止テスト
}
```

## 実装計画

### Step 1: 検証フレームワークの基盤構築
### Step 2: 個別検証器の実装
### Step 3: セキュリティルールの実装
### Step 4: 統合とテスト
### Step 5: パフォーマンス最適化

## 成功基準

### セキュリティ
- 既知の攻撃パターンの100%ブロック
- 入力検証による脆弱性の排除

### 品質
- データ整合性の確保
- 適切なエラーメッセージの提供

### パフォーマンス
- 検証処理オーバーヘッド5ms以下
- システム全体への性能影響最小化