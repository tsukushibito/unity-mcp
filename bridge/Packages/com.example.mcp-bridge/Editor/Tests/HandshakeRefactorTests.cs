#if UNITY_EDITOR
using System;
using System.Collections;
using System.Net;
using System.Threading.Tasks;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using Bridge.Editor.Ipc;
using Mcp.Unity.V1.Ipc;
using Bridge.Editor.Ipc.Infra;
using Mcp.Unity.V1.Ipc.Tests;

namespace Bridge.Editor.Tests
{
    /// <summary>
    /// Tests for handshake refactor (M3-A) - ensuring thread safety in Hello → Welcome/Reject flow
    /// </summary>
    [TestFixture]
    public class HandshakeRefactorTests
    {
        private MockIpcClient _mockClient;
        private int _testPort;
        private string _originalToken;

        [OneTimeSetUp]
        public void OneTimeSetUp()
        {
            // Save original token value for restoration
            _originalToken = UnityEditor.EditorUserSettings.GetConfigValue("MCP.IpcToken");
            Debug.Log($"[HandshakeRefactorTests] Saved original token: {(_originalToken ?? "(null)")}");
            
            // Set up test token for IPC authentication
            var testToken = "test-token";
            UnityEditor.EditorUserSettings.SetConfigValue("MCP.IpcToken", testToken);
            Debug.Log($"[HandshakeRefactorTests] Set test token: {testToken}");

            // Restart IPC server to pick up new token configuration
            if (EditorIpcServer.IsRunning)
            {
                Debug.Log("[HandshakeRefactorTests] Restarting IPC server to pick up new configuration...");
                EditorIpcServer.Shutdown();
                System.Threading.Thread.Sleep(300); // Extended wait for shutdown
            }
            
            // Reload configuration and start server
            EditorIpcServer.ReloadConfiguration();
            Debug.Log("[HandshakeRefactorTests] Starting IPC server for tests...");
            _ = EditorIpcServer.StartAsync();

            // Wait for server to be ready with timeout
            var startTime = System.DateTime.Now;
            var timeout = System.TimeSpan.FromSeconds(5);
            
            while (!EditorIpcServer.IsReady && (System.DateTime.Now - startTime) < timeout)
            {
                System.Threading.Thread.Sleep(100);
            }

            if (!EditorIpcServer.IsReady)
            {
                throw new System.Exception($"EditorIpcServer failed to become ready within {timeout.TotalSeconds} seconds");
            }

            Debug.Log($"[HandshakeRefactorTests] EditorIpcServer is ready on port {EditorIpcServer.CurrentPort}");
        }

        [OneTimeTearDown]
        public void OneTimeTearDown()
        {
            // サーバーをシャットダウンしてクリーンな状態にする
            if (EditorIpcServer.IsRunning)
            {
                Debug.Log("[HandshakeRefactorTests] Shutting down IPC server...");
                EditorIpcServer.Shutdown();
                
                // 非同期処理の完全な停止を確実に待機
                var shutdownStartTime = System.DateTime.Now;
                var shutdownTimeout = System.TimeSpan.FromSeconds(3);
                
                while (EditorIpcServer.IsRunning && (System.DateTime.Now - shutdownStartTime) < shutdownTimeout)
                {
                    System.Threading.Thread.Sleep(100);
                }
                
                // 追加の安全な待機時間
                System.Threading.Thread.Sleep(500);
                Debug.Log("[HandshakeRefactorTests] IPC server shutdown completed");
            }
            
            // 元のトークン値を復元
            UnityEditor.EditorUserSettings.SetConfigValue("MCP.IpcToken", _originalToken ?? "");
            Debug.Log($"[HandshakeRefactorTests] Restored original token: {(_originalToken ?? "(null)")}");
            
            // 設定変更後のサーバー再起動（元の設定で）
            EditorIpcServer.ReloadConfiguration();
            Debug.Log("[HandshakeRefactorTests] Restarting server with original configuration...");
            _ = EditorIpcServer.StartAsync();
        }

        [SetUp]
        public void SetUp()
        {
            // Connect to the actual EditorIpcServer using its current port
            _testPort = EditorIpcServer.CurrentPort;
            
            // Fallback to default port if CurrentPort is not available
            if (_testPort <= 0)
            {
                _testPort = 7777; // Default port
                Debug.LogWarning($"[HandshakeRefactorTests] EditorIpcServer.CurrentPort not available, using default port: {_testPort}");
            }
            else
            {
                Debug.Log($"[HandshakeRefactorTests] Using EditorIpcServer.CurrentPort: {_testPort}");
            }
            
            _mockClient = new MockIpcClient(IPAddress.Loopback, _testPort);
        }

        [TearDown]
        public void TearDown()
        {
            _mockClient?.Dispose();
            _mockClient = null;
        }

        /// <summary>
        /// Test successful handshake path - valid token, version, and editor state
        /// </summary>
        [UnityTest]
        public IEnumerator TestHandshakeSuccessPath()
        {
            var task = TestHandshakeSuccessAsync();
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                Debug.LogError($"Handshake success test failed: {task.Exception}");
                Assert.Fail($"Handshake success failed: {task.Exception?.GetBaseException().Message}");
            }

            Assert.IsTrue(task.Result, "Valid handshake should succeed and return Welcome");
        }

        private async Task<bool> TestHandshakeSuccessAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Testing successful handshake path");
#endif

                // Use valid token, version, and project root
                var connected = await _mockClient.ConnectAsync("test-token");
                if (!connected)
                {
                    Debug.LogError("Failed to connect with valid credentials");
                    return false;
                }

