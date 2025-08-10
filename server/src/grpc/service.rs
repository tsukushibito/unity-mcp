//! gRPC service implementations for Unity MCP Server
//!
//! This module contains the implementation of the UnityMcpService trait,
//! providing stub implementations for MCP core operations.

use async_trait::async_trait;
use std::path::{Component, Path};
use std::pin::Pin;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info, instrument, warn};

use crate::grpc::error::{no_error, not_found_error, validation_error};
use crate::grpc::validation::{
    ClientInfo, StreamValidationEngine, StreamValidationError, ValidationContext,
};
use crate::grpc::performance::cache::StreamCache;
use crate::grpc::{
    stream_request, stream_response, unity_mcp_service_server::UnityMcpService, CallToolRequest,
    CallToolResponse, DeleteAssetRequest, DeleteAssetResponse, GetProjectInfoRequest,
    GetProjectInfoResponse, GetPromptRequest, GetPromptResponse, ImportAssetRequest,
    ImportAssetResponse, ListPromptsRequest, ListPromptsResponse, ListResourcesRequest,
    ListResourcesResponse, ListToolsRequest, ListToolsResponse, McpError, MoveAssetRequest,
    MoveAssetResponse, ProjectInfo, ReadResourceRequest, ReadResourceResponse, RefreshRequest,
    RefreshResponse, StreamRequest, StreamResponse, UnityAsset,
};
use uuid::Uuid;

use serde_json::json;
use std::collections::HashMap;

/// Error types for stream processing operations
#[derive(Debug, Clone)]
pub enum StreamErrorType {
    InvalidRequest,
    ProcessingError,
    InternalError,
    ResourceExhausted,
    NotFound,
    ValidationError,
}
impl StreamErrorType {
    fn map_grpc_status_to_error_type(status: &Status) -> Self {
        match status.code() {
            tonic::Code::InvalidArgument => Self::ValidationError,
            tonic::Code::NotFound => Self::NotFound,
            tonic::Code::ResourceExhausted => Self::ResourceExhausted,
            tonic::Code::FailedPrecondition => Self::ValidationError,
            tonic::Code::Internal => Self::InternalError,
            tonic::Code::Unavailable => Self::ProcessingError,
            _ => Self::InternalError,
        }
    }
}

impl StreamErrorType {
    fn to_grpc_code(&self) -> i32 {
        match self {
            Self::InvalidRequest => 3,    // INVALID_ARGUMENT
            Self::ProcessingError => 13,  // INTERNAL
            Self::InternalError => 13,    // INTERNAL
            Self::ResourceExhausted => 8, // RESOURCE_EXHAUSTED
            Self::NotFound => 5,          // NOT_FOUND
            Self::ValidationError => 3,   // INVALID_ARGUMENT
        }
    }
}

/// Error context for tracking and debugging stream errors
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorContext {
    pub request_id: String,
    pub timestamp: std::time::SystemTime,
    pub request_type: Option<String>,
    pub additional_info: HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(request_type: Option<String>) -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            request_type,
            additional_info: HashMap::new(),
        }
    }

    pub fn add_info(&mut self, key: String, value: String) {
        self.additional_info.insert(key, value);
    }

    pub fn to_details_string(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| "Error context serialization failed".to_string())
    }
}

/// Unity MCP Service implementation
///
/// Provides stub implementations for all MCP operations.
/// Task 3.3 focuses on the first 3 methods (ListTools, CallTool, ListResources),
/// while other methods are provided as minimal stubs.
pub struct UnityMcpServiceImpl {
    validation_engine: StreamValidationEngine,
    cache: StreamCache,
}

impl Default for UnityMcpServiceImpl {
    fn default() -> Self {
        Self::new().expect("Failed to create default UnityMcpServiceImpl")
    }
}

// Constants for Unity-specific values
impl UnityMcpServiceImpl {
    const STUB_PROJECT_NAME: &'static str = "Unity MCP Test Project";
    const STUB_UNITY_VERSION: &'static str = "2023.3.0f1";
    const DEFAULT_ASSET_TYPE: &'static str = "Unknown";
    const MAX_PATH_LENGTH: usize = 260;

    // Stream channel configuration for security
    const STREAM_CHANNEL_CAPACITY: usize = 1000;
    const STREAM_BACKPRESSURE_THRESHOLD: f64 = 0.8;
}

