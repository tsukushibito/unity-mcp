using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using UnityEditor;
using UnityEditor.TestTools.TestRunner.Api;
using UnityEngine;

namespace MCP.Editor
{
    [Serializable]
    public class TestRequest
    {
        public string runId;
        public string mode;
        public string testFilter;
        public string[] categories;
        public int timeoutSec;
        public int maxItems;
        public bool includePassed;
    }

    [Serializable]
    public class TestSummary
    {
        public int total;
        public int passed;
        public int failed;
        public int skipped;
        public float durationSec;
    }

    [Serializable]
    public class TestResult
    {
        public string assembly;
        public string suite;
        public string name;
        public string fullName;
        public string status;
        public float durationSec;
        public string message;
        public string stackTrace;
        public string[] categories;
        public string owner;
        public string file;
        public int line;
    }

    [Serializable]
    public class TestResults
    {
        public string runId;
        public string startedAt;
        public string finishedAt;
        public string mode;
        public string filter;
        public string[] categories;
        public TestSummary summary;
        public TestResult[] tests;
        public bool truncated;
    }

    [Serializable]
    public class TestRunStatus
    {
        public string status;
        public string runId;
        public string timestamp;
    }

    [InitializeOnLoad]
    public static class McpTestRunner
    {
        private static readonly string RequestDirectory = Path.Combine(Application.dataPath, "../Temp/AI/requests");
        private static readonly string OutputDirectory = Path.Combine(Application.dataPath, "../Temp/AI/tests");
        private static readonly string LatestJsonPath = Path.Combine(OutputDirectory, "latest.json");
        
        private static readonly Queue<TestRequest> pendingRequests = new Queue<TestRequest>();
        private static bool isRunning = false;
        private static TestRunnerApi testRunnerApi;
        private static TestResults currentResults;
        private static List<TestResult> collectedResults = new List<TestResult>();
        private static DateTime testStartTime;
        private static TestRequest currentRequest;

        static McpTestRunner()
        {
            testRunnerApi = TestRunnerApi.Instance;
            testRunnerApi.testStarted += OnTestStarted;
            testRunnerApi.testFinished += OnTestFinished;
            testRunnerApi.runStarted += OnRunStarted;
            testRunnerApi.runFinished += OnRunFinished;
            
            EditorApplication.update += Update;
            
            EnsureDirectories();
        }

        private static void EnsureDirectories()
        {
            try
            {
                if (!Directory.Exists(RequestDirectory))
                {
                    Directory.CreateDirectory(RequestDirectory);
                }
                if (!Directory.Exists(OutputDirectory))
                {
                    Directory.CreateDirectory(OutputDirectory);
                }
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to create directories: {e.Message}");
            }
        }

        private static void Update()
        {
            try
            {
                CheckForNewRequests();
                ProcessPendingRequests();
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Update error: {e.Message}");
            }
        }

        private static void CheckForNewRequests()
        {
            if (!Directory.Exists(RequestDirectory)) return;

            var requestFiles = Directory.GetFiles(RequestDirectory, "runTests-*.json");
            foreach (var file in requestFiles)
            {
                try
                {
                    var json = File.ReadAllText(file);
                    var request = JsonUtility.FromJson<TestRequest>(json);
                    
                    if (request != null)
                    {
                        pendingRequests.Enqueue(request);
                        File.Delete(file);
                        Debug.Log($"[McpTestRunner] Queued test request: {request.runId}");
                    }
                }
                catch (Exception e)
                {
                    Debug.LogError($"[McpTestRunner] Failed to parse request file {file}: {e.Message}");
                    try { File.Delete(file); } catch { }
                }
            }
        }

        private static void ProcessPendingRequests()
        {
            if (isRunning || pendingRequests.Count == 0) return;

            var request = pendingRequests.Dequeue();
            ExecuteTestRun(request);
        }