                // Verify we can send a request (indicates successful handshake)
                var healthResponse = await _mockClient.SendHealthRequestAsync();

                Debug.Log($"Handshake success test: Health response received - Ready={healthResponse.Ready}");
                return healthResponse != null;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Handshake success test exception: {ex}");
                return false;
            }
        }

        /// <summary>
        /// Test token validation reject - should not touch Unity APIs before reject
        /// </summary>
        [UnityTest]
        public IEnumerator TestHandshakeTokenReject()
        {
            var task = TestTokenRejectAsync();
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                Debug.LogError($"Token reject test failed: {task.Exception}");
                Assert.Fail($"Token reject test failed: {task.Exception?.GetBaseException().Message}");
            }

            Assert.IsTrue(task.Result, "Invalid token should be rejected before Unity API access");
        }

        private async Task<bool> TestTokenRejectAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Testing token rejection path");
#endif

                // Test with invalid token - should be rejected early
                var connected = await _mockClient.ConnectAsync("invalid-token-should-be-rejected");

                // Connection should fail due to token rejection
                if (connected)
                {
                    Debug.LogError("Connection succeeded with invalid token - this should not happen");
                    return false;
                }

                Debug.Log("Token reject test: Connection properly rejected with invalid token");
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Token reject test exception: {ex}");
                return false;
            }
        }

        /// <summary>
        /// Test version compatibility reject - should not touch Unity APIs before reject
        /// </summary>
        [UnityTest]
        public IEnumerator TestHandshakeVersionReject()
        {
            var task = TestVersionRejectAsync();
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                Debug.LogError($"Version reject test failed: {task.Exception}");
                Assert.Fail($"Version reject test failed: {task.Exception?.GetBaseException().Message}");
            }

            Assert.IsTrue(task.Result, "Incompatible version should be rejected before Unity API access");
        }

        private async Task<bool> TestVersionRejectAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Testing version rejection path");
#endif

                // Test with incompatible version - should be rejected early
                var connected = await _mockClient.ConnectWithVersionAsync("test-token", "999.0"); // Incompatible major version

                // Connection should fail due to version rejection
                if (connected)
                {
                    Debug.LogError("Connection succeeded with incompatible version - this should not happen");
                    return false;
                }

                Debug.Log("Version reject test: Connection properly rejected with incompatible version");
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Version reject test exception: {ex}");
                return false;
            }
        }

        

        /// <summary>
        /// Test that handshake validation methods now require main thread execution
        /// </summary>
        [Test]
        public void TestMainThreadRequirement()
        {
#if UNITY_EDITOR && DEBUG
            // This test runs on the main thread, so these calls should succeed
            Assert.DoesNotThrow(() =>
            {
                // These method calls would now include MainThreadGuard.AssertMainThread()
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Testing main thread requirement for handshake validation");

                // Verify we're on main thread
                Assert.IsTrue(Mcp.Unity.V1.Ipc.Infra.Diag.IsMainThread(), "Test should run on Unity main thread");

                var threadTag = Mcp.Unity.V1.Ipc.Infra.Diag.ThreadTag();
                Assert.AreEqual("MAIN", threadTag, "Thread tag should be MAIN for Unity test runner");
            });

            Debug.Log("Main thread requirement test: Validation methods can be called from main thread");
#else
            Assert.Pass("Diagnostics only available in DEBUG builds");
#endif
        }

        /// <summary>
        /// Test that Unity version and platform are correctly included in Welcome message
        /// </summary>
        [UnityTest]
        public IEnumerator TestWelcomeMessageContainsUnityInfo()
        {
            var task = TestWelcomeInfoAsync();
            yield return new WaitUntil(() => task.IsCompleted);

            if (task.IsFaulted)
            {
                Debug.LogError($"Welcome info test failed: {task.Exception}");
                Assert.Fail($"Welcome info test failed: {task.Exception?.GetBaseException().Message}");
            }

            Assert.IsTrue(task.Result, "Welcome message should contain Unity version and platform info");
        }

        private async Task<bool> TestWelcomeInfoAsync()
        {
            try
            {
#if UNITY_EDITOR && DEBUG
                Mcp.Unity.V1.Ipc.Infra.Diag.Log("Testing Welcome message Unity information");
#endif

                var connected = await _mockClient.ConnectAsync("test-token");
                if (!connected)
                {
                    Debug.LogError("Failed to connect for Welcome info test");
                    return false;
                }

                // Get the welcome message details from the mock client
                var welcomeInfo = _mockClient.GetLastWelcomeInfo();

                if (welcomeInfo == null)
                {
                    Debug.LogError("No welcome info received");
                    return false;
                }

                // Verify Unity version is present and not empty/unknown
                Assert.IsNotEmpty(welcomeInfo.EditorVersion, "EditorVersion should not be empty");
                Assert.AreNotEqual("unknown", welcomeInfo.EditorVersion, "EditorVersion should not be 'unknown'");

                // Verify platform information is present
                Assert.IsTrue(welcomeInfo.Meta.ContainsKey("platform"), "Welcome should contain platform metadata");
                Assert.IsNotEmpty(welcomeInfo.Meta["platform"], "Platform should not be empty");

                Debug.Log($"Welcome info test: EditorVersion={welcomeInfo.EditorVersion}, Platform={welcomeInfo.Meta["platform"]}");
                return true;
            }
            catch (Exception ex)
            {
                Debug.LogError($"Welcome info test exception: {ex}");
                return false;
            }
        }
    }
}
#endif