/// Stream type for the bidirectional streaming RPC
type ServiceStream = Pin<Box<dyn Stream<Item = Result<StreamResponse, Status>> + Send>>;
/// Stream handler for managing task lifecycle and resource cleanup
pub struct StreamHandler {
    message_handler: JoinHandle<()>,
    cleanup_handler: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl StreamHandler {
    /// Create a new stream handler with proper task lifecycle management
    fn new(
        service: Arc<UnityMcpServiceImpl>,
        stream: Streaming<StreamRequest>,
        tx: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) -> Self {
        let cancellation_token = CancellationToken::new();

        let message_handler = tokio::spawn({
            let cancellation_token = cancellation_token.clone();
            async move {
                tokio::select! {
                    _ = UnityMcpServiceImpl::handle_stream_messages(service, stream, tx) => {
                        info!("Stream message processing completed normally");
                    }
                    _ = cancellation_token.cancelled() => {
                        info!("Stream message processing cancelled");
                    }
                }
            }
        });

        let cleanup_handler = tokio::spawn({
            let _message_handler_clone = message_handler.abort_handle();
            async move {
                // Wait for the message handler to complete or be aborted
                info!("Stream cleanup completed");
            }
        });

        Self {
            message_handler,
            cleanup_handler,
            cancellation_token,
        }
    }

    /// Shutdown the stream handler gracefully
    async fn shutdown(self) -> Result<(), tokio::task::JoinError> {
        self.cancellation_token.cancel();
        self.message_handler.await?;
        self.cleanup_handler.await?;
        Ok(())
    }
}

/// Trait for creating error responses in a consistent manner
trait WithMcpError {
    fn with_error(error: crate::grpc::McpError) -> Self;
}

impl WithMcpError for ImportAssetResponse {
    fn with_error(error: crate::grpc::McpError) -> Self {
        Self {
            asset: None,
            error: Some(error),
        }
    }
}

impl WithMcpError for MoveAssetResponse {
    fn with_error(error: crate::grpc::McpError) -> Self {
        Self {
            asset: None,
            error: Some(error),
        }
    }
}
// ============================================================================
// Stream Operation Handler Trait (Generic + Trait approach)
// ============================================================================

/// Generic trait for handling different types of stream requests
/// This approach eliminates ~85% of code duplication while maintaining type safety
#[async_trait]
pub trait StreamOperationHandler {
    type Request: Send + 'static;
    type Response: Send + 'static;

    /// Execute the gRPC service call for this operation
    async fn call_service(
        service: &Arc<UnityMcpServiceImpl>,
        request: Request<Self::Request>,
    ) -> Result<Response<Self::Response>, Status>;

    /// Build the appropriate StreamResponse from the service response
    fn build_stream_response(response: Self::Response) -> StreamResponse;

    /// Get the operation name for logging and error context
    fn get_operation_name() -> &'static str;