        private static void ExecuteTestRun(TestRequest request)
        {
            try
            {
                isRunning = true;
                currentRequest = request; // Store current request for later reference
                testStartTime = DateTime.UtcNow;
                
                // Initialize results structure
                currentResults = new TestResults
                {
                    runId = request.runId,
                    startedAt = testStartTime.ToString("yyyy-MM-ddTHH:mm:ss.fffZ"),
                    mode = request.mode,
                    filter = request.testFilter,
                    categories = request.categories ?? new string[0],
                    summary = new TestSummary(),
                    tests = new TestResult[0],
                    truncated = false
                };
                
                collectedResults.Clear();
                
                // Create execution settings
                var executionSettings = new ExecutionSettings();
                
                // Set test mode
                if (request.mode == "play" || request.mode == "all")
                {
                    executionSettings.runSynchronously = true;
                    // Note: targetPlatform removed for cross-platform compatibility
                    // Unity will use the default platform settings
                }
                
                // Apply filters
                var filter = new Filter
                {
                    testMode = GetTestMode(request.mode),
                    testNames = !string.IsNullOrEmpty(request.testFilter) ? 
                        new[] { request.testFilter } : null,
                    categoryNames = request.categories
                };

                // Set filters on execution settings
                executionSettings.filters = new[] { filter };

                // Write status file to indicate test started
                WriteStatusFile("started", currentResults);
                
                Debug.Log($"[McpTestRunner] Starting test run: {request.runId} (mode: {request.mode})");
                
                // Execute tests
                testRunnerApi.Execute(executionSettings);
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to execute test run: {e.Message}");
                
                // Create error results
                currentResults.finishedAt = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ");
                currentResults.summary = new TestSummary
                {
                    total = 0,
                    passed = 0,
                    failed = 1,
                    skipped = 0,
                    durationSec = 0
                };
                
                SaveResults(currentResults);
                WriteStatusFile("finished", currentResults);
                isRunning = false;
            }
        }

        private static TestMode GetTestMode(string mode)
        {
            return mode switch
            {
                "edit" => TestMode.EditMode,
                "play" => TestMode.PlayMode,
                "all" => TestMode.EditMode | TestMode.PlayMode,
                _ => TestMode.EditMode
            };
        }

        private static void OnRunStarted(ITestRunStarted runStarted)
        {
            Debug.Log($"[McpTestRunner] Test run started with {runStarted.executedTests.Count} tests");
        }

        private static void OnTestStarted(ITestStarted testStarted)
        {
            // Test started - could log here if needed
        }

        private static void OnTestFinished(ITestFinished testFinished)
        {
            try
            {
                var result = CreateTestResult(testFinished.test, testFinished.result);
                if (result != null)
                {
                    collectedResults.Add(result);
                }
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to process test result: {e.Message}");
            }
        }

        private static void OnRunFinished(ITestRunFinished runFinished)
        {
            try
            {
                Debug.Log($"[McpTestRunner] Test run finished");
                
                // Calculate final results
                var finishTime = DateTime.UtcNow;
                var duration = (float)(finishTime - testStartTime).TotalSeconds;
                
                currentResults.finishedAt = finishTime.ToString("yyyy-MM-ddTHH:mm:ss.fffZ");
                currentResults.summary = CalculateSummary(collectedResults, duration);
                
                // Apply result limits and filters
                var filteredResults = collectedResults.AsEnumerable();
                
                // Apply includePassed filter
                if (currentRequest != null && !currentRequest.includePassed)
                {
                    filteredResults = filteredResults.Where(r => r.status != "passed");
                }
                
                // Apply max items limit from request
                var maxItems = currentRequest?.maxItems ?? 2000;
                var totalCount = filteredResults.Count();
                currentResults.truncated = totalCount > maxItems;
                if (currentResults.truncated)
                {
                    filteredResults = filteredResults.Take(maxItems);
                }
                
                currentResults.tests = filteredResults.ToArray();
                
                // Save results
                SaveResults(currentResults);
                WriteStatusFile("finished", currentResults);
                
                Debug.Log($"[McpTestRunner] Results saved: {currentResults.summary.passed} passed, " +
                         $"{currentResults.summary.failed} failed, {currentResults.summary.skipped} skipped");
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to finalize test run: {e.Message}");
            }
            finally
            {
                isRunning = false;
            }
        }

