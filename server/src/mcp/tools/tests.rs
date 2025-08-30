use crate::mcp::service::McpService;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityRunTestsRequest {
    #[serde(default = "default_mode")]
    pub mode: String,

    #[serde(default)]
    pub test_filter: Option<String>,

    #[serde(default)]
    pub categories: Option<Vec<String>>,

    #[serde(default = "default_timeout_sec")]
    pub timeout_sec: u32,

    #[serde(default = "default_max_items")]
    pub max_items: u32,

    #[serde(default = "default_include_passed")]
    pub include_passed: bool,
}

fn default_mode() -> String {
    "edit".to_string()
}

fn default_timeout_sec() -> u32 {
    180
}

fn default_max_items() -> u32 {
    2000
}

fn default_include_passed() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityGetTestResultsRequest {
    #[serde(default)]
    pub run_id: Option<String>,

    #[serde(default = "default_max_items")]
    pub max_items: u32,

    #[serde(default = "default_include_passed")]
    pub include_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    #[serde(rename = "durationSec")]
    pub duration_sec: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestResult {
    pub assembly: String,
    pub suite: String,
    pub name: String,
    #[serde(rename = "fullName")]
    pub full_name: String,
    pub status: String,
    #[serde(rename = "durationSec")]
    pub duration_sec: f64,
    pub message: String,
    #[serde(rename = "stackTrace")]
    pub stack_trace: String,
    pub categories: Vec<String>,
    pub owner: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestResults {
    #[serde(rename = "runId")]
    pub run_id: String,
    #[serde(rename = "startedAt")]
    pub started_at: String,
    #[serde(rename = "finishedAt")]
    pub finished_at: Option<String>,
    pub mode: String,
    pub filter: String,
    pub categories: Vec<String>,
    pub summary: TestSummary,
    pub tests: Vec<TestResult>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UnityRunTestsResponse {
    pub run_id: String,
    pub summary: TestSummary,
    pub tests: Vec<TestResult>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TestRequest {
    #[serde(rename = "runId")]
    pub run_id: String,
    pub mode: String,
    #[serde(rename = "testFilter")]
    pub test_filter: String,
    pub categories: Vec<String>,
    #[serde(rename = "timeoutSec")]
    pub timeout_sec: u32,
    #[serde(rename = "maxItems")]
    pub max_items: u32,
    #[serde(rename = "includePassed")]
    pub include_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StatusFile {
    pub status: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    pub timestamp: String,
}

impl McpService {
    pub async fn do_unity_run_tests(
        &self,
        mode: String,
        test_filter: Option<String>,
        categories: Option<Vec<String>>,
        timeout_sec: u32,
        max_items: u32,
        include_passed: bool,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "Running Unity tests with mode={}, filter={:?}, categories={:?}, timeout={}s",
            mode,
            test_filter,
            categories,
            timeout_sec
        );

        // Generate unique run ID
        let run_id = self.generate_run_id();

        // Create test request
        let request = TestRequest {
            run_id: run_id.clone(),
            mode: mode.clone(),
            test_filter: test_filter.clone().unwrap_or_default(),
            categories: categories.clone().unwrap_or_default(),
            timeout_sec,
            max_items,
            include_passed,
        };

        // Write request file
        let request_path = self.get_requests_path();
        let request_file = request_path.join(format!("runTests-{}.json", run_id));

        match self.write_request_file(&request_file, &request).await {
            Ok(_) => tracing::info!("Test request written to: {}", request_file.display()),
            Err(e) => {
                tracing::error!("Failed to write test request: {}", e);
                return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    format!(
                        "Failed to write test request. Please ensure Unity Editor is running and MCP Bridge is installed.\n\nError: {}",
                        e
                    ),
                )]));
            }
        }

        // Send MCP notification for test started
        self.send_test_started_notification(&run_id, &mode, &test_filter, &categories)
            .await;

        // Wait for test completion
        match self.wait_for_test_completion(&run_id, timeout_sec).await {
            Ok(results) => {
                tracing::info!(
                    "Test run completed: {} tests ({} passed, {} failed, {} skipped)",
                    results.summary.total,
                    results.summary.passed,
                    results.summary.failed,
                    results.summary.skipped
                );

                // Send MCP notification for test finished
                self.send_test_finished_notification(&run_id, &results.summary, results.truncated)
                    .await;

                // Apply filters and limits
                let filtered_tests =
                    self.apply_test_filters(&results.tests, max_items, include_passed);
                let truncated = filtered_tests.len() < results.tests.len() || results.truncated;

                let response = UnityRunTestsResponse {
                    run_id: results.run_id,
                    summary: results.summary,
                    tests: filtered_tests,
                    truncated,
                };

                let json_value = serde_json::to_value(response).map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("Failed to serialize response: {}", e),
                        None,
                    )
                })?;
                Ok(CallToolResult::structured(json_value))
            }
            Err(e) => {
                tracing::error!("Test run failed or timed out: {}", e);
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    format!(
                        "Test run failed or timed out after {}s. Please ensure Unity Editor is running and not busy.\n\nError: {}",
                        timeout_sec, e
                    ),
                )]))
            }
        }
    }

    pub async fn do_unity_get_test_results(
        &self,
        run_id: Option<String>,
        max_items: u32,
        include_passed: bool,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            "Getting Unity test results for run_id={:?}, max_items={}, include_passed={}",
            run_id,
            max_items,
            include_passed
        );

        let results_path = if let Some(id) = run_id.as_ref() {
            self.get_tests_path().join(format!("run-{}.json", id))
        } else {
            self.get_tests_path().join("latest.json")
        };

        match self.read_test_results_file(&results_path).await {
            Ok(results) => {
                let filtered_tests =
                    self.apply_test_filters(&results.tests, max_items, include_passed);
                let truncated = filtered_tests.len() < results.tests.len() || results.truncated;

                let response = UnityRunTestsResponse {
                    run_id: results.run_id,
                    summary: results.summary,
                    tests: filtered_tests,
                    truncated,
                };

                let json_value = serde_json::to_value(response).map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("Failed to serialize response: {}", e),
                        None,
                    )
                })?;
                Ok(CallToolResult::structured(json_value))
            }
            Err(e) => {
                tracing::warn!("Failed to read test results file: {}", e);
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    format!(
                        "Unity test results not available. Please run tests first using unity_run_tests.\n\nError: {}",
                        e
                    ),
                )]))
            }
        }
    }

    fn generate_run_id(&self) -> String {
        let now = chrono::Utc::now();
        let uuid_short = &Uuid::new_v4().to_string()[..8];
        format!("{}-{}", now.format("%Y-%m-%dT%H:%M:%SZ"), uuid_short)
    }

    fn get_requests_path(&self) -> std::path::PathBuf {
        if let Ok(env_path) = std::env::var("UNITY_MCP_REQ_PATH") {
            return std::path::PathBuf::from(env_path);
        }

        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("bridge")
            .join("Temp")
            .join("AI")
            .join("requests")
    }

    fn get_tests_path(&self) -> std::path::PathBuf {
        if let Ok(env_path) = std::env::var("UNITY_MCP_TESTS_PATH") {
            return std::path::PathBuf::from(env_path);
        }

        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("bridge")
            .join("Temp")
            .join("AI")
            .join("tests")
    }

    async fn write_request_file(&self, path: &Path, request: &TestRequest) -> anyhow::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Security check: ensure the path is within our allowed directory
        let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
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

        let json_content = serde_json::to_string(request)?;
        tokio::fs::write(path, json_content).await?;

        Ok(())
    }

    async fn read_test_results_file(&self, path: &Path) -> anyhow::Result<TestResults> {
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Test results file does not exist: {}. Run tests first using unity_run_tests.",
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
                "Test results file too large ({} bytes). Use filters to reduce the result set.",
                content.len()
            ));
        }

        let results: TestResults = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in test results file: {}", e))?;

        Ok(results)
    }

    async fn wait_for_test_completion(
        &self,
        run_id: &str,
        timeout_sec: u32,
    ) -> anyhow::Result<TestResults> {
        let status_path = self.get_tests_path().join("status.json");
        let per_run_status_path = self
            .get_tests_path()
            .join(format!("status-{}.json", run_id));
        let results_path = self.get_tests_path().join("latest.json");

        let timeout_duration = tokio::time::Duration::from_secs(timeout_sec as u64);
        let start_time = tokio::time::Instant::now();
        let poll_interval = tokio::time::Duration::from_millis(500);

        loop {
            // Check timeout
            if start_time.elapsed() > timeout_duration {
                return Err(anyhow::anyhow!(
                    "Test execution timed out after {} seconds",
                    timeout_sec
                ));
            }

            // Prefer per-run status file when available (helps when multiple runs occur)
            if per_run_status_path.exists()
                && let Ok(status_content) = tokio::fs::read_to_string(&per_run_status_path).await
                && let Ok(status) = serde_json::from_str::<StatusFile>(&status_content)
                && status.status == "finished"
                && status.run_id == run_id
            {
                return self.read_test_results_file(&results_path).await;
            } else if status_path.exists()
                && let Ok(status_content) = tokio::fs::read_to_string(&status_path).await
                && let Ok(status) = serde_json::from_str::<StatusFile>(&status_content)
                && status.run_id == run_id
                && status.status == "finished"
            {
                return self.read_test_results_file(&results_path).await;
            }

            // Wait before next poll
            tokio::time::sleep(poll_interval).await;
        }
    }

    fn apply_test_filters(
        &self,
        tests: &[TestResult],
        max_items: u32,
        include_passed: bool,
    ) -> Vec<TestResult> {
        let mut filtered: Vec<TestResult> = tests.to_vec();

        // Apply include_passed filter
        if !include_passed {
            filtered.retain(|test| test.status != "passed");
        }

        // Apply max_items limit
        if filtered.len() > max_items as usize {
            filtered.truncate(max_items as usize);
        }

        filtered
    }

    async fn send_test_started_notification(
        &self,
        run_id: &str,
        mode: &str,
        filter: &Option<String>,
        categories: &Option<Vec<String>>,
    ) {
        // Log notification instead of actual MCP notification for MVP
        // TODO: Implement actual MCP notification in future version
        tracing::info!(
            "[MCP Notification] unity.tests.started: run_id={}, mode={}, filter={:?}, categories={:?}",
            run_id,
            mode,
            filter,
            categories
        );
    }

    async fn send_test_finished_notification(
        &self,
        run_id: &str,
        summary: &TestSummary,
        truncated: bool,
    ) {
        // Log notification instead of actual MCP notification for MVP
        // TODO: Implement actual MCP notification in future version
        tracing::info!(
            "[MCP Notification] unity.tests.finished: run_id={}, total={}, passed={}, failed={}, skipped={}, truncated={}",
            run_id,
            summary.total,
            summary.passed,
            summary.failed,
            summary.skipped,
            truncated
        );
    }
}