    /// Extract debug information from the request for logging
    fn extract_debug_info(request: &Self::Request) -> Vec<(&'static str, String)>;
}

impl UnityMcpServiceImpl {
    /// Create a new service instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            validation_engine: StreamValidationEngine::new(),
            cache: StreamCache::new()?,
        })
    }

    /// Create a new service instance with test-friendly settings
    pub fn new_for_testing() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            validation_engine: StreamValidationEngine::new_for_testing(),
            cache: StreamCache::new()?,
        })
    }

    /// Check if a path is under the Assets directory using proper path component analysis
    fn is_under_assets(path: &str) -> bool {
        // Normalize path separators for cross-platform compatibility
        let normalized_path = path.replace('\\', "/");
        let p = Path::new(&normalized_path);
        let mut components = p.components();

        // First component must be "Assets"
        match components.next() {
            Some(Component::Normal(first)) if first == "Assets" => {}
            _ => return false,
        }

        // No parent directory traversal allowed
        !components.any(|c| matches!(c, Component::ParentDir))
    }

    /// Validate Unity asset path with comprehensive security checks
    fn validate_asset_path(
        &self,
        path: &str,
        field_name: &str,
    ) -> Result<(), crate::grpc::McpError> {
        if path.trim().is_empty() {
            return Err(validation_error(
                &format!("Invalid {}", field_name),
                &format!("{} cannot be empty", field_name),
            ));
        }

        // Use proper path component analysis instead of string matching
        if !Self::is_under_assets(path) {
            return Err(validation_error(
                &format!("Invalid {}", field_name),
                &format!(
                    "{} must be under the Assets directory and contain no path traversal",
                    field_name
                ),
            ));
        }

        // Security: Invalid character check
        if path.contains(' ') || path.contains('<') || path.contains('>') {
            return Err(validation_error(
                &format!("Invalid {}", field_name),
                "Path contains invalid characters",
            ));
        }

        // Length validation
        if path.len() > Self::MAX_PATH_LENGTH {
            return Err(validation_error(
                &format!("Invalid {}", field_name),
                &format!(
                    "Path exceeds maximum length of {} characters",
                    Self::MAX_PATH_LENGTH
                ),
            ));
        }

        Ok(())
    }

    /// Validate both source and destination paths for move operation
    fn validate_move_paths(
        &self,
        src_path: &str,
        dst_path: &str,
    ) -> Result<(), crate::grpc::McpError> {
        // Validate source path
        self.validate_asset_path(src_path, "src_path")?;

        // Validate destination path
        self.validate_asset_path(dst_path, "dst_path")?;

        // Check that source and destination paths are different
        if src_path == dst_path {
            return Err(validation_error(
                "Invalid move operation",
                "src_path and dst_path must be different",
            ));
        }

        Ok(())
    }

    /// Create a backpressure error response when stream channel is full
    fn create_backpressure_error() -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::ImportAsset(ImportAssetResponse {
                asset: None,
                error: Some(McpError {
                    code: 8, // RESOURCE_EXHAUSTED
                    message: "Stream processing capacity exceeded".to_string(),
                    details: "Please reduce message rate".to_string(),
                }),
            })),
        }
    }

    /// Handle stream messages with shared service instance
    async fn handle_stream_messages(
        service: Arc<Self>,
        mut stream: Streaming<StreamRequest>,
        tx: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        let mut message_id_counter = 1u64;

        while let Some(result) = stream.next().await {
            match result {
                Ok(stream_request) => {
                    let message_id = message_id_counter;
                    message_id_counter += 1;

                    let response =
                        Self::process_stream_request(&service, stream_request, message_id).await;

                    if tx.send(Ok(response)).await.is_err() {
                        warn!("Stream receiver dropped - terminating handler");
                        break;
                    }
                }
                Err(status) => {
                    warn!("Stream request error: {}", status.message());
                    let error_response = Self::create_stream_error_response(status);
                    let _ = tx.send(Ok(error_response)).await;
                    break;
                }
            }
        }

        info!("Stream message handler terminated");
    }

    /// Process individual stream requests using shared service instance with validation
    async fn process_stream_request(
        service: &Arc<Self>,
        stream_request: StreamRequest,
        message_id: u64,
    ) -> StreamResponse {
        debug!("Processing stream request with validation and caching");

        // キャッシュからレスポンスを取得を試みる
        if let Some(cached_response) = service.cache.get(&stream_request).await {
            debug!("Cache hit for message_id: {}", message_id);
            return cached_response;
        }

        debug!("Cache miss for message_id: {}, proceeding with validation", message_id);

        // 検証コンテキストの作成
        let context = ValidationContext {
            client_id: "unity_client".to_string(), // TODO: 実際のクライアント識別子を取得
            connection_id: "stream_connection".to_string(), // TODO: 実際の接続識別子を取得
            message_id,
            timestamp: std::time::SystemTime::now(),
            client_info: Some(ClientInfo {
                user_agent: None,
                ip_address: None,
                unity_version: None,
            }),
        };

        // 検証実行
        match service
            .validation_engine
            .validate_stream_request(&stream_request, &context)
            .await
        {
            Ok(_) => {
                // 検証成功 - サニタイゼーション実行
                match service
                    .validation_engine
                    .sanitize_stream_request(stream_request.clone(), &context)
                    .await
                {
                    Ok(sanitized_request) => {
                        // 正常処理続行
                        let response = match sanitized_request.message {
                            Some(request_message) => {
                                Self::handle_request_message(service, request_message).await
                            }
                            None => {
                                warn!("Sanitized request has no message content");
                                Self::create_empty_message_error()
                            }
                        };

                        // レスポンスをキャッシュに保存
                        service.cache.put(&stream_request, response.clone()).await;
                        
                        response
                    }
                    Err(sanitize_error) => {
                        // サニタイゼーションエラー
                        Self::create_validation_error_response(sanitize_error, message_id)
                    }
                }
            }
            Err(validation_error) => {
                // 検証失敗
                Self::create_validation_error_response(validation_error, message_id)
            }
        }
    }

    /// Validate request for testing purposes
    pub async fn validate_request(&self, request: &StreamRequest, client_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let context = ValidationContext {
            client_id: client_id.to_string(),
            connection_id: "test_connection".to_string(),
            message_id: 1,
            timestamp: std::time::SystemTime::now(),
            client_info: None,
        };

        match self.validation_engine.validate_stream_request(request, &context).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Route request messages to appropriate handlers using generic trait approach
    async fn handle_request_message(
        service: &Arc<Self>,
        request_message: stream_request::Message,
    ) -> StreamResponse {
        match request_message {
            stream_request::Message::ImportAsset(import_req) => {
                Self::handle_stream_request::<ImportAssetOperation>(service, import_req).await
            }
            stream_request::Message::MoveAsset(move_req) => {
                Self::handle_stream_request::<MoveAssetOperation>(service, move_req).await
            }
            stream_request::Message::DeleteAsset(delete_req) => {
                Self::handle_stream_request::<DeleteAssetOperation>(service, delete_req).await
            }
            stream_request::Message::Refresh(refresh_req) => {
                Self::handle_stream_request::<RefreshOperation>(service, refresh_req).await
            }
        }
    }

    fn create_stream_error_response(status: Status) -> StreamResponse {
        let error_type = StreamErrorType::map_grpc_status_to_error_type(&status);
        let context = ErrorContext::new(None);

        Self::log_stream_error(
            &error_type,
            &format!("Stream processing error: {}", status.message()),
            &context,
            Some(&status),
        );

        Self::create_error_response(
            error_type,
            &format!("Stream processing error: {}", status.message()),
            &format!("Stream handler encountered an error: {:?}", status),
            None,
        )
    }

    /// Create unified error response based on request type and error details
    fn create_error_response(
        error_type: StreamErrorType,
        message: &str,
        details: &str,
        request_type: Option<&str>,
    ) -> StreamResponse {
        let mut context = ErrorContext::new(request_type.map(String::from));
        context.add_info("error_type".to_string(), format!("{:?}", error_type));

        let enhanced_details = format!("{} | Context: {}", details, context.to_details_string());

        let mcp_error = McpError {
            code: error_type.to_grpc_code(),
            message: message.to_string(),
            details: enhanced_details.clone(),
        };

        match request_type {
            Some("import_asset") => StreamResponse {
                message: Some(stream_response::Message::ImportAsset(ImportAssetResponse {
                    asset: None,
                    error: Some(mcp_error),
                })),
            },
            Some("move_asset") => StreamResponse {
                message: Some(stream_response::Message::MoveAsset(MoveAssetResponse {
                    asset: None,
                    error: Some(mcp_error),
                })),
            },
            Some("delete_asset") => StreamResponse {
                message: Some(stream_response::Message::DeleteAsset(DeleteAssetResponse {
                    success: false,
                    error: Some(mcp_error),
                })),
            },
            Some("refresh") => StreamResponse {
                message: Some(stream_response::Message::Refresh(RefreshResponse {
                    success: false,
                    error: Some(mcp_error),
                })),
            },
            _ => {
                // Generic error response - use ImportAsset but mark as generic
                StreamResponse {
                    message: Some(stream_response::Message::ImportAsset(ImportAssetResponse {
                        asset: None,
                        error: Some(McpError {
                            code: error_type.to_grpc_code(),
                            message: format!("Generic stream error: {}", message),
                            details: format!("GENERIC_ERROR | {}", enhanced_details),
                        }),
                    })),
                }
            }
        }
    }

    /// Log structured error information
    fn log_stream_error(
        error_type: &StreamErrorType,
        message: &str,
        context: &ErrorContext,
        status: Option<&Status>,
    ) {
        let error_details = json!({
            "error_type": format!("{:?}", error_type),
            "message": message,
            "context": context,
            "grpc_status": status.map(|s| {
                json!({
                    "code": s.code() as i32,
                    "message": s.message(),
                })
            }),
        });

        match error_type {
            StreamErrorType::InternalError | StreamErrorType::ProcessingError => {
                tracing::error!(error_details = %error_details, "Stream processing error");
            }
            StreamErrorType::ValidationError | StreamErrorType::InvalidRequest => {
                warn!(error_details = %error_details, "Stream validation error");
            }
            StreamErrorType::ResourceExhausted => {
                warn!(error_details = %error_details, "Stream resource exhausted");
            }
            StreamErrorType::NotFound => {
                info!(error_details = %error_details, "Stream resource not found");
            }
        }
    }

    fn create_empty_message_error() -> StreamResponse {
        let context = ErrorContext::new(None);
        Self::log_stream_error(
            &StreamErrorType::InvalidRequest,
            "Empty stream request received",
            &context,
            None,
        );

        Self::create_error_response(
            StreamErrorType::InvalidRequest,
            "Empty stream request received",
            "StreamRequest must contain a valid message field",
            None,
        )
    }

    /// Unified stream request handler that eliminates code duplication
    /// This replaces the 4 individual handle_*_stream methods with a single generic function
    async fn handle_stream_request<H: StreamOperationHandler>(
        service: &Arc<UnityMcpServiceImpl>,
        request: H::Request,
    ) -> StreamResponse {
        let operation_name = H::get_operation_name();
        let debug_info = H::extract_debug_info(&request);

        // Log debug information
        let mut debug_msg = format!("Processing {} stream request", operation_name);
        for (key, value) in &debug_info {
            debug_msg.push_str(&format!(", {} = {}", key, value));
        }
        debug!("{}", debug_msg);

        // Execute the service call
        let grpc_request = Request::new(request);
        match H::call_service(service, grpc_request).await {
            Ok(response) => {
                let inner_response = response.into_inner();
                debug!(
                    operation = %operation_name,
                    "Operation completed successfully"
                );
                H::build_stream_response(inner_response)
            }
            Err(status) => {
                let error_type = StreamErrorType::map_grpc_status_to_error_type(&status);
                let mut context = ErrorContext::new(Some(operation_name.to_string()));

                // Add debug info to error context
                for (key, value) in debug_info {
                    context.add_info(key.to_string(), value);
                }

                Self::log_stream_error(
                    &error_type,
                    &format!("{} operation failed: {}", operation_name, status.message()),
                    &context,
                    Some(&status),
                );

                Self::create_error_response(
                    error_type,
                    &format!("{} operation failed: {}", operation_name, status.message()),
                    &format!(
                        "gRPC status: {:?} | Details: {}",
                        status.code(),
                        status.message()
                    ),
                    Some(operation_name),
                )
            }
        }
    }

    /// Create validation error response from StreamValidationError
    fn create_validation_error_response(
        error: StreamValidationError,
        message_id: u64,
    ) -> StreamResponse {
        warn!(
            message_id = message_id,
            error = %error,
            "Stream request validation failed"
        );

        Self::create_error_response(
            StreamErrorType::ValidationError,
            &format!("Request validation failed: {}", error),
            &format!(
                "Message: {} | Validation error details: {:?}",
                message_id, error
            ),
            None,
        )
    }

    // Legacy individual handler methods removed - replaced by generic trait approach above
}

