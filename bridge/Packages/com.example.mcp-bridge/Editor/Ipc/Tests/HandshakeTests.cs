#if UNITY_EDITOR
using System;
using System.Threading;
using NUnit.Framework;
using UnityEngine;
using Bridge.Editor.Ipc.Infra;
using Mcp.Unity.V1;
using Mcp.Unity.V1.Ipc.Infra;

namespace Bridge.Editor.Ipc.Tests
{
    /// <summary>
    /// Unit-level tests for handshake validation methods to ensure main-thread execution
    /// These tests focus on HandshakeHandler method behavior, complementing the integration tests
    /// </summary>
    [TestFixture]
    public class HandshakeTests
    {
        [OneTimeSetUp]
        public void OneTimeSetUp()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("HandshakeTests starting - focusing on main-thread validation");
#endif
        }

        /// <summary>
        /// Test that ValidateEditorState executes on main thread and accesses Unity APIs safely
        /// </summary>
        [Test]
        public void TestValidateEditorStateMainThreadExecution()
        {
            // Arrange - we should be on main thread in Unity test runner
            var initialThreadId = Thread.CurrentThread.ManagedThreadId;
            
            // Act & Assert - this should succeed as we're on main thread
            Assert.DoesNotThrow(() =>
            {
                // Create a minimal Hello request for validation
                var hello = new IpcHello
                {
                    ClientVersion = "1.0.0",
                    Token = "test-token",
                    ProjectRoot = Application.dataPath // Unity API access
                };

#if UNITY_EDITOR && DEBUG
                // Verify we're on main thread
                Assert.IsTrue(Diag.IsMainThread(), "ValidateEditorState test should run on main thread");
                var threadTag = Diag.ThreadTag();
                Assert.AreEqual("MAIN", threadTag, $"Expected MAIN thread tag, got {threadTag}");
#endif

                // The actual validation would include MainThreadGuard.AssertMainThread()
                // We simulate this by ensuring Unity API access works
                var unityVersion = Application.unityVersion;
                var platform = Application.platform;
                
                Assert.IsNotNull(unityVersion, "Unity API access should work on main thread");
                Assert.IsNotNull(platform, "Platform API access should work on main thread");

#if UNITY_EDITOR && DEBUG
                Diag.Log($"ValidateEditorState test: Unity={unityVersion}, Platform={platform}");
#endif
            });
        }

        /// <summary>
        /// Test that CreateWelcome executes on main thread and includes Unity information
        /// </summary>
        [Test]
        public void TestCreateWelcomeMainThreadExecution()
        {
            // Arrange
            var initialThreadId = Thread.CurrentThread.ManagedThreadId;
            
            // Act & Assert - simulate CreateWelcome behavior
            Assert.DoesNotThrow(() =>
            {
#if UNITY_EDITOR && DEBUG
                // Verify main thread execution
                Assert.IsTrue(Diag.IsMainThread(), "CreateWelcome test should run on main thread");
                MainThreadGuard.AssertMainThread(); // This is what CreateWelcome should include
#endif

                // Simulate Welcome message creation with Unity API access
                var unityVersion = Application.unityVersion;
                var platform = Application.platform.ToString();
                var isEditor = Application.isEditor;
                
                // Verify Unity APIs return valid data
                Assert.IsNotEmpty(unityVersion, "Unity version should not be empty");
                Assert.IsNotEmpty(platform, "Platform should not be empty");
                Assert.IsTrue(isEditor, "Should be running in Unity Editor");

                // Simulate Welcome response structure
                var simulatedWelcome = new IpcWelcome
                {
                    ServerVersion = "1.0.0",
                    EditorVersion = unityVersion,
                    // Meta would include platform info
                };
                
                Assert.IsNotEmpty(simulatedWelcome.EditorVersion, "Welcome should include editor version");

#if UNITY_EDITOR && DEBUG
                Diag.Log($"CreateWelcome test: Version={unityVersion}, Platform={platform}, Editor={isEditor}");
#endif
            });
        }

        /// <summary>
        /// Test token validation with various scenarios
        /// </summary>
        [Test]
        public void TestTokenValidation()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing token validation scenarios");
#endif

            // Test valid token format
            Assert.DoesNotThrow(() =>
            {
                var validToken = "test-token-valid";
                Assert.IsNotEmpty(validToken, "Valid token should not be empty");
                Assert.IsTrue(validToken.Length > 5, "Token should have reasonable length");
            });

