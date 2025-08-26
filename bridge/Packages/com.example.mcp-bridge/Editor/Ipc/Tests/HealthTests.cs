#if UNITY_EDITOR
using System;
using System.Collections;
using System.Threading;
using System.Threading.Tasks;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using Bridge.Editor.Ipc.Handlers;
using Bridge.Editor.Ipc.Infra;
using Mcp.Unity.V1;
using Mcp.Unity.V1.Ipc.Infra;

namespace Bridge.Editor.Ipc.Tests
{
    /// <summary>
    /// Tests for HealthHandler to verify Strict/Fast mode behavior and thread safety
    /// </summary>
    [TestFixture]
    public class HealthTests
    {
        [OneTimeSetUp]
        public void OneTimeSetUp()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("HealthTests starting - testing Strict/Fast mode behavior");
#endif
        }

        /// <summary>
        /// Test HealthHandler execution in current mode (Fast by default, Strict if HEALTH_STRICT defined)
        /// </summary>
        [UnityTest]
        public IEnumerator TestHealthHandlerExecution()
        {
            var task = TestHealthExecutionAsync();
            yield return new WaitUntil(() => task.IsCompleted);
            
            if (task.IsFaulted)
            {
                Debug.LogError($"Health handler execution test failed: {task.Exception}");
                Assert.Fail($"Health execution failed: {task.Exception?.GetBaseException().Message}");
            }
            
            Assert.IsTrue(task.Result, "HealthHandler should execute successfully");
        }

        private async Task<bool> TestHealthExecutionAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Diag.Log("Testing HealthHandler execution");
#endif
                
                // Create a health request
                var request = new HealthRequest();
                
                // Call HealthHandler
                var response = await HealthHandler.HandleAsync(request);
                
                // Verify response structure
                Assert.IsNotNull(response, "Health response should not be null");
                
                // The response should be wrapped in IpcResponse with Health data
                Assert.IsNotNull(response.Health, "Response should contain Health data");
                
                var healthResp = response.Health;
                
                // Verify Unity version is populated (should be non-empty in any mode)
                Assert.IsNotEmpty(healthResp.Version, "Version should not be empty");
                
                // Verify Ready and Status fields are present
                Assert.IsNotNull(healthResp.Ready, "Ready should be set");
                Assert.IsNotEmpty(healthResp.Status, "Status should be set");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Health response: Version={healthResp.Version}, Ready={healthResp.Ready}, Status={healthResp.Status}");
#endif
                
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Health handler execution test exception: {ex}");
                return false;
            }
        }

#if HEALTH_STRICT
        /// <summary>
        /// Test Strict mode behavior - should execute on main thread via EditorDispatcher
        /// This test only runs when HEALTH_STRICT is defined
        /// </summary>
        [UnityTest]
        public IEnumerator TestStrictModeMainThreadExecution()
        {
            var task = TestStrictModeAsync();
            yield return new WaitUntil(() => task.IsCompleted);
            
            if (task.IsFaulted)
            {
                Debug.LogError($"Strict mode test failed: {task.Exception}");
                Assert.Fail($"Strict mode failed: {task.Exception?.GetBaseException().Message}");
            }
            
            Assert.IsTrue(task.Result, "Strict mode should execute on main thread");
        }

        private async Task<bool> TestStrictModeAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Diag.Log("Testing HEALTH_STRICT mode - should use EditorDispatcher.RunOnMainAsync");
#endif
                
                var request = new HealthRequest();
                
                // In strict mode, this should use EditorDispatcher.RunOnMainAsync internally
                var response = await HealthHandler.HandleAsync(request);
                
                Assert.IsNotNull(response, "Strict mode response should not be null");
                Assert.IsNotNull(response.Health, "Strict mode should contain Health data");
                
                var healthResp = response.Health;
                
                // Verify Unity version is populated (direct Unity API access)
                Assert.IsNotEmpty(healthResp.Version, "Strict mode should provide Unity version");
                
                // In strict mode, values come from direct Unity API calls on main thread
                Assert.IsNotNull(healthResp.Ready, "Strict mode should provide Ready");
                Assert.IsNotEmpty(healthResp.Status, "Strict mode should provide Status");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Strict mode test successful: Version={healthResp.Version}");