// ============================================================================
// Individual Operation Handlers (Trait Implementations)
// ============================================================================

/// ImportAsset operation handler
pub struct ImportAssetOperation;

#[async_trait]
impl StreamOperationHandler for ImportAssetOperation {
    type Request = ImportAssetRequest;
    type Response = ImportAssetResponse;

    async fn call_service(
        service: &Arc<UnityMcpServiceImpl>,
        request: Request<Self::Request>,
    ) -> Result<Response<Self::Response>, Status> {
        service.import_asset(request).await
    }

    fn build_stream_response(response: Self::Response) -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::ImportAsset(response)),
        }
    }

    fn get_operation_name() -> &'static str {
        "import_asset"
    }

    fn extract_debug_info(request: &Self::Request) -> Vec<(&'static str, String)> {
        vec![("asset_path", request.asset_path.clone())]
    }
}

/// MoveAsset operation handler
pub struct MoveAssetOperation;

#[async_trait]
impl StreamOperationHandler for MoveAssetOperation {
    type Request = MoveAssetRequest;
    type Response = MoveAssetResponse;

    async fn call_service(
        service: &Arc<UnityMcpServiceImpl>,
        request: Request<Self::Request>,
    ) -> Result<Response<Self::Response>, Status> {
        service.move_asset(request).await
    }

