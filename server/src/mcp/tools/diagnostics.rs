use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

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

        // Read diagnostics from file
        let diagnostics_path = self.get_diagnostics_path().await;
        let compile_diagnostics = match self.read_diagnostics_file(&diagnostics_path).await {
            Ok(diagnostics) => diagnostics,
            Err(e) => {
                tracing::warn!("Failed to read diagnostics file: {}", e);
                return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    format!(
                        "Unity compile diagnostics not available. Please trigger a compilation in Unity Editor first.\n\nError: {}",
                        e
                    ),
                )]));
            }
        };

        // Apply filters
        let total_before = compile_diagnostics.diagnostics.len();
        let filtered_diagnostics: Vec<Diagnostic> = compile_diagnostics
            .diagnostics
            .into_iter()
            .filter(|d| {
                if severity != "all" {
                    d.severity == severity
                } else {
                    true
                }
            })
            .filter(|d| match &assembly {
                Some(a) => &d.assembly == a,
                None => true,
            })
            .take(max_items as usize)
            .collect();

        let truncated = compile_diagnostics.truncated || filtered_diagnostics.len() < total_before;

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

    async fn get_diagnostics_path(&self) -> PathBuf {
        let project_root = { self.unity_project_path.read().await.clone() };

        if let Ok(env_path) = std::env::var("UNITY_MCP_DIAG_PATH") {
            let custom = PathBuf::from(env_path);
            if custom.is_absolute() {
                return custom;
            } else {
                return project_root.join(custom);
            }
        }

        project_root.join("Temp").join("AI").join("latest.json")
    }

    async fn read_diagnostics_file(&self, path: &Path) -> anyhow::Result<CompileDiagnostics> {
        let content = tokio::fs::read_to_string(path).await?;
        let diagnostics: CompileDiagnostics = serde_json::from_str(&content)?;
        Ok(diagnostics)
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

    #[tokio::test]
    async fn test_read_diagnostics_file_success() {
        let service = create_test_service().await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("diag.json");
        let diags = create_test_diagnostics();
        tokio::fs::write(&path, serde_json::to_string(&diags).unwrap())
            .await
            .unwrap();

        let read = service.read_diagnostics_file(&path).await.unwrap();
        assert_eq!(read.compile_id, diags.compile_id);
    }

    #[tokio::test]
    async fn test_read_diagnostics_file_missing() {
        let service = create_test_service().await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.json");
        let result = service.read_diagnostics_file(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_diagnostics_file_parse_error() {
        let service = create_test_service().await;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.json");
        tokio::fs::write(&path, "not json").await.unwrap();
        let result = service.read_diagnostics_file(&path).await;
        assert!(result.is_err());
    }
}
