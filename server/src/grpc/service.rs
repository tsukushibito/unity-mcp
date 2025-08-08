//! gRPC service implementations for Unity MCP Server
//!
//! This module contains the implementation of the UnityMcpService trait,
//! providing stub implementations for MCP core operations.

use async_trait::async_trait;
use tonic::{Request, Response, Status, Streaming};
use tokio_stream::Stream;
use tracing::{info, debug, instrument};
use std::pin::Pin;

use crate::grpc::{
    unity_mcp_service_server::UnityMcpService,
    ListToolsRequest, ListToolsResponse,
    CallToolRequest, CallToolResponse,
    ListResourcesRequest, ListResourcesResponse,
    ReadResourceRequest, ReadResourceResponse,
    ListPromptsRequest, ListPromptsResponse,
    GetPromptRequest, GetPromptResponse,
    GetProjectInfoRequest, GetProjectInfoResponse,
    ImportAssetRequest, ImportAssetResponse,
    MoveAssetRequest, MoveAssetResponse,
    DeleteAssetRequest, DeleteAssetResponse,
    RefreshRequest, RefreshResponse,
    StreamRequest, StreamResponse,
};
use crate::grpc::error::{validation_error, no_error, internal_server_error, not_found_error};

/// Unity MCP Service implementation
/// 
/// Provides stub implementations for all MCP operations.
/// Task 3.3 focuses on the first 3 methods (ListTools, CallTool, ListResources),
/// while other methods are provided as minimal stubs.
pub struct UnityMcpServiceImpl;

/// Stream type for the bidirectional streaming RPC
type ServiceStream = Pin<Box<dyn Stream<Item = Result<StreamResponse, Status>> + Send>>;

impl UnityMcpServiceImpl {
    /// Create a new service instance
    pub fn new() -> Self {
        Self
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
                error: Some(validation_error("Invalid tool_id", "tool_id cannot be empty")),
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
                error: Some(validation_error("Invalid URI", "URI parameter cannot be empty")),
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
                error: Some(validation_error("Invalid prompt_id", "prompt_id parameter cannot be empty")),
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
        info!("GetProjectInfo called (stub)");
        
        let response = GetProjectInfoResponse {
            project: None,
            error: Some(internal_server_error("Not implemented in this task")),
        };
        
        Ok(Response::new(response))
    }

    /// Import an asset into the Unity project (stub implementation)
    #[instrument(skip(self))]
    async fn import_asset(
        &self,
        _request: Request<ImportAssetRequest>,
    ) -> Result<Response<ImportAssetResponse>, Status> {
        info!("ImportAsset called (stub)");
        
        let response = ImportAssetResponse {
            asset: None,
            error: Some(internal_server_error("Not implemented in this task")),
        };
        
        Ok(Response::new(response))
    }

    /// Move an asset to a new location (stub implementation)
    #[instrument(skip(self))]
    async fn move_asset(
        &self,
        _request: Request<MoveAssetRequest>,
    ) -> Result<Response<MoveAssetResponse>, Status> {
        info!("MoveAsset called (stub)");
        
        let response = MoveAssetResponse {
            asset: None,
            error: Some(internal_server_error("Not implemented in this task")),
        };
        
        Ok(Response::new(response))
    }

    /// Delete an asset from the Unity project (stub implementation)
    #[instrument(skip(self))]
    async fn delete_asset(
        &self,
        _request: Request<DeleteAssetRequest>,
    ) -> Result<Response<DeleteAssetResponse>, Status> {
        info!("DeleteAsset called (stub)");
        
        let response = DeleteAssetResponse {
            success: false,
            error: Some(internal_server_error("Not implemented in this task")),
        };
        
        Ok(Response::new(response))
    }

    /// Refresh the AssetDatabase (stub implementation)
    #[instrument(skip(self))]
    async fn refresh(
        &self,
        _request: Request<RefreshRequest>,
    ) -> Result<Response<RefreshResponse>, Status> {
        info!("Refresh called (stub)");
        
        let response = RefreshResponse {
            success: false,
            error: Some(internal_server_error("Not implemented in this task")),
        };
        
        Ok(Response::new(response))
    }

    /// Stream type for bidirectional streaming
    type StreamStream = ServiceStream;

    /// Bidirectional stream for real-time Unity operations (stub implementation)
    #[instrument(skip(self))]
    async fn stream(
        &self,
        _request: Request<Streaming<StreamRequest>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        info!("Stream called (stub)");
        
        // Return an empty stream for stub implementation
        let stream = tokio_stream::empty();
        let boxed_stream: Self::StreamStream = Box::pin(stream);
        
        Ok(Response::new(boxed_stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_list_tools() {
        let service = UnityMcpServiceImpl::new();
        let request = Request::new(ListToolsRequest {});
        
        let response = service.list_tools(request).await.unwrap();
        let inner = response.into_inner();
        
        assert!(inner.tools.is_empty());
        assert!(inner.error.is_none());
    }

    #[tokio::test]
    async fn test_call_tool_valid() {
        let service = UnityMcpServiceImpl::new();
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
        let service = UnityMcpServiceImpl::new();
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
        let service = UnityMcpServiceImpl::new();
        let request = Request::new(ListResourcesRequest {});
        
        let response = service.list_resources(request).await.unwrap();
        let inner = response.into_inner();
        
        assert!(inner.resources.is_empty());
        assert!(inner.error.is_none());
    }
}