    fn build_stream_response(response: Self::Response) -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::MoveAsset(response)),
        }
    }

    fn get_operation_name() -> &'static str {
        "move_asset"
    }

    fn extract_debug_info(request: &Self::Request) -> Vec<(&'static str, String)> {
        vec![
            ("src_path", request.src_path.clone()),
            ("dst_path", request.dst_path.clone()),
        ]
    }
}

/// DeleteAsset operation handler
pub struct DeleteAssetOperation;

#[async_trait]
impl StreamOperationHandler for DeleteAssetOperation {
    type Request = DeleteAssetRequest;
    type Response = DeleteAssetResponse;

    async fn call_service(
        service: &Arc<UnityMcpServiceImpl>,
        request: Request<Self::Request>,
    ) -> Result<Response<Self::Response>, Status> {
        service.delete_asset(request).await
    }

    fn build_stream_response(response: Self::Response) -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::DeleteAsset(response)),
        }
    }

    fn get_operation_name() -> &'static str {
        "delete_asset"
    }

    fn extract_debug_info(request: &Self::Request) -> Vec<(&'static str, String)> {
        vec![("asset_path", request.asset_path.clone())]
    }
}

/// Refresh operation handler  
pub struct RefreshOperation;

#[async_trait]
impl StreamOperationHandler for RefreshOperation {
    type Request = RefreshRequest;
    type Response = RefreshResponse;