#[cfg(test)]
mod test_runner_tests {
    use super::*;
    use tempfile::TempDir;
    use tokio;

    async fn create_test_service() -> McpService {
        McpService::new()
            .await
            .expect("Failed to create test service")
    }

    fn create_test_results() -> TestResults {
        TestResults {
            run_id: "test123".to_string(),
            started_at: "2025-08-30T12:00:00.000Z".to_string(),
            finished_at: Some("2025-08-30T12:00:05.000Z".to_string()),
            mode: "edit".to_string(),
            filter: "".to_string(),
            categories: vec![],
            summary: TestSummary {
                total: 3,
                passed: 2,
                failed: 1,
                skipped: 0,
                duration_sec: 5.0,
            },
            tests: vec![
                TestResult {
                    assembly: "Test.Assembly".to_string(),
                    suite: "TestSuite".to_string(),
                    name: "PassingTest".to_string(),
                    full_name: "Test.Assembly.TestSuite.PassingTest".to_string(),
                    status: "passed".to_string(),
                    duration_sec: 1.0,
                    message: "".to_string(),
                    stack_trace: "".to_string(),
                    categories: vec!["fast".to_string()],
                    owner: "".to_string(),
                    file: "Assets/Tests/TestSuite.cs".to_string(),
                    line: 10,
                },
                TestResult {
                    assembly: "Test.Assembly".to_string(),
                    suite: "TestSuite".to_string(),
                    name: "FailingTest".to_string(),
                    full_name: "Test.Assembly.TestSuite.FailingTest".to_string(),
                    status: "failed".to_string(),
                    duration_sec: 2.0,
                    message: "Test failed".to_string(),
                    stack_trace: "at Assets/Tests/TestSuite.cs:line 20".to_string(),
                    categories: vec!["slow".to_string()],
                    owner: "".to_string(),
                    file: "Assets/Tests/TestSuite.cs".to_string(),
                    line: 20,
                },
                TestResult {
                    assembly: "Test.Assembly".to_string(),
                    suite: "TestSuite".to_string(),
                    name: "AnotherPassingTest".to_string(),
                    full_name: "Test.Assembly.TestSuite.AnotherPassingTest".to_string(),
                    status: "passed".to_string(),
                    duration_sec: 2.0,
                    message: "".to_string(),
                    stack_trace: "".to_string(),
                    categories: vec!["fast".to_string()],
                    owner: "".to_string(),
                    file: "Assets/Tests/TestSuite.cs".to_string(),
                    line: 30,
                },
            ],
            truncated: false,
        }
    }