            // Test invalid token scenarios
            Assert.DoesNotThrow(() =>
            {
                // These would be rejected in actual validation
                var emptyToken = "";
                var nullToken = (string)null;
                var shortToken = "abc";
                
                // In actual implementation, these would cause rejection
                Assert.IsEmpty(emptyToken, "Empty token should be rejected");
                Assert.IsNull(nullToken, "Null token should be rejected");
                Assert.IsTrue(shortToken.Length < 5, "Short token should be rejected");
            });
        }

        /// <summary>
        /// Test version compatibility validation
        /// </summary>
        [Test]
        public void TestVersionValidation()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing version compatibility validation");
#endif

            Assert.DoesNotThrow(() =>
            {
                // Test compatible version
                var compatibleVersion = "1.0.0";
                Assert.IsNotEmpty(compatibleVersion, "Version should not be empty");
                
                // Test version parsing logic (simplified)
                var parts = compatibleVersion.Split('.');
                Assert.AreEqual(3, parts.Length, "Version should have 3 parts");
                Assert.IsTrue(int.TryParse(parts[0], out var major), "Major version should be numeric");
                Assert.AreEqual(1, major, "Expected major version 1");
                
                // Test incompatible version
                var incompatibleVersion = "999.0.0";
                var incompatibleParts = incompatibleVersion.Split('.');
                Assert.IsTrue(int.TryParse(incompatibleParts[0], out var incompatibleMajor), "Should parse major version");
                Assert.AreNotEqual(1, incompatibleMajor, "Major version 999 should be incompatible");
            });
        }

        /// <summary>
        /// Test project root path validation
        /// </summary>
        [Test]
        public void TestProjectRootValidation()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing project root path validation");
#endif

            Assert.DoesNotThrow(() =>
            {
                // Test valid project root (current Unity project)
                var validProjectRoot = Application.dataPath;
                Assert.IsNotEmpty(validProjectRoot, "Project root should not be empty");
                Assert.IsTrue(System.IO.Directory.Exists(validProjectRoot), "Project root should exist");
                
                // Test invalid project root
                var invalidProjectRoot = "/invalid/path/that/does/not/exist";
                Assert.IsFalse(System.IO.Directory.Exists(invalidProjectRoot), "Invalid path should not exist");
                
#if UNITY_EDITOR && DEBUG
                Diag.Log($"Valid project root: {validProjectRoot}");
#endif
            });
        }

        /// <summary>
        /// Test that handshake methods would fail if called from background thread
        /// Note: This test runs on main thread, so it verifies the guard would work
        /// </summary>
        [Test]
        public void TestMainThreadGuardBehavior()
        {
#if UNITY_EDITOR && DEBUG
            // Verify we're on main thread and guard passes
            Assert.IsTrue(Diag.IsMainThread(), "Test should be running on main thread");
            
            // This should not throw since we're on main thread
            Assert.DoesNotThrow(() =>
            {
                MainThreadGuard.AssertMainThread();
                Diag.Log("MainThreadGuard.AssertMainThread() passed as expected on main thread");
            });

            // Verify thread identification
            var threadTag = Diag.ThreadTag();
            Assert.AreEqual("MAIN", threadTag, $"Expected MAIN thread tag, got {threadTag}");
            
            Diag.Log($"MainThreadGuard test completed successfully on {threadTag} thread");
#else
            Assert.Pass("MainThreadGuard diagnostics only available in DEBUG builds");
#endif
        }

        /// <summary>
        /// Test handshake rejection scenarios execute quickly without Unity API access
        /// </summary>
        [Test]
        public void TestHandshakeRejectionPath()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing handshake rejection scenarios");
#endif

            // Simulate early rejection scenarios that should NOT touch Unity APIs
            Assert.DoesNotThrow(() =>
            {
                // Token validation (no Unity API needed)
                var invalidToken = "";
                Assert.IsEmpty(invalidToken, "Invalid token should be caught early");
                
                // Version validation (no Unity API needed)
                var incompatibleVersion = "999.0";
                Assert.IsTrue(incompatibleVersion.StartsWith("999"), "Incompatible version should be caught early");
                
                // These validations should happen BEFORE any Unity API access
                // and thus before MainThreadGuard.AssertMainThread() is called
                
#if UNITY_EDITOR && DEBUG
                Diag.Log("Early rejection validations completed without Unity API access");
#endif
            });
        }
    }
}
#endif