    async fn call_service(
        service: &Arc<UnityMcpServiceImpl>,
        request: Request<Self::Request>,
    ) -> Result<Response<Self::Response>, Status> {
        service.refresh(request).await
    }

    fn build_stream_response(response: Self::Response) -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::Refresh(response)),
        }
    }

    fn get_operation_name() -> &'static str {
        "refresh"
    }

    fn extract_debug_info(_request: &Self::Request) -> Vec<(&'static str, String)> {
        vec![] // Refresh request has no specific parameters to log
    }
}
#[async_trait]
impl UnityMcpService for UnityMcpServiceImpl {
    /// List all available MCP tools
    ///
    /// Currently returns an empty list as this is a stub implementation.
    #[instrument(skip(self))]
    async fn list_tools(
        &self,
        _request: Request<ListToolsRequest>,
    ) -> Result<Response<ListToolsResponse>, Status> {
        info!("ListTools called");

        let response = ListToolsResponse {
            tools: vec![], // Empty list for stub implementation
            error: no_error(),
        };

        debug!("Returning {} tools", response.tools.len());
        Ok(Response::new(response))
    }

    /// Execute an MCP tool with given input
    ///
    /// Currently provides basic validation and returns a dummy success response.
    #[instrument(skip(self))]
    async fn call_tool(
        &self,
        request: Request<CallToolRequest>,
    ) -> Result<Response<CallToolResponse>, Status> {
        let req = request.into_inner();

        info!(tool_id = %req.tool_id, "CallTool called");

        // Basic validation
        if req.tool_id.trim().is_empty() {
            let response = CallToolResponse {
                output_json: String::new(),
                error: Some(validation_error(
                    "Invalid tool_id",
                    "tool_id cannot be empty",
                )),
            };
            return Ok(Response::new(response));
        }

        // For stub implementation, return a dummy success response
        let response = CallToolResponse {
            output_json: r#"{"status": "not_implemented", "message": "Tool functionality will be implemented in future tasks"}"#.to_string(),
            error: no_error(),
        };

        debug!(tool_id = %req.tool_id, "Tool execution completed (stub)");
        Ok(Response::new(response))
    }

    /// List all available MCP resources
    ///
    /// Currently returns an empty list as this is a stub implementation.
    #[instrument(skip(self))]
    async fn list_resources(
        &self,
        _request: Request<ListResourcesRequest>,
    ) -> Result<Response<ListResourcesResponse>, Status> {
        info!("ListResources called");

        let response = ListResourcesResponse {
            resources: vec![], // Empty list for stub implementation
            error: no_error(),
        };

        debug!("Returning {} resources", response.resources.len());
        Ok(Response::new(response))
    }

    /// Read content of an MCP resource
    ///
    /// Currently returns dummy binary data for demonstration purposes.
    #[instrument(skip(self))]
    async fn read_resource(
        &self,
        request: Request<ReadResourceRequest>,
    ) -> Result<Response<ReadResourceResponse>, Status> {
        let req = request.into_inner();

        info!(uri = %req.uri, "ReadResource called");

        // Basic validation
        if req.uri.trim().is_empty() {
            let response = ReadResourceResponse {
                data: Vec::new(),
                mime_type: String::new(),
                error: Some(validation_error(
                    "Invalid URI",
                    "URI parameter cannot be empty",
                )),
            };
            return Ok(Response::new(response));
        }

        // Check for existence (dummy logic - only accept specific URIs)
        if !req.uri.starts_with("unity://") {
            let response = ReadResourceResponse {
                data: Vec::new(),
                mime_type: String::new(),
                error: Some(not_found_error("resource", &req.uri)),
            };
            return Ok(Response::new(response));
        }

        // Return dummy binary data for valid URIs
        let dummy_data = "Hello from Unity MCP".as_bytes().to_vec();
        let response = ReadResourceResponse {
            data: dummy_data,
            mime_type: "text/plain".to_string(),
            error: no_error(),
        };

        debug!("Returning {} bytes of data", response.data.len());
        Ok(Response::new(response))
    }

