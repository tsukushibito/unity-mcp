use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityGetCompileDiagnosticsRequest {
    #[serde(default = "default_max_items")]
    pub max_items: u32,

    #[serde(default = "default_severity")]
    pub severity: String,

    #[serde(default)]
    pub changed_only: bool,

    pub assembly: Option<String>,
}

fn default_max_items() -> u32 {
    500
}

fn default_severity() -> String {
    "all".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagnosticPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagnosticRange {
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    pub file_uri: String,
    pub range: DiagnosticRange,
    pub severity: String,
    pub message: String,
    pub code: Option<String>,
    pub assembly: String,
    pub source: String,
    pub fingerprint: String,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiagnosticSummary {
    pub errors: u32,
    pub warnings: u32,
    pub infos: u32,
    pub assemblies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompileDiagnostics {
    pub compile_id: String,
    pub summary: DiagnosticSummary,
    pub diagnostics: Vec<Diagnostic>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityGetCompileDiagnosticsResponse {
    pub compile_id: String,
    pub summary: DiagnosticSummary,
    pub diagnostics: Vec<Diagnostic>,
    pub truncated: bool,
}

impl McpService {
    pub async fn do_unity_get_compile_diagnostics(
        &self,
        max_items: u32,
        severity: String,
        changed_only: bool,
        assembly: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "Getting Unity compile diagnostics with max_items={}, severity={}, changed_only={}, assembly={:?}",
            max_items,
            severity,
            changed_only,
            assembly
        );

        // Request diagnostics via IPC
        let compile_diagnostics = match self
            .request_diagnostics_via_ipc(max_items, &severity, changed_only, &assembly)
            .await
        {
            Ok(diagnostics) => diagnostics,
            Err(e) => {
                tracing::warn!("Failed to get diagnostics via IPC: {}", e);
                return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    format!(
                        "Unity compile diagnostics not available. Please trigger a compilation in Unity Editor first.\n\nError: {}",
                        e
                    ),
                )]));
            }
        };

        // Diagnostics are already filtered and limited by Unity Bridge
        let filtered_diagnostics = compile_diagnostics.diagnostics;
        let truncated = compile_diagnostics.truncated;

        // Recalculate summary for filtered results
        let summary = self.calculate_summary(&filtered_diagnostics);

        let response = UnityGetCompileDiagnosticsResponse {
            compile_id: compile_diagnostics.compile_id,
            summary,
            diagnostics: filtered_diagnostics,
            truncated,
        };

        tracing::info!(
            "Returning {} diagnostics (truncated: {})",
            response.diagnostics.len(),
            truncated
        );

        let json_value = serde_json::to_value(response).map_err(|e| {
            rmcp::ErrorData::internal_error(format!("Failed to serialize response: {}", e), None)
        })?;
        Ok(CallToolResult::structured(json_value))
    }

    /// Request compile diagnostics from Unity Bridge via IPC
    async fn request_diagnostics_via_ipc(
        &self,
        max_items: u32,
        severity: &str,
        changed_only: bool,
        assembly: &Option<String>,
    ) -> anyhow::Result<CompileDiagnostics> {
        use crate::generated::mcp::unity::v1::{GetCompileDiagnosticsRequest, ipc_request};

        // Get IPC client
        let client = self
            .require_ipc()
            .await
            .map_err(|e| anyhow::anyhow!("IPC client not available: {}", e.message))?;

        // Create request
        let request = GetCompileDiagnosticsRequest {
            max_items,
            severity: severity.to_string(),
            changed_only,
            assembly: assembly.clone().unwrap_or_default(),
        };

        // Send IPC request
        let ipc_request = crate::generated::mcp::unity::v1::IpcRequest {
            payload: Some(ipc_request::Payload::GetCompileDiagnostics(request)),
        };

        tracing::debug!(
            "Sending GetCompileDiagnostics request via IPC: max_items={}, severity={}, assembly={:?}",
            max_items,
            severity,
            assembly
        );

        let response = client
            .request(ipc_request, std::time::Duration::from_secs(30))
            .await
            .map_err(|e| anyhow::anyhow!("IPC request failed: {}", e))?;

        // Extract diagnostics response
        let diagnostics_response = match response.payload {
            Some(
                crate::generated::mcp::unity::v1::ipc_response::Payload::GetCompileDiagnostics(
                    resp,
                ),
            ) => resp,
            _ => return Err(anyhow::anyhow!("Unexpected IPC response type")),
        };

        // Check if request was successful
        if !diagnostics_response.success {
            return Err(anyhow::anyhow!(
                "Unity Bridge error: {}",
                diagnostics_response.error_message
            ));
        }

        // Convert protobuf response to internal format
        let diagnostics: Vec<Diagnostic> = diagnostics_response
            .diagnostics
            .into_iter()
            .map(|pb_diag| {
                let range = pb_diag.range.as_ref();
                Diagnostic {
                    file_uri: pb_diag.file_uri,
                    range: DiagnosticRange {
                        start: DiagnosticPosition {
                            line: range.map(|r| r.line).unwrap_or(0),
                            character: range.map(|r| r.column).unwrap_or(0),
                        },
                        end: DiagnosticPosition {
                            line: range.map(|r| r.line).unwrap_or(0),
                            character: range.map(|r| r.column).unwrap_or(0),
                        },
                    },
                    severity: pb_diag.severity,
                    message: pb_diag.message,
                    code: if pb_diag.code.is_empty() {
                        None
                    } else {
                        Some(pb_diag.code)
                    },
                    assembly: pb_diag.assembly,
                    source: pb_diag.source,
                    fingerprint: pb_diag.fingerprint,
                    first_seen: if pb_diag.first_seen.is_empty() {
                        None
                    } else {
                        Some(pb_diag.first_seen)
                    },
                    last_seen: if pb_diag.last_seen.is_empty() {
                        None
                    } else {
                        Some(pb_diag.last_seen)
                    },
                }
            })
            .collect();

        let summary = DiagnosticSummary {
            errors: diagnostics_response
                .summary
                .as_ref()
                .map(|s| s.errors)
                .unwrap_or(0),
            warnings: diagnostics_response
                .summary
                .as_ref()
                .map(|s| s.warnings)
                .unwrap_or(0),
            infos: diagnostics_response
                .summary
                .as_ref()
                .map(|s| s.infos)
                .unwrap_or(0),
            assemblies: diagnostics_response
                .summary
                .as_ref()
                .map(|s| s.assemblies.clone())
                .unwrap_or_default(),
        };

        Ok(CompileDiagnostics {
            compile_id: diagnostics_response.compile_id,
            summary,
            diagnostics,
            truncated: diagnostics_response.truncated,
        })
    }

    fn calculate_summary(&self, diagnostics: &[Diagnostic]) -> DiagnosticSummary {
        let mut assemblies = HashSet::new();
        let mut errors = 0;
        let mut warnings = 0;
        let mut infos = 0;

        for diagnostic in diagnostics {
            assemblies.insert(diagnostic.assembly.clone());
            match diagnostic.severity.as_str() {
                "error" => errors += 1,
                "warning" => warnings += 1,
                "info" => infos += 1,
                _ => {}
            }
        }

        DiagnosticSummary {
            errors,
            warnings,
            infos,
            assemblies: assemblies.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    async fn create_test_service() -> McpService {
        // Create a test service using the actual constructor
        McpService::new()
            .await
            .expect("Failed to create test service")
    }

    fn create_test_diagnostics() -> CompileDiagnostics {
        CompileDiagnostics {
            compile_id: "test123".to_string(),
            summary: DiagnosticSummary {
                errors: 2,
                warnings: 1,
                infos: 0,
                assemblies: vec!["Assembly-CSharp".to_string()],
            },
            diagnostics: vec![
                Diagnostic {
                    file_uri: "file:///test/Foo.cs".to_string(),
                    range: DiagnosticRange {
                        start: DiagnosticPosition {
                            line: 10,
                            character: 5,
                        },
                        end: DiagnosticPosition {
                            line: 10,
                            character: 5,
                        },
                    },
                    severity: "error".to_string(),
                    message: "Test error message".to_string(),
                    code: Some("CS0103".to_string()),
                    assembly: "Assembly-CSharp".to_string(),
                    source: "Unity".to_string(),
                    fingerprint: "abc123".to_string(),
                    first_seen: Some("2025-08-30T12:00:00.000Z".to_string()),
                    last_seen: Some("2025-08-30T12:00:00.000Z".to_string()),
                },
                Diagnostic {
                    file_uri: "file:///test/Bar.cs".to_string(),
                    range: DiagnosticRange {
                        start: DiagnosticPosition {
                            line: 5,
                            character: 10,
                        },
                        end: DiagnosticPosition {
                            line: 5,
                            character: 10,
                        },
                    },
                    severity: "warning".to_string(),
                    message: "Test warning message".to_string(),
                    code: Some("CS0162".to_string()),
                    assembly: "Assembly-CSharp".to_string(),
                    source: "Unity".to_string(),
                    fingerprint: "def456".to_string(),
                    first_seen: Some("2025-08-30T12:00:00.000Z".to_string()),
                    last_seen: Some("2025-08-30T12:00:00.000Z".to_string()),
                },
                Diagnostic {
                    file_uri: "file:///test/Baz.cs".to_string(),
                    range: DiagnosticRange {
                        start: DiagnosticPosition {
                            line: 15,
                            character: 0,
                        },
                        end: DiagnosticPosition {
                            line: 15,
                            character: 0,
                        },
                    },
                    severity: "error".to_string(),
                    message: "Another test error".to_string(),
                    code: Some("CS1061".to_string()),
                    assembly: "Assembly-CSharp".to_string(),
                    source: "Unity".to_string(),
                    fingerprint: "ghi789".to_string(),
                    first_seen: Some("2025-08-30T12:00:00.000Z".to_string()),
                    last_seen: Some("2025-08-30T12:00:00.000Z".to_string()),
                },
            ],
            truncated: false,
        }
    }

    #[tokio::test]
    async fn test_calculate_summary() {
        let service = create_test_service().await;
        let diagnostics = create_test_diagnostics();

        let summary = service.calculate_summary(&diagnostics.diagnostics);

        assert_eq!(summary.errors, 2);
        assert_eq!(summary.warnings, 1);
        assert_eq!(summary.infos, 0);
        assert_eq!(summary.assemblies.len(), 1);
        assert!(summary.assemblies.contains(&"Assembly-CSharp".to_string()));
    }

    #[test]
    fn test_default_values() {
        let req = UnityGetCompileDiagnosticsRequest {
            max_items: default_max_items(),
            severity: default_severity(),
            changed_only: false,
            assembly: None,
        };

        assert_eq!(req.max_items, 500);
        assert_eq!(req.severity, "all");
        assert!(!req.changed_only);
        assert!(req.assembly.is_none());
    }
}
