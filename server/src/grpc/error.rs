//! Error handling module for gRPC operations
//!
//! This module provides error type conversions between internal errors and gRPC status codes,
//! enabling proper error reporting to gRPC clients.

use crate::grpc::McpError;
use tonic::{Code, Status};
use tracing::error;

/// Converts a `McpError` protobuf message to a `tonic::Status` for gRPC response
///
/// This function maps application-specific error codes to appropriate gRPC status codes
/// and preserves error messages for client debugging.
pub fn mcp_error_to_status(mcp_error: &McpError) -> Status {
    let code = match mcp_error.code {
        // Application-specific error code mappings
        400 => Code::InvalidArgument,
        404 => Code::NotFound,
        403 => Code::PermissionDenied,
        408 => Code::DeadlineExceeded,
        409 => Code::AlreadyExists,
        429 => Code::ResourceExhausted,
        500 => Code::Internal,
        501 => Code::Unimplemented,
        503 => Code::Unavailable,
        // Default to Internal for unknown error codes
        _ => Code::Internal,
    };

    let message = if mcp_error.message.is_empty() {
        "Unknown error".to_string()
    } else {
        mcp_error.message.clone()
    };

    // Include details in the status message if available
    let full_message = if mcp_error.details.is_empty() {
        message
    } else {
        format!("{}: {}", message, mcp_error.details)
    };

    Status::new(code, full_message)
}

/// Creates a `McpError` from a generic error and converts it to `tonic::Status`
///
/// This is a convenience function for handling internal errors that need to be
/// returned as gRPC responses.
pub fn internal_error_to_status(error: &anyhow::Error) -> Status {
    error!("Internal gRPC error: {:?}", error);

    let mcp_error = McpError {
        code: 500, // Internal Server Error
        message: "Internal server error".to_string(),
        details: error.to_string(),
    };

    mcp_error_to_status(&mcp_error)
}

/// Creates a successful response with no error
///
/// This function creates an `Option<McpError>` set to `None` for successful responses,
/// following the protobuf pattern where error fields are optional.
pub fn no_error() -> Option<McpError> {
    None
}

/// Creates a validation error for invalid request parameters
pub fn validation_error(message: &str, details: &str) -> McpError {
    McpError {
        code: 400,
        message: message.to_string(),
        details: details.to_string(),
    }
}

/// Creates a not found error for missing resources
pub fn not_found_error(resource_type: &str, resource_id: &str) -> McpError {
    McpError {
        code: 404,
        message: format!("{} not found", resource_type),
        details: format!("Resource ID: {}", resource_id),
    }
}

/// Creates an internal server error
pub fn internal_server_error(message: &str) -> McpError {
    McpError {
        code: 500,
        message: message.to_string(),
        details: "Internal server error occurred".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_error_to_status() {
        let mcp_error = McpError {
            code: 404,
            message: "Resource not found".to_string(),
            details: "The requested resource does not exist".to_string(),
        };

        let status = mcp_error_to_status(&mcp_error);
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(
            status.message(),
            "Resource not found: The requested resource does not exist"
        );
    }

    #[test]
    fn test_validation_error() {
        let error = validation_error("Invalid parameter", "Parameter 'name' is required");
        assert_eq!(error.code, 400);
        assert_eq!(error.message, "Invalid parameter");
        assert_eq!(error.details, "Parameter 'name' is required");
    }

    #[test]
    fn test_not_found_error() {
        let error = not_found_error("Asset", "12345");
        assert_eq!(error.code, 404);
        assert_eq!(error.message, "Asset not found");
        assert_eq!(error.details, "Resource ID: 12345");
    }

    #[test]
    fn test_internal_server_error() {
        let error = internal_server_error("Database connection failed");
        assert_eq!(error.code, 500);
        assert_eq!(error.message, "Database connection failed");
        assert_eq!(error.details, "Internal server error occurred");
    }
}