    /// List all available MCP prompts
    ///
    /// Currently returns an empty list as this is a stub implementation.
    /// Future tasks will implement actual prompt discovery and listing.
    #[instrument(skip(self))]
    async fn list_prompts(
        &self,
        _request: Request<ListPromptsRequest>,
    ) -> Result<Response<ListPromptsResponse>, Status> {
        info!("ListPrompts called");

        let response = ListPromptsResponse {
            prompt_ids: vec![], // Empty list for stub implementation
            error: no_error(),
        };

        debug!("Returning {} prompt IDs", response.prompt_ids.len());
        Ok(Response::new(response))
    }

    /// Get content of an MCP prompt
    ///
    /// Currently returns dummy prompt text for demonstration purposes.
    #[instrument(skip(self))]
    async fn get_prompt(
        &self,
        request: Request<GetPromptRequest>,
    ) -> Result<Response<GetPromptResponse>, Status> {
        let req = request.into_inner();

        info!(prompt_id = %req.prompt_id, "GetPrompt called");

        // Basic validation
        if req.prompt_id.trim().is_empty() {
            let response = GetPromptResponse {
                prompt_text: String::new(),
                error: Some(validation_error(
                    "Invalid prompt_id",
                    "prompt_id parameter cannot be empty",
                )),
            };
            return Ok(Response::new(response));
        }

        // Check for existence (dummy logic - only accept specific prompt IDs)
        if !req.prompt_id.starts_with("unity_") {
            let response = GetPromptResponse {
                prompt_text: String::new(),
                error: Some(not_found_error("prompt", &req.prompt_id)),
            };
            return Ok(Response::new(response));
        }

        // Return dummy prompt text for valid prompt IDs
        let dummy_prompt = format!(
            "This is a sample prompt from Unity MCP for prompt ID: {}. \
             Use this prompt to interact with Unity assets and operations.",
            req.prompt_id
        );

        let response = GetPromptResponse {
            prompt_text: dummy_prompt,
            error: no_error(),
        };

        debug!(prompt_id = %req.prompt_id, "Returning prompt text");
        Ok(Response::new(response))
    }

    /// Get Unity project information (stub implementation)
    #[instrument(skip(self))]
    async fn get_project_info(
        &self,
        _request: Request<GetProjectInfoRequest>,
    ) -> Result<Response<GetProjectInfoResponse>, Status> {
        info!("GetProjectInfo called");

        // Create dummy project information
        let project_info = ProjectInfo {
            project_name: Self::STUB_PROJECT_NAME.to_string(),
            unity_version: Self::STUB_UNITY_VERSION.to_string(),
        };

        debug!(project_name = %project_info.project_name, unity_version = %project_info.unity_version, "Returning project information (stub)");

        let response = GetProjectInfoResponse {
            project: Some(project_info),
            error: no_error(),
        };
        Ok(Response::new(response))
    }

    /// Import an asset into the Unity project (stub implementation)
    #[instrument(skip(self))]
    async fn import_asset(
        &self,
        request: Request<ImportAssetRequest>,
    ) -> Result<Response<ImportAssetResponse>, Status> {
        let req = request.into_inner();

        info!(asset_path = %req.asset_path, "ImportAsset called");

        // Comprehensive path validation
        if let Err(error) = self.validate_asset_path(&req.asset_path, "asset_path") {
            return Ok(Response::new(ImportAssetResponse::with_error(error)));
        }

        // Generate dummy Unity Asset with UUID-based GUID
        let guid = Uuid::new_v4().simple().to_string(); // 32-character hex string without hyphens
        let asset = UnityAsset {
            guid: guid.clone(),
            asset_path: req.asset_path.clone(),
            r#type: Self::DEFAULT_ASSET_TYPE.to_string(), // Using r#type to handle 'type' keyword
        };

        let response = ImportAssetResponse {
            asset: Some(asset),
            error: no_error(),
        };

        debug!(asset_path = %req.asset_path, guid = %guid, "Asset import completed (stub)");
        Ok(Response::new(response))
    }

    /// Move an asset to a new location (stub implementation)
    #[instrument(skip(self))]
    async fn move_asset(
        &self,
        request: Request<MoveAssetRequest>,
    ) -> Result<Response<MoveAssetResponse>, Status> {
        let req = request.into_inner();

        info!(src_path = %req.src_path, dst_path = %req.dst_path, "MoveAsset called");

        // Comprehensive path validation
        if let Err(error) = self.validate_move_paths(&req.src_path, &req.dst_path) {
            return Ok(Response::new(MoveAssetResponse::with_error(error)));
        }

        // Generate dummy Unity Asset representing the moved asset
        let guid = Uuid::new_v4().simple().to_string(); // 32-character hex string without hyphens
        let asset = UnityAsset {
            guid: guid.clone(),
            asset_path: req.dst_path.clone(), // Use destination path for moved asset
            r#type: Self::DEFAULT_ASSET_TYPE.to_string(),
        };

        let response = MoveAssetResponse {
            asset: Some(asset),
            error: no_error(),
        };

        debug!(src_path = %req.src_path, dst_path = %req.dst_path, guid = %guid, "Asset move completed (stub)");
        Ok(Response::new(response))
    }

    /// Delete an asset from the Unity project (stub implementation)
    #[instrument(skip(self))]
    async fn delete_asset(
        &self,
        request: Request<DeleteAssetRequest>,
    ) -> Result<Response<DeleteAssetResponse>, Status> {
        let req = request.into_inner();

        info!(asset_path = %req.asset_path, "DeleteAsset called");

        // Comprehensive path validation
        if let Err(error) = self.validate_asset_path(&req.asset_path, "asset_path") {
            let response = DeleteAssetResponse {
                success: false,
                error: Some(error),
            };
            return Ok(Response::new(response));
        }

        // Check for asset existence (dummy logic - only accept specific patterns)
        // In a real implementation, this would query Unity's AssetDatabase
        if !req.asset_path.ends_with(".cs")
            && !req.asset_path.ends_with(".prefab")
            && !req.asset_path.ends_with(".png")
            && !req.asset_path.ends_with(".fbx")
        {
            let response = DeleteAssetResponse {
                success: false,
                error: Some(not_found_error("asset", &req.asset_path)),
            };
            debug!(asset_path = %req.asset_path, "Asset not found or unsupported type");
            return Ok(Response::new(response));
        }

        // Simulate successful deletion
        let response = DeleteAssetResponse {
            success: true,
            error: no_error(),
        };

        debug!(asset_path = %req.asset_path, "Asset deletion completed (stub)");
        Ok(Response::new(response))
    }

    /// Refresh the AssetDatabase (stub implementation)
    #[instrument(skip(self))]
    async fn refresh(
        &self,
        _request: Request<RefreshRequest>,
    ) -> Result<Response<RefreshResponse>, Status> {
        info!("Refresh called");

        // Simulate AssetDatabase refresh operation
        // In a real implementation, this would trigger Unity's AssetDatabase.Refresh()
        debug!("Simulating AssetDatabase refresh operation");

        // For stub implementation, always succeed
        let response = RefreshResponse {
            success: true,
            error: no_error(),
        };

        debug!("AssetDatabase refresh completed (stub)");
        Ok(Response::new(response))
    }

    /// Stream type for bidirectional streaming
    type StreamStream = ServiceStream;

    /// Bidirectional stream for real-time Unity operations
    #[instrument(skip(self))]
    async fn stream(
        &self,
        request: Request<Streaming<StreamRequest>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        info!("Stream connection established");

        let stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(Self::STREAM_CHANNEL_CAPACITY);

        // Create shared service instance using Arc
        let service = Arc::new(
            UnityMcpServiceImpl::new()
                .map_err(|e| Status::internal(format!("Failed to create service: {}", e)))?
        );

        // Create and start stream handler with proper task lifecycle management
        let _stream_handler = StreamHandler::new(service, stream, tx);

        // Convert the receiver into a stream
        let response_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let boxed_stream: Self::StreamStream = Box::pin(response_stream);

        Ok(Response::new(boxed_stream))
    }
}


#[cfg(test)]
mod tests;
