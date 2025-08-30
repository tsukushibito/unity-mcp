use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

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

        // Read diagnostics from latest.json
        let diagnostics_path = self.get_diagnostics_path();
        let compile_diagnostics = match self.read_diagnostics_file(&diagnostics_path).await {
            Ok(diagnostics) => diagnostics,
            Err(e) => {
                tracing::warn!("Failed to read diagnostics file: {}", e);
                return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    format!(
                        "Unity compile diagnostics not available. Please save a script file in Unity Editor to trigger compilation and generate diagnostics.\n\nError: {}",
                        e
                    ),
                )]));
            }
        };

        // Apply filters
        let mut filtered_diagnostics = compile_diagnostics.diagnostics;

        // Filter by severity
        if severity != "all" {
            filtered_diagnostics.retain(|d| d.severity == severity);
        }

        // Filter by assembly
        if let Some(ref assembly_filter) = assembly {
            filtered_diagnostics.retain(|d| d.assembly == *assembly_filter);
        }

        // TODO: Implement changed_only filter when we have historical data
        if changed_only {
            tracing::warn!("changed_only filter not yet implemented in MVP");
        }

        // Apply max_items limit and set truncated flag
        let total_count = filtered_diagnostics.len();
        let truncated = total_count > max_items as usize;
        if truncated {
            filtered_diagnostics.truncate(max_items as usize);
        }

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

    fn get_diagnostics_path(&self) -> std::path::PathBuf {
        // Use environment variable override if provided
        if let Ok(env_path) = std::env::var("UNITY_MCP_DIAG_PATH") {
            return std::path::PathBuf::from(env_path);
        }

        // Default: Use CARGO_MANIFEST_DIR/../bridge/Temp/AI/latest.json
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("bridge")
            .join("Temp")
            .join("AI")
            .join("latest.json")
    }

    async fn read_diagnostics_file(&self, path: &Path) -> anyhow::Result<CompileDiagnostics> {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Diagnostics file does not exist: {}. Run a Unity compilation first.",
                path.display()
            ));
        }

        // Security check: ensure the path is within our allowed directory
        let canonical_path = path.canonicalize()?;
        let bridge_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("bridge")
            .canonicalize()
            .unwrap_or_else(|_| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .join("bridge")
            });

        if !canonical_path.starts_with(&bridge_path) {
            return Err(anyhow::anyhow!(
                "Access denied: path outside bridge directory: {}",
                canonical_path.display()
            ));
        }

        let content = tokio::fs::read_to_string(path).await?;

        // Check file size (limit to ~2MB)
        const MAX_FILE_SIZE: usize = 2 * 1024 * 1024;
        if content.len() > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!(
                "Diagnostics file too large ({} bytes). Use filters to reduce the result set.",
                content.len()
            ));
        }

        let diagnostics: CompileDiagnostics = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in diagnostics file: {}", e))?;

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
    use tempfile::TempDir;
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
    async fn test_file_size_limit() {
        let service = create_test_service().await;

        // Create test directory within bridge/Temp/AI
        let bridge_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("bridge");
        let test_dir = bridge_path.join("Temp").join("AI").join("test");
        tokio::fs::create_dir_all(&test_dir).await.unwrap();

        let large_file_path = test_dir.join("large.json");

        // Create a file larger than 2MB
        let large_content = "a".repeat(3 * 1024 * 1024);
        tokio::fs::write(&large_file_path, large_content)
            .await
            .unwrap();

        let result = service.read_diagnostics_file(&large_file_path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));

        // Cleanup
        tokio::fs::remove_file(&large_file_path).await.ok();
    }

    #[tokio::test]
    async fn test_file_not_exists() {
        let service = create_test_service().await;

        // Use a path within bridge directory but that doesn't exist
        let bridge_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("bridge");
        let non_existent_path = bridge_path.join("non_existent_path.json");

        let result = service.read_diagnostics_file(&non_existent_path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[tokio::test]
    async fn test_security_path_outside_bridge() {
        let service = create_test_service().await;

        // Create a temporary file outside bridge directory
        let temp_dir = TempDir::new().unwrap();
        let outside_path = temp_dir.path().join("test.json");
        tokio::fs::write(&outside_path, "{}").await.unwrap();

        let result = service.read_diagnostics_file(&outside_path).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Access denied")
                || error_msg.contains("path outside bridge directory")
        );
    }
}