    #[tokio::test]
    async fn test_generate_run_id() {
        let service = create_test_service().await;
        let run_id = service.generate_run_id();

        // Validate format: YYYY-MM-DDTHH:MM:SSZ-xxxxxxxx
        let regex_pattern = r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z-[a-f0-9]{8}$";
        let regex = regex::Regex::new(regex_pattern).unwrap();

        assert!(
            regex.is_match(&run_id),
            "run_id format should match ISO8601 + 8-char hash: {}",
            run_id
        );
        assert!(
            run_id.len() >= 28,
            "run_id should be at least 28 characters long: {}",
            run_id
        );
    }

    #[test]
    fn test_default_values() {
        let req = UnityRunTestsRequest {
            mode: default_mode(),
            test_filter: None,
            categories: None,
            timeout_sec: default_timeout_sec(),
            max_items: default_max_items(),
            include_passed: default_include_passed(),
        };

        assert_eq!(req.mode, "edit");
        assert_eq!(req.timeout_sec, 180);
        assert_eq!(req.max_items, 2000);
        assert!(req.include_passed);
    }

    #[tokio::test]
    async fn test_apply_test_filters() {
        let service = create_test_service().await;
        let results = create_test_results();

        // Test include_passed = false
        let filtered = service.apply_test_filters(&results.tests, 1000, false);
        assert_eq!(filtered.len(), 1); // Only failed test
        assert_eq!(filtered[0].status, "failed");

        // Test include_passed = true
        let filtered = service.apply_test_filters(&results.tests, 1000, true);
        assert_eq!(filtered.len(), 3); // All tests

        // Test max_items limit
        let filtered = service.apply_test_filters(&results.tests, 2, true);
        assert_eq!(filtered.len(), 2); // Limited to 2
    }

    #[tokio::test]
    async fn test_file_size_limit() {
        let service = create_test_service().await;

        // Create test directory within bridge/Temp/AI/tests
        let bridge_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("bridge");
        let test_dir = bridge_path
            .join("Temp")
            .join("AI")
            .join("tests")
            .join("test");
        tokio::fs::create_dir_all(&test_dir).await.unwrap();

        let large_file_path = test_dir.join("large.json");

        // Create a file larger than 2MB
        let large_content = "a".repeat(3 * 1024 * 1024);
        tokio::fs::write(&large_file_path, large_content)
            .await
            .unwrap();

        let result = service.read_test_results_file(&large_file_path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large"));

        // Cleanup
        tokio::fs::remove_file(&large_file_path).await.ok();
    }

    #[tokio::test]
    async fn test_file_not_exists() {
        let service = create_test_service().await;

        let bridge_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("bridge");
        let non_existent_path = bridge_path.join("non_existent_test.json");

        let result = service.read_test_results_file(&non_existent_path).await;
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

        let result = service.read_test_results_file(&outside_path).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Access denied")
                || error_msg.contains("path outside bridge directory")
        );
    }
}
