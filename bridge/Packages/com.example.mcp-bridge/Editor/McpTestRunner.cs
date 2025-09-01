using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using UnityEditor;
using UnityEditor.TestTools.TestRunner.Api;
using UnityEngine;
using Bridge.Editor;

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
        private static readonly string RequestDirectory = McpFilePathManager.GetTestsRequestsDirectory();
        private static readonly string OutputDirectory = McpFilePathManager.GetTestsDirectory();
        private static readonly string LatestJsonPath = McpFilePathManager.GetLatestJsonPath(OutputDirectory);
        
        // Phase 2: IPC and file output configuration
        private static bool EnableIpcCommunication = true;  // Enable IPC communication (Phase 2)
        private static bool EnableFileOutput = true;        // Keep file output for debugging
        
        private static readonly Queue<TestRequest> pendingRequests = new Queue<TestRequest>();
        private static bool isRunning = false;
        private static TestRunnerApi testRunnerApi;
        private static TestResults currentResults;
        private static List<TestResult> collectedResults = new List<TestResult>();
        private static DateTime testStartTime;
        private static TestRequest currentRequest;
        // For sequential execution when mode == "all"
        private static Queue<TestMode> phaseQueue;
        private static bool multiPhase;

        static McpTestRunner()
        {
            // Instantiate API and register callbacks (no static Instance in UTF)
            testRunnerApi = ScriptableObject.CreateInstance<TestRunnerApi>();
            testRunnerApi.RegisterCallbacks(new CallbackReceiver());
            
            EditorApplication.update += Update;
            
            EnsureDirectories();
        }

        private static void EnsureDirectories()
        {
            try
            {
                McpFilePathManager.EnsureDirectoryExists(RequestDirectory);
                McpFilePathManager.EnsureDirectoryExists(OutputDirectory);
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
                
                // Phase 2: Send IPC notification about test run acceptance
                if (EnableIpcCommunication)
                {
                    SendIpcTestRunAccepted(request);
                }
                
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
                
                // Decide phases
                multiPhase = request.mode == "all";
                phaseQueue = new Queue<TestMode>();
                if (multiPhase)
                {
                    phaseQueue.Enqueue(TestMode.EditMode);
                    phaseQueue.Enqueue(TestMode.PlayMode);
                }
                else
                {
                    phaseQueue.Enqueue(GetTestMode(request.mode));
                }

                // Phase 2: Write status file to indicate test started (optional for debugging)
                if (EnableFileOutput)
                {
                    WriteStatusFile("started", currentResults);
                }

                Debug.Log($"[McpTestRunner] Starting test run: {request.runId} (mode: {request.mode})");

                // Kick off first phase
                ExecuteNextPhase();
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
                
                // Phase 2: Optional file output for error cases
                if (EnableFileOutput)
                {
                    SaveResults(currentResults);
                    WriteStatusFile("finished", currentResults);
                }
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

        private static void ExecuteNextPhase()
        {
            if (phaseQueue == null || phaseQueue.Count == 0)
            {
                Debug.Log("[McpTestRunner] No more phases to execute.");
                return;
            }

            var phase = phaseQueue.Dequeue();

            var executionSettings = new ExecutionSettings();
            // For play mode execution, run synchronously for determinism
            if ((phase & TestMode.PlayMode) == TestMode.PlayMode)
            {
                executionSettings.runSynchronously = true;
            }

            var filter = new Filter
            {
                testMode = phase,
                testNames = !string.IsNullOrEmpty(currentRequest?.testFilter) ?
                    new[] { currentRequest.testFilter } : null,
                categoryNames = currentRequest?.categories
            };
            executionSettings.filters = new[] { filter };

            Debug.Log($"[McpTestRunner] Executing phase: {phase}");
            testRunnerApi.Execute(executionSettings);
        }

        private static void OnRunStarted(ITestAdaptor runStarted)
        {
            try
            {
                // ITestAdaptor may represent a suite; children count may not be loaded yet
                Debug.Log($"[McpTestRunner] Test run started: {runStarted.FullName}");
            }
            catch (Exception)
            {
                Debug.Log("[McpTestRunner] Test run started");
            }
        }

        private static void OnTestStarted(ITestAdaptor testStarted)
        {
            // Test started - could log here if needed
        }

        private static void OnTestFinished(ITestResultAdaptor testFinished)
        {
            try
            {
                var result = CreateTestResult(testFinished.Test, testFinished);
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

        private static void OnRunFinished(ITestResultAdaptor runFinished)
        {
            try
            {
                Debug.Log($"[McpTestRunner] Test phase finished");

                // If multi-phase and there are remaining phases, continue accumulating and start next phase
                if (multiPhase && phaseQueue != null && phaseQueue.Count > 0)
                {
                    // Just keep accumulating results; do not finalize yet
                    Debug.Log("[McpTestRunner] Starting next phase...");
                    ExecuteNextPhase();
                    return;
                }

                // Finalize combined results
                var finishTime = DateTime.UtcNow;
                var duration = (float)(finishTime - testStartTime).TotalSeconds;

                currentResults.finishedAt = finishTime.ToString("yyyy-MM-ddTHH:mm:ss.fffZ");
                currentResults.summary = CalculateSummary(collectedResults, duration);

                // Apply result limits and filters
                var filteredResults = collectedResults.AsEnumerable();

                if (currentRequest != null && !currentRequest.includePassed)
                {
                    filteredResults = filteredResults.Where(r => r.status != "passed");
                }

                var maxItems = currentRequest?.maxItems ?? 2000;
                var totalCount = filteredResults.Count();
                currentResults.truncated = totalCount > maxItems;
                if (currentResults.truncated)
                {
                    filteredResults = filteredResults.Take(maxItems);
                }

                currentResults.tests = filteredResults.ToArray();

                // Phase 2: Send IPC notification with results (primary communication)
                if (EnableIpcCommunication)
                {
                    SendIpcTestResultsReady(currentResults);
                }
                
                // Phase 2: Optionally save results and status files (for debugging)
                if (EnableFileOutput)
                {
                    SaveResults(currentResults);
                    WriteStatusFile("finished", currentResults);
                }

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
                // Reset phase state
                phaseQueue = null;
                multiPhase = false;
            }
        }

        private static TestResult CreateTestResult(ITestAdaptor test, ITestResultAdaptor result)
        {
            try
            {
                var status = result.TestStatus switch
                {
                    TestStatus.Passed => "passed",
                    TestStatus.Failed => "failed",
                    TestStatus.Skipped => "skipped",
                    TestStatus.Inconclusive => "inconclusive",
                    _ => "unknown"
                };

                // Extract file and line from stack trace
                var (file, line) = ExtractFileAndLine(result.StackTrace);
                
                return new TestResult
                {
                    assembly = ExtractAssemblyName(test.FullName),
                    suite = ExtractSuiteName(test.FullName),
                    name = test.Name,
                    fullName = test.FullName,
                    status = status,
                    durationSec = (float)result.Duration,
                    message = result.Message ?? "",
                    stackTrace = result.StackTrace ?? "",
                    categories = GetCategories(test),
                    owner = "",
                    file = file ?? "",
                    line = line
                };
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to create test result for {test?.Name}: {e.Message}");
                return null;
            }
        }

        private static string[] GetCategories(ITestAdaptor test)
        {
            try
            {
                var cats = test.Categories;
                if (cats == null) return new string[0];
                // Categories may be IEnumerable<string> or string[] depending on Unity version
                return cats.ToArray();
            }
            catch
            {
                return new string[0];
            }
        }

        // Callback receiver bridging Unity Test Framework callbacks to our static handlers
        private class CallbackReceiver : ICallbacks
        {
            public void RunStarted(ITestAdaptor tests)
            {
                OnRunStarted(tests);
            }

            public void RunFinished(ITestResultAdaptor result)
            {
                OnRunFinished(result);
            }

            public void TestStarted(ITestAdaptor test)
            {
                OnTestStarted(test);
            }

            public void TestFinished(ITestResultAdaptor result)
            {
                OnTestFinished(result);
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

                // Also write per-run status to ease debugging parallel runs
                var statusPerRunPath = Path.Combine(OutputDirectory, $"status-{results.runId}.json");
                File.WriteAllText(statusPerRunPath, statusJson);
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to write status file: {e.Message}");
            }
        }
        
        // === Phase 2: IPC Communication Methods ===
        
        /// <summary>
        /// Send IPC notification that test run was accepted and started
        /// </summary>
        private static void SendIpcTestRunAccepted(TestRequest request)
        {
            try
            {
                // TODO: Implement actual IPC communication to Rust server
                // For now, just log the intent
                Debug.Log($"[McpTestRunner] IPC: Test run accepted - {request.runId} (mode: {request.mode})");
                
                // In Phase 2, this would send a RunTestsResponse via IPC
                // var response = new RunTestsResponse
                // {
                //     runId = request.runId,
                //     accepted = true,
                //     message = "Test run accepted"
                // };
                // IpcManager.SendResponse(response);
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to send IPC test run accepted: {e.Message}");
            }
        }
        
        /// <summary>
        /// Send IPC notification that test results are ready
        /// </summary>
        private static void SendIpcTestResultsReady(TestResults results)
        {
            try
            {
                // TODO: Implement actual IPC communication to Rust server
                // For now, just log the intent
                Debug.Log($"[McpTestRunner] IPC: Test results ready - {results.runId} " +
                         $"({results.summary.passed} passed, {results.summary.failed} failed)");
                
                // In Phase 2, this would send TestResults via IPC
                // var ipcResults = ConvertToIpcTestResults(results);
                // IpcManager.SendTestResults(ipcResults);
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to send IPC test results: {e.Message}");
            }
        }
        
        /// <summary>
        /// Handle incoming IPC test run requests (Phase 2)
        /// </summary>
        public static void HandleIpcRunTestsRequest(string runId, string mode, string testFilter, 
                                                   string[] categories, int timeoutSec, int maxItems, bool includePassed)
        {
            try
            {
                Debug.Log($"[McpTestRunner] IPC: Received test run request - {runId}");
                
                // Convert IPC request to internal TestRequest format
                var request = new TestRequest
                {
                    runId = runId,
                    mode = mode,
                    testFilter = testFilter ?? "",
                    categories = categories ?? new string[0],
                    timeoutSec = timeoutSec,
                    maxItems = maxItems,
                    includePassed = includePassed
                };
                
                // Queue the request for processing (reuse existing infrastructure)
                pendingRequests.Enqueue(request);
                Debug.Log($"[McpTestRunner] IPC request queued: {request.runId}");
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to handle IPC test run request: {e.Message}");
            }
        }
        
        /// <summary>
        /// Handle incoming IPC test results requests (Phase 2)
        /// </summary>
        public static TestResults HandleIpcGetTestResultsRequest(string runId, int maxItems, bool includePassed)
        {
            try
            {
                Debug.Log($"[McpTestRunner] IPC: Received get test results request - runId={runId}");
                
                // Try to get cached results first (from current run)
                if (currentResults != null && 
                    (string.IsNullOrEmpty(runId) || currentResults.runId == runId))
                {
                    return ApplyTestResultsFiltering(currentResults, maxItems, includePassed);
                }
                
                // Fallback: Try to load from file if available and file output is enabled
                if (EnableFileOutput)
                {
                    var resultsPath = string.IsNullOrEmpty(runId) 
                        ? LatestJsonPath 
                        : Path.Combine(OutputDirectory, $"run-{runId}.json");
                        
                    if (File.Exists(resultsPath))
                    {
                        var json = File.ReadAllText(resultsPath);
                        var results = JsonUtility.FromJson<TestResults>(json);
                        return ApplyTestResultsFiltering(results, maxItems, includePassed);
                    }
                }
                
                Debug.Log($"[McpTestRunner] IPC: No test results found for runId={runId}");
                return null;
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpTestRunner] Failed to handle IPC get test results request: {e.Message}");
                return null;
            }
        }
        
        /// <summary>
        /// Apply filtering to test results (helper method)
        /// </summary>
        private static TestResults ApplyTestResultsFiltering(TestResults results, int maxItems, bool includePassed)
        {
            if (results == null) return null;
            
            var filteredTests = results.tests.AsEnumerable();
            
            // Apply include_passed filter
            if (!includePassed)
            {
                filteredTests = filteredTests.Where(t => t.status != "passed");
            }
            
            // Apply max_items limit
            var testsList = filteredTests.ToList();
            bool truncated = testsList.Count > maxItems;
            if (truncated)
            {
                testsList = testsList.Take(maxItems).ToList();
            }
            
            // Create filtered copy
            return new TestResults
            {
                runId = results.runId,
                startedAt = results.startedAt,
                finishedAt = results.finishedAt,
                mode = results.mode,
                filter = results.filter,
                categories = results.categories,
                summary = results.summary,
                tests = testsList.ToArray(),
                truncated = truncated || results.truncated
            };
        }
    }
}