        private static TestResult CreateTestResult(ITest test, ITestResult result)
        {
            try
            {
                var status = result.testStatus switch
                {
                    TestStatus.Passed => "passed",
                    TestStatus.Failed => "failed",
                    TestStatus.Skipped => "skipped",
                    TestStatus.Inconclusive => "inconclusive",
                    _ => "unknown"
                };

                // Extract file and line from stack trace
                var (file, line) = ExtractFileAndLine(result.stackTrace);
                
                return new TestResult
                {
                    assembly = ExtractAssemblyName(test.fullName),
                    suite = ExtractSuiteName(test.fullName),
                    name = test.name,
                    fullName = test.fullName,
                    status = status,
                    durationSec = (float)result.duration,
                    message = result.message ?? "",
                    stackTrace = result.stackTrace ?? "",
                    categories = test.categories?.ToArray() ?? new string[0],
                    owner = test.properties?.Get("Owner") as string ?? "",
                    file = file ?? "",
                    line = line
                };
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to create test result for {test?.name}: {e.Message}");
                return null;
            }
        }

        private static string ExtractAssemblyName(string fullName)
        {
            // Try to extract assembly name from full name
            if (string.IsNullOrEmpty(fullName)) return "";
            
            var parts = fullName.Split('.');
            if (parts.Length >= 2)
            {
                return $"{parts[0]}.{parts[1]}"; // e.g., "Game.EditModeTests"
            }
            
            return parts.Length > 0 ? parts[0] : "";
        }

        private static string ExtractSuiteName(string fullName)
        {
            if (string.IsNullOrEmpty(fullName)) return "";
            
            var lastDotIndex = fullName.LastIndexOf('.');
            var secondLastDotIndex = fullName.LastIndexOf('.', lastDotIndex - 1);
            
            if (secondLastDotIndex >= 0 && lastDotIndex > secondLastDotIndex)
            {
                return fullName.Substring(secondLastDotIndex + 1, lastDotIndex - secondLastDotIndex - 1);
            }
            
            return "";
        }

        private static (string file, int line) ExtractFileAndLine(string stackTrace)
        {
            if (string.IsNullOrEmpty(stackTrace)) return (null, 0);
            
            // Simple extraction - look for file paths in stack trace
            var lines = stackTrace.Split('\n');
            foreach (var line in lines)
            {
                if (line.Contains("Assets/") && line.Contains(":line "))
                {
                    var parts = line.Split(new[] { ":line " }, StringSplitOptions.None);
                    if (parts.Length >= 2)
                    {
                        var filePart = parts[0].Trim();
                        var linePart = parts[1].Trim();
                        
                        if (int.TryParse(linePart, out int lineNumber))
                        {
                            var fileStart = filePart.IndexOf("Assets/");
                            if (fileStart >= 0)
                            {
                                return (filePart.Substring(fileStart), lineNumber);
                            }
                        }
                    }
                }
            }
            
            return (null, 0);
        }

        private static TestSummary CalculateSummary(List<TestResult> results, float duration)
        {
            return new TestSummary
            {
                total = results.Count,
                passed = results.Count(r => r.status == "passed"),
                failed = results.Count(r => r.status == "failed"),
                skipped = results.Count(r => r.status == "skipped" || r.status == "inconclusive"),
                durationSec = duration
            };
        }

        private static void SaveResults(TestResults results)
        {
            try
            {
                EnsureDirectories();
                
                var json = JsonUtility.ToJson(results, false);
                
                // Write latest.json
                File.WriteAllText(LatestJsonPath, json);
                
                // Write run-specific file
                var runSpecificPath = Path.Combine(OutputDirectory, $"run-{results.runId}.json");
                File.WriteAllText(runSpecificPath, json);
                
                Debug.Log($"[McpTestRunner] Results written to {LatestJsonPath}");
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to save results: {e.Message}");
            }
        }

        private static void WriteStatusFile(string status, TestResults results)
        {
            try
            {
                var statusPath = Path.Combine(OutputDirectory, "status.json");
                var statusData = new TestRunStatus
                {
                    status = status,
                    runId = results.runId,
                    timestamp = DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
                };
                var statusJson = JsonUtility.ToJson(statusData, false);
                File.WriteAllText(statusPath, statusJson);
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to write status file: {e.Message}");
            }
        }
    }
}