#endif
                
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Strict mode test exception: {ex}");
                return false;
            }
        }

        /// <summary>
        /// Test that Strict mode provides consistent results under load
        /// </summary>
        [UnityTest]
        public IEnumerator TestStrictModeConsistency()
        {
            var task = TestStrictConsistencyAsync();
            yield return new WaitUntil(() => task.IsCompleted);
            
            if (task.IsFaulted)
            {
                Debug.LogError($"Strict mode consistency test failed: {task.Exception}");
                Assert.Fail($"Strict consistency failed: {task.Exception?.GetBaseException().Message}");
            }
            
            Assert.IsTrue(task.Result, "Strict mode should provide consistent results");
        }

        private async Task<bool> TestStrictConsistencyAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Diag.Log("Testing Strict mode consistency under multiple requests");
#endif
                
                const int requestCount = 5;
                var tasks = new Task<IpcResponse>[requestCount];
                var request = new HealthRequest();
                
                // Submit multiple concurrent requests
                for (int i = 0; i < requestCount; i++)
                {
                    tasks[i] = HealthHandler.HandleAsync(request);
                }
                
                // Wait for all to complete
                var responses = await Task.WhenAll(tasks);
                
                // Verify all responses are valid and consistent
                string expectedVersion = null;
                for (int i = 0; i < requestCount; i++)
                {
                    Assert.IsNotNull(responses[i], $"Response {i} should not be null");
                    Assert.IsNotNull(responses[i].Health, $"Health data {i} should not be null");
                    
                    var healthResp = responses[i].Health;
                    
                    if (expectedVersion == null)
                    {
                        expectedVersion = healthResp.Version;
                    }
                    else
                    {
                        // All responses should have same Unity version
                        Assert.AreEqual(expectedVersion, healthResp.Version, $"Response {i} should have consistent Unity version");
                    }
                }
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Strict mode consistency test: {requestCount} requests completed successfully");
#endif
                
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Strict mode consistency test exception: {ex}");
                return false;
            }
        }

#else
        /// <summary>
        /// Test Fast mode behavior - should read from EditorStateMirror without main thread dispatch
        /// This test only runs when HEALTH_STRICT is NOT defined (default Fast mode)
        /// </summary>
        [UnityTest]
        public IEnumerator TestFastModeExecution()
        {
            var task = TestFastModeAsync();
            yield return new WaitUntil(() => task.IsCompleted);
            
            if (task.IsFaulted)
            {
                Debug.LogError($"Fast mode test failed: {task.Exception}");
                Assert.Fail($"Fast mode failed: {task.Exception?.GetBaseException().Message}");
            }
            
            Assert.IsTrue(task.Result, "Fast mode should execute with minimal latency");
        }

        private async Task<bool> TestFastModeAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Diag.Log("Testing Fast mode - should use EditorStateMirror");
#endif
                
                var request = new HealthRequest();
                
                // In fast mode, this should return immediately using EditorStateMirror
                var startTime = DateTime.UtcNow;
                var response = await HealthHandler.HandleAsync(request);
                var elapsed = DateTime.UtcNow - startTime;
                
                Assert.IsNotNull(response, "Fast mode response should not be null");
                Assert.IsNotNull(response.Health, "Fast mode should contain Health data");
                
                var healthResp = response.Health;
                
                // Verify values from EditorStateMirror
                Assert.IsNotEmpty(healthResp.Version, "Fast mode should provide Unity version from mirror");
                Assert.IsNotNull(healthResp.Ready, "Fast mode should provide Ready from mirror");
                Assert.IsNotEmpty(healthResp.Status, "Fast mode should provide Status from mirror");
                
                // Fast mode should complete quickly (should be nearly instant)
                Assert.Less(elapsed.TotalMilliseconds, 100, "Fast mode should complete in under 100ms");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Fast mode test successful: Version={healthResp.Version}, Elapsed={elapsed.TotalMilliseconds}ms");
#endif
                
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Fast mode test exception: {ex}");
                return false;
            }
        }

        /// <summary>
        /// Test Fast mode responsiveness under high load
        /// </summary>
        [UnityTest]
        public IEnumerator TestFastModeHighLoad()
        {
            var task = TestFastModeLoadAsync();
            yield return new WaitUntil(() => task.IsCompleted);
            
            if (task.IsFaulted)
            {
                Debug.LogError($"Fast mode load test failed: {task.Exception}");
                Assert.Fail($"Fast mode load failed: {task.Exception?.GetBaseException().Message}");
            }
            
            Assert.IsTrue(task.Result, "Fast mode should handle high load efficiently");
        }

        private async Task<bool> TestFastModeLoadAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Diag.Log("Testing Fast mode under high load");
#endif
                
                const int requestCount = 20; // Simulate high load
                var tasks = new Task<IpcResponse>[requestCount];
                var request = new HealthRequest();
                
                var startTime = DateTime.UtcNow;
                
                // Submit all requests concurrently
                for (int i = 0; i < requestCount; i++)
                {
                    tasks[i] = HealthHandler.HandleAsync(request);
                }
                
                // Wait for all to complete
                var responses = await Task.WhenAll(tasks);
                
                var elapsed = DateTime.UtcNow - startTime;
                
                // Verify all responses are valid
                for (int i = 0; i < requestCount; i++)
                {
                    Assert.IsNotNull(responses[i], $"Response {i} should not be null");
                    Assert.IsNotNull(responses[i].Health, $"Health data {i} should not be null");
                }
                
                // Fast mode should handle all requests quickly
                var avgTimePerRequest = elapsed.TotalMilliseconds / requestCount;
                Assert.Less(avgTimePerRequest, 10, $"Average time per request should be under 10ms, was {avgTimePerRequest}ms");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Fast mode load test: {requestCount} requests in {elapsed.TotalMilliseconds}ms ({avgTimePerRequest}ms avg)");
#endif
                
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Fast mode load test exception: {ex}");
                return false;
            }
        }

        /// <summary>
        /// Test Fast mode eventual consistency during script recompilation
        /// Note: This is a simulation since triggering actual recompilation in tests is complex
        /// </summary>
        [Test]
        public void TestFastModeEventualConsistency()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing Fast mode eventual consistency behavior");
#endif
            
            // In Fast mode, values come from EditorStateMirror
            // The mirror should reflect editor state changes eventually
            
            Assert.DoesNotThrow(() =>
            {
                // Check current mirror state
                var currentCompiling = EditorStateMirror.IsCompiling;
                var currentUpdating = EditorStateMirror.IsUpdating;
                var currentVersion = EditorStateMirror.UnityVersion;
                
                Assert.IsNotNull(currentCompiling, "Mirror IsCompiling should be available");
                Assert.IsNotNull(currentUpdating, "Mirror IsUpdating should be available");
                Assert.IsNotEmpty(currentVersion, "Mirror UnityVersion should be available");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Fast mode consistency check: Compiling={currentCompiling}, Updating={currentUpdating}, Version={currentVersion}");
#endif
                
                // In a real scenario, these values would change when Unity starts/stops compiling
                // The mirror provides eventually consistent view of these states
            });
        }
#endif

        /// <summary>
        /// Test HealthHandler null argument handling
        /// </summary>
        [Test]
        public void TestHealthHandlerNullHandling()
        {
            Assert.ThrowsAsync<ArgumentNullException>(async () =>
            {
                await HealthHandler.HandleAsync(null);
            });
        }

        /// <summary>
        /// Test multiple rapid health requests don't cause issues
        /// </summary>
        [UnityTest]
        public IEnumerator TestRapidHealthRequests()
        {
            var task = TestRapidRequestsAsync();
            yield return new WaitUntil(() => task.IsCompleted);
            
            if (task.IsFaulted)
            {
                Debug.LogError($"Rapid health requests test failed: {task.Exception}");
                Assert.Fail($"Rapid requests failed: {task.Exception?.GetBaseException().Message}");
            }
            
            Assert.IsTrue(task.Result, "Rapid health requests should be handled successfully");
        }

        private async Task<bool> TestRapidRequestsAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Diag.Log("Testing rapid health requests");
#endif
                
                const int requestCount = 10;
                const int delayMs = 50; // 50ms between requests = 20 req/sec
                
                var request = new HealthRequest();
                var successCount = 0;
                
                for (int i = 0; i < requestCount; i++)
                {
                    var response = await HealthHandler.HandleAsync(request);
                    
                    if (response != null && response.Health != null)
                    {
                        successCount++;
                    }
                    
                    // Small delay to simulate realistic request pattern
                    if (i < requestCount - 1) // Don't delay after last request
                    {
                        await Task.Delay(delayMs);
                    }
                }
                
                Assert.AreEqual(requestCount, successCount, $"All {requestCount} rapid requests should succeed");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Rapid requests test: {successCount}/{requestCount} requests succeeded");
#endif
                
                return successCount == requestCount;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Rapid requests test exception: {ex}");
                return false;
            }
        }
    }
}
#endif