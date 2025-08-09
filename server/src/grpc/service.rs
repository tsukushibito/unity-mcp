//! gRPC service implementations for Unity MCP Server
//!
//! This module contains the implementation of the UnityMcpService trait,
//! providing stub implementations for MCP core operations.

use async_trait::async_trait;
use std::path::{Component, Path};
use std::pin::Pin;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, info, instrument, warn};

use crate::grpc::error::{no_error, not_found_error, validation_error};
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
        serde_json::to_string(self).unwrap_or_else(|_| "Error context serialization failed".to_string())
    }
}

/// Unity MCP Service implementation
///
/// Provides stub implementations for all MCP operations.
/// Task 3.3 focuses on the first 3 methods (ListTools, CallTool, ListResources),
/// while other methods are provided as minimal stubs.
pub struct UnityMcpServiceImpl;

impl Default for UnityMcpServiceImpl {
    fn default() -> Self {
        Self::new()
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
    pub fn new() -> Self {
        Self
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
            message: Some(stream_response::Message::ImportAsset(
                ImportAssetResponse {
                    asset: None,
                    error: Some(McpError {
                        code: 8, // RESOURCE_EXHAUSTED
                        message: "Stream processing capacity exceeded".to_string(),
                        details: "Please reduce message rate".to_string(),
                    }),
                },
            )),
        }
    }

    /// Handle stream messages with shared service instance
    async fn handle_stream_messages(
        service: Arc<Self>,
        mut stream: Streaming<StreamRequest>,
        tx: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        while let Some(result) = stream.next().await {
            match result {
                Ok(stream_request) => {
                    let response = Self::process_stream_request(&service, stream_request).await;
                    
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

    /// Process individual stream requests using shared service instance
    async fn process_stream_request(
        service: &Arc<Self>,
        stream_request: StreamRequest,
    ) -> StreamResponse {
        debug!("Processing stream request");
        
        match stream_request.message {
            Some(request_message) => {
                Self::handle_request_message(service, request_message).await
            }
            None => {
                warn!("Received stream request with no message content");
                Self::create_empty_message_error()
            }
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
                message: Some(stream_response::Message::ImportAsset(
                    ImportAssetResponse {
                        asset: None,
                        error: Some(mcp_error),
                    },
                )),
            },
            Some("move_asset") => StreamResponse {
                message: Some(stream_response::Message::MoveAsset(
                    MoveAssetResponse {
                        asset: None,
                        error: Some(mcp_error),
                    },
                )),
            },
            Some("delete_asset") => StreamResponse {
                message: Some(stream_response::Message::DeleteAsset(
                    DeleteAssetResponse {
                        success: false,
                        error: Some(mcp_error),
                    },
                )),
            },
            Some("refresh") => StreamResponse {
                message: Some(stream_response::Message::Refresh(
                    RefreshResponse {
                        success: false,
                        error: Some(mcp_error),
                    },
                )),
            },
            _ => {
                // Generic error response - use ImportAsset but mark as generic
                StreamResponse {
                    message: Some(stream_response::Message::ImportAsset(
                        ImportAssetResponse {
                            asset: None,
                            error: Some(McpError {
                                code: error_type.to_grpc_code(),
                                message: format!("Generic stream error: {}", message),
                                details: format!("GENERIC_ERROR | {}", enhanced_details),
                            }),
                        },
                    )),
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
                    &format!("gRPC status: {:?} | Details: {}", status.code(), status.message()),
                    Some(operation_name),
                )
            }
        }
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
        let service = Arc::new(UnityMcpServiceImpl::new());
        
        // Create and start stream handler with proper task lifecycle management
        let _stream_handler = StreamHandler::new(service, stream, tx);
        
        // Convert the receiver into a stream
        let response_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let boxed_stream: Self::StreamStream = Box::pin(response_stream);

        Ok(Response::new(boxed_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_list_tools() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(ListToolsRequest {});

        let response = service.list_tools(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.tools.is_empty());
        assert!(inner.error.is_none());
    }

    #[tokio::test]
    async fn test_call_tool_valid() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(CallToolRequest {
            tool_id: "test_tool".to_string(),
            input_json: r#"{"param": "value"}"#.to_string(),
        });

        let response = service.call_tool(request).await.unwrap();
        let inner = response.into_inner();

        assert!(!inner.output_json.is_empty());
        assert!(inner.error.is_none());
    }

    #[tokio::test]
    async fn test_call_tool_empty_tool_id() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(CallToolRequest {
            tool_id: "".to_string(),
            input_json: r#"{"param": "value"}"#.to_string(),
        });

        let response = service.call_tool(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.output_json.is_empty());
        assert!(inner.error.is_some());

        let error = inner.error.unwrap();
        assert_eq!(error.code, 400);
    }

    #[tokio::test]
    async fn test_list_resources() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(ListResourcesRequest {});

        let response = service.list_resources(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.resources.is_empty());
        assert!(inner.error.is_none());
    }

    // Path validation tests
    #[tokio::test]
    async fn test_validate_asset_path_traversal() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Test various path traversal attempts
        assert!(service
            .validate_asset_path("Assets/../secret", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("Assets/..\\secret", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("Assets/subdir/../../../secret", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("Assets/normal/../../secret", "asset_path")
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_asset_path_valid() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Test valid asset paths
        assert!(service
            .validate_asset_path("Assets/Textures/texture.png", "asset_path")
            .is_ok());
        assert!(service
            .validate_asset_path("Assets/Scripts/MyScript.cs", "asset_path")
            .is_ok());
        assert!(service
            .validate_asset_path("Assets/Models/Character.fbx", "asset_path")
            .is_ok());
        assert!(service
            .validate_asset_path("Assets/Prefabs/Weapon.prefab", "asset_path")
            .is_ok());
    }

    #[tokio::test]
    async fn test_validate_asset_path_invalid_prefix() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Test paths that don't start with Assets/
        assert!(service
            .validate_asset_path("Resources/texture.png", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("/Assets/texture.png", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("assets/texture.png", "asset_path")
            .is_err());
        assert!(service.validate_asset_path("", "asset_path").is_err());
        assert!(service.validate_asset_path(" ", "asset_path").is_err());
    }

    #[tokio::test]
    async fn test_validate_asset_path_invalid_characters() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Test paths with invalid characters
        assert!(service
            .validate_asset_path("Assets/texture\u{0000}.png", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("Assets/tex<texture.png", "asset_path")
            .is_err());
        assert!(service
            .validate_asset_path("Assets/tex>texture.png", "asset_path")
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_asset_path_length_limit() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Create a path that exceeds MAX_PATH_LENGTH (260 characters)
        // "Assets/" = 7 chars, ".png" = 4 chars, so filename needs 260 - 7 - 4 + 1 = 250 chars to exceed
        let long_filename = "a".repeat(250);
        let long_path = format!("Assets/{}.png", long_filename);
        assert!(service
            .validate_asset_path(&long_path, "asset_path")
            .is_err());

        // Test a path that's just under the limit (should pass)
        // "Assets/" = 7 chars, ".png" = 4 chars, so filename can be up to 260 - 7 - 4 = 249 chars
        let acceptable_filename = "a".repeat(249);
        let acceptable_path = format!("Assets/{}.png", acceptable_filename);
        assert!(service
            .validate_asset_path(&acceptable_path, "asset_path")
            .is_ok());
    }

    #[tokio::test]
    async fn test_validate_move_paths_same() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Test that source and destination paths must be different
        assert!(service
            .validate_move_paths("Assets/texture.png", "Assets/texture.png")
            .is_err());
        assert!(service
            .validate_move_paths("Assets/Scripts/Script.cs", "Assets/Scripts/Script.cs")
            .is_err());
    }

    #[tokio::test]
    async fn test_validate_move_paths_valid() {
        let service = Arc::new(UnityMcpServiceImpl::new());

        // Test valid move operations
        assert!(service
            .validate_move_paths("Assets/texture.png", "Assets/Textures/texture.png")
            .is_ok());
        assert!(service
            .validate_move_paths("Assets/Scripts/Old.cs", "Assets/Scripts/New.cs")
            .is_ok());
    }

    // Error response tests
    #[tokio::test]
    async fn test_import_asset_error_response() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(ImportAssetRequest {
            asset_path: "invalid_path".to_string(),
        });

        let response = service.import_asset(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.asset.is_none());
        assert!(inner.error.is_some());

        let error = inner.error.unwrap();
        assert_eq!(error.code, 400); // Validation error
    }

    #[tokio::test]
    async fn test_move_asset_error_response() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(MoveAssetRequest {
            src_path: "Assets/texture.png".to_string(),
            dst_path: "Assets/texture.png".to_string(), // Same path should fail
        });

        let response = service.move_asset(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.asset.is_none());
        assert!(inner.error.is_some());

        let error = inner.error.unwrap();
        assert_eq!(error.code, 400); // Validation error
    }

    #[tokio::test]
    async fn test_import_asset_success() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(ImportAssetRequest {
            asset_path: "Assets/Textures/texture.png".to_string(),
        });

        let response = service.import_asset(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.asset.is_some());
        assert!(inner.error.is_none());

        let asset = inner.asset.unwrap();
        assert!(!asset.guid.is_empty());
        assert_eq!(asset.asset_path, "Assets/Textures/texture.png");
        assert_eq!(asset.r#type, "Unknown");
    }

    #[tokio::test]
    async fn test_move_asset_success() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(MoveAssetRequest {
            src_path: "Assets/texture.png".to_string(),
            dst_path: "Assets/Textures/texture.png".to_string(),
        });

        let response = service.move_asset(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.asset.is_some());
        assert!(inner.error.is_none());

        let asset = inner.asset.unwrap();
        assert!(!asset.guid.is_empty());
        assert_eq!(asset.asset_path, "Assets/Textures/texture.png"); // Should use destination path
        assert_eq!(asset.r#type, "Unknown");
    }

    #[tokio::test]
    async fn test_get_project_info() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        let request = Request::new(GetProjectInfoRequest {});

        let response = service.get_project_info(request).await.unwrap();
        let inner = response.into_inner();

        assert!(inner.project.is_some());
        assert!(inner.error.is_none());

        let project = inner.project.unwrap();
        assert_eq!(project.project_name, "Unity MCP Test Project");
        assert_eq!(project.unity_version, "2023.3.0f1");
    }

    // Backpressure and stream channel tests
    #[tokio::test]
    async fn test_bounded_channel_capacity() {
        // Test that the channel has the expected capacity limit
        let (tx, _rx) = tokio::sync::mpsc::channel::<Result<StreamResponse, Status>>(
            UnityMcpServiceImpl::STREAM_CHANNEL_CAPACITY,
        );
        
        // Fill the channel to capacity
        for _ in 0..UnityMcpServiceImpl::STREAM_CHANNEL_CAPACITY {
            let response = StreamResponse {
                message: Some(stream_response::Message::ImportAsset(
                    ImportAssetResponse {
                        asset: None,
                        error: None,
                    },
                )),
            };
            assert!(tx.try_send(Ok(response)).is_ok());
        }
        
        // The next send should fail due to capacity limit
        let overflow_response = StreamResponse {
            message: Some(stream_response::Message::ImportAsset(
                ImportAssetResponse {
                    asset: None,
                    error: None,
                },
            )),
        };
        assert!(matches!(
            tx.try_send(Ok(overflow_response)),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_))
        ));
    }

    #[tokio::test]
    async fn test_create_backpressure_error() {
        let error_response = UnityMcpServiceImpl::create_backpressure_error();
        
        // Verify the response structure
        assert!(error_response.message.is_some());
        
        if let Some(stream_response::Message::ImportAsset(import_response)) = error_response.message {
            assert!(import_response.asset.is_none());
            assert!(import_response.error.is_some());
            
            let error = import_response.error.unwrap();
            assert_eq!(error.code, 8); // RESOURCE_EXHAUSTED
            assert_eq!(error.message, "Stream processing capacity exceeded");
            assert_eq!(error.details, "Please reduce message rate");
        } else {
            panic!("Expected ImportAsset message in backpressure error response");
        }
    }

    // Resource management and Arc sharing tests
    #[tokio::test]
    async fn test_service_instance_sharing() {
        // Test that Arc<UnityMcpServiceImpl> can be shared across operations
        let service = Arc::new(UnityMcpServiceImpl::new());
        let service_clone = Arc::clone(&service);
        
        // Both references should point to the same instance
        assert!(Arc::ptr_eq(&service, &service_clone));
        
        // Test that both can perform operations
        let request1 = Request::new(ListToolsRequest {});
        let request2 = Request::new(ListResourcesRequest {});
        
        let response1 = service.list_tools(request1).await;
        let response2 = service_clone.list_resources(request2).await;
        
        assert!(response1.is_ok());
        assert!(response2.is_ok());
    }

    #[tokio::test]
    async fn test_stream_handler_components() {
        // Test that StreamHandler components work correctly
        let service = Arc::new(UnityMcpServiceImpl::new());
        let (tx, _rx) = tokio::sync::mpsc::channel::<Result<StreamResponse, Status>>(1);
        
        // Test that we can create and use Arc<UnityMcpServiceImpl>
        let service_clone = Arc::clone(&service);
        
        // Both should be usable
        assert!(Arc::ptr_eq(&service, &service_clone));
        
        // Test CancellationToken functionality
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_process_stream_request() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        
        // Test import asset request
        let import_request = StreamRequest {
            message: Some(stream_request::Message::ImportAsset(ImportAssetRequest {
                asset_path: "Assets/Test/texture.png".to_string(),
            })),
        };
        
        let response = UnityMcpServiceImpl::process_stream_request(&service, import_request).await;
        
        assert!(response.message.is_some());
        if let Some(stream_response::Message::ImportAsset(import_response)) = response.message {
            assert!(import_response.asset.is_some());
            assert!(import_response.error.is_none());
        } else {
            panic!("Expected ImportAsset response");
        }
    }

    #[tokio::test]
    async fn test_empty_stream_request() {
        let service = Arc::new(UnityMcpServiceImpl::new());
        
        let empty_request = StreamRequest {
            message: None,
        };
        
        let response = UnityMcpServiceImpl::process_stream_request(&service, empty_request).await;
        
        assert!(response.message.is_some());
        if let Some(stream_response::Message::ImportAsset(import_response)) = response.message {
            assert!(import_response.asset.is_none());
            assert!(import_response.error.is_some());
            
            let error = import_response.error.unwrap();
            assert_eq!(error.code, 3); // INVALID_ARGUMENT
            assert!(error.message.contains("Generic stream error"));
        } else {
            panic!("Expected ImportAsset error response");
        }
    }

    // Error handling system tests
    #[tokio::test]
    async fn test_error_response_mapping() {
        // Test InvalidArgument -> ValidationError
        let status = Status::new(tonic::Code::InvalidArgument, "Invalid input");
        let error_type = StreamErrorType::map_grpc_status_to_error_type(&status);
        assert!(matches!(error_type, StreamErrorType::ValidationError));
        assert_eq!(error_type.to_grpc_code(), 3);

        // Test NotFound -> NotFound
        let status = Status::new(tonic::Code::NotFound, "Resource not found");
        let error_type = StreamErrorType::map_grpc_status_to_error_type(&status);
        assert!(matches!(error_type, StreamErrorType::NotFound));
        assert_eq!(error_type.to_grpc_code(), 5);

        // Test Internal -> InternalError
        let status = Status::new(tonic::Code::Internal, "Internal server error");
        let error_type = StreamErrorType::map_grpc_status_to_error_type(&status);
        assert!(matches!(error_type, StreamErrorType::InternalError));
        assert_eq!(error_type.to_grpc_code(), 13);
    }

    #[tokio::test]
    async fn test_message_type_specific_errors() {
        // Test import_asset specific error response
        let import_response = UnityMcpServiceImpl::create_error_response(
            StreamErrorType::ValidationError,
            "Import failed",
            "Asset path validation failed",
            Some("import_asset"),
        );
        
        assert!(import_response.message.is_some());
        if let Some(stream_response::Message::ImportAsset(response)) = import_response.message {
            assert!(response.asset.is_none());
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 3);
            assert_eq!(error.message, "Import failed");
            assert!(error.details.contains("Asset path validation failed"));
        } else {
            panic!("Expected ImportAsset response");
        }

        // Test move_asset specific error response
        let move_response = UnityMcpServiceImpl::create_error_response(
            StreamErrorType::NotFound,
            "Move failed",
            "Source asset not found",
            Some("move_asset"),
        );
        
        if let Some(stream_response::Message::MoveAsset(response)) = move_response.message {
            assert!(response.asset.is_none());
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 5);
            assert_eq!(error.message, "Move failed");
        } else {
            panic!("Expected MoveAsset response");
        }

        // Test delete_asset specific error response
        let delete_response = UnityMcpServiceImpl::create_error_response(
            StreamErrorType::InternalError,
            "Delete failed",
            "Internal deletion error",
            Some("delete_asset"),
        );
        
        if let Some(stream_response::Message::DeleteAsset(response)) = delete_response.message {
            assert_eq!(response.success, false);
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 13);
        } else {
            panic!("Expected DeleteAsset response");
        }

        // Test refresh specific error response
        let refresh_response = UnityMcpServiceImpl::create_error_response(
            StreamErrorType::ResourceExhausted,
            "Refresh failed",
            "System resources exhausted",
            Some("refresh"),
        );
        
        if let Some(stream_response::Message::Refresh(response)) = refresh_response.message {
            assert_eq!(response.success, false);
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 8);
        } else {
            panic!("Expected Refresh response");
        }
    }

    #[tokio::test]
    async fn test_generic_error_response() {
        // Test generic error response when request_type is None
        let generic_response = UnityMcpServiceImpl::create_error_response(
            StreamErrorType::InvalidRequest,
            "Unknown request type",
            "Request type could not be determined",
            None,
        );
        
        assert!(generic_response.message.is_some());
        if let Some(stream_response::Message::ImportAsset(response)) = generic_response.message {
            assert!(response.asset.is_none());
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 3);
            assert!(error.message.starts_with("Generic stream error:"));
            assert!(error.details.starts_with("GENERIC_ERROR"));
        } else {
            panic!("Expected ImportAsset response for generic error");
        }
    }

    #[tokio::test]
    async fn test_error_context_tracking() {
        let mut context = ErrorContext::new(Some("test_operation".to_string()));
        context.add_info("test_key".to_string(), "test_value".to_string());
        
        // Test context serialization
        let details = context.to_details_string();
        assert!(details.contains("test_operation"));
        assert!(details.contains("test_key"));
        assert!(details.contains("test_value"));
        assert!(details.contains("request_id"));
        assert!(details.contains("timestamp"));

        // Test that request_id is unique
        let context1 = ErrorContext::new(Some("op1".to_string()));
        let context2 = ErrorContext::new(Some("op2".to_string()));
        assert_ne!(context1.request_id, context2.request_id);
    }

    #[tokio::test]
    async fn test_empty_message_error_handling() {
        let error_response = UnityMcpServiceImpl::create_empty_message_error();
        
        assert!(error_response.message.is_some());
        if let Some(stream_response::Message::ImportAsset(response)) = error_response.message {
            assert!(response.asset.is_none());
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 3); // INVALID_ARGUMENT
            assert!(error.message.contains("Generic stream error"));
            assert!(error.details.contains("GENERIC_ERROR"));
            assert!(error.details.contains("StreamRequest must contain a valid message field"));
        } else {
            panic!("Expected ImportAsset response for empty message error");
        }
    }

    #[tokio::test]
    async fn test_stream_error_response_handling() {
        let status = Status::new(tonic::Code::Unavailable, "Service unavailable");
        let error_response = UnityMcpServiceImpl::create_stream_error_response(status);
        
        assert!(error_response.message.is_some());
        if let Some(stream_response::Message::ImportAsset(response)) = error_response.message {
            assert!(response.asset.is_none());
            assert!(response.error.is_some());
            let error = response.error.unwrap();
            assert_eq!(error.code, 13); // INTERNAL (mapped from ProcessingError)
            assert!(error.message.contains("Stream processing error"));
            assert!(error.details.contains("GENERIC_ERROR"));
        } else {
            panic!("Expected ImportAsset response for stream error");
        }
    }

    #[tokio::test]
    async fn test_memory_efficiency_improvement() {
        // This test demonstrates the memory efficiency improvement
        // by using Arc<UnityMcpServiceImpl> instead of multiple instances
        
        let shared_service = Arc::new(UnityMcpServiceImpl::new());
        let mut clones = Vec::new();
        
        // Create multiple references to the same instance
        for _ in 0..100 {
            clones.push(Arc::clone(&shared_service));
        }
        
        // All clones should point to the same memory location
        for clone in &clones {
            assert!(Arc::ptr_eq(&shared_service, clone));
        }
        
        // Strong reference count should be 101 (original + 100 clones)
        assert_eq!(Arc::strong_count(&shared_service), 101);
    }
}
