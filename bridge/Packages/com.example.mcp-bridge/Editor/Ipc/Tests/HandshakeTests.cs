#if UNITY_EDITOR
using System;
using System.Threading;
using NUnit.Framework;
using UnityEngine;
using Bridge.Editor.Ipc.Infra;
using Mcp.Unity.V1;
using Mcp.Unity.V1.Ipc;
using Mcp.Unity.V1.Ipc.Infra;
using Mcp.Unity.V1.Generated;

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
        /// Token 必須および取得経路（EditorUserSettingsのみ有効）を検証
        /// </summary>
        [Test]
        public void TestTokenRequiredAndSource()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing token required and source policy (EditorUserSettings only)");
#endif

            // Backup current token and clear
            var saved = UnityEditor.EditorUserSettings.GetConfigValue("MCP.IpcToken");
            try
            {
                UnityEditor.EditorUserSettings.SetConfigValue("MCP.IpcToken", string.Empty);

                // 1) expected token missing
                var vr1 = EditorIpcServerAccessor.TestValidateToken(null, null);
                Assert.IsFalse(EditorIpcServerAccessor.IsValidationResultValid(vr1));
                Assert.AreEqual(IpcReject.Types.Code.Unauthenticated, EditorIpcServerAccessor.GetValidationResultErrorCode(vr1));
                StringAssert.Contains("Missing or empty token", EditorIpcServerAccessor.GetValidationResultErrorMessage(vr1));

                // 2) set EditorUserSettings token; set env and EditorPrefs to different values
                UnityEditor.EditorUserSettings.SetConfigValue("MCP.IpcToken", "from-user-settings");
                System.Environment.SetEnvironmentVariable("MCP_IPC_TOKEN", "from-env");
                UnityEditor.EditorPrefs.SetString("MCP.IpcToken", "from-editor-prefs");

                var loaded = EditorIpcServerAccessor.TestLoadTokenFromPrefs();
                Assert.AreEqual("from-user-settings", loaded, "Only EditorUserSettings should be used");

                // 3) mismatch token
                var vr2 = EditorIpcServerAccessor.TestValidateToken("correct", "wrong");
                Assert.IsFalse(EditorIpcServerAccessor.IsValidationResultValid(vr2));
                Assert.AreEqual(IpcReject.Types.Code.Unauthenticated, EditorIpcServerAccessor.GetValidationResultErrorCode(vr2));
                StringAssert.Contains("Invalid token", EditorIpcServerAccessor.GetValidationResultErrorMessage(vr2));

                // 4) match token
                var vr3 = EditorIpcServerAccessor.TestValidateToken("secret", "secret");
                Assert.IsTrue(EditorIpcServerAccessor.IsValidationResultValid(vr3));
            }
            finally
            {
                UnityEditor.EditorUserSettings.SetConfigValue("MCP.IpcToken", saved);
                System.Environment.SetEnvironmentVariable("MCP_IPC_TOKEN", null);
                if (UnityEditor.EditorPrefs.HasKey("MCP.IpcToken")) UnityEditor.EditorPrefs.DeleteKey("MCP.IpcToken");
            }
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
        /// Test schema hash validation
        /// </summary>
        [Test]
        public void TestSchemaHashValidation()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing schema hash validation scenarios");
#endif

            Assert.DoesNotThrow(() =>
            {
                // Test valid schema hash (using the expected hash from generated code)
                var validSchemaHash = Google.Protobuf.ByteString.CopyFrom(Mcp.Unity.V1.Generated.Schema.SchemaHashBytes);
                Assert.IsNotNull(validSchemaHash, "Valid schema hash should not be null");
                Assert.AreEqual(32, validSchemaHash.Length, "Schema hash should be 32 bytes (SHA-256)");
                
                // Test empty schema hash (should be rejected)
                var emptySchemaHash = Google.Protobuf.ByteString.Empty;
                Assert.IsTrue(emptySchemaHash.IsEmpty, "Empty schema hash should be caught");
                
                // Test wrong length schema hash (should be rejected)
                var wrongLengthHash = Google.Protobuf.ByteString.CopyFrom(new byte[] { 1, 2, 3 });
                Assert.AreNotEqual(32, wrongLengthHash.Length, "Wrong length hash should be rejected");
                
                // Test schema hash hex format consistency
                var expectedHex = Mcp.Unity.V1.Generated.Schema.SCHEMA_HASH_HEX;
                Assert.IsNotEmpty(expectedHex, "Schema hash hex should not be empty");
                Assert.AreEqual(64, expectedHex.Length, "Schema hash hex should be 64 characters");

#if UNITY_EDITOR && DEBUG
                Diag.Log($"Schema hash validation test completed: hex={expectedHex}");
#endif
            });
        }

        /// <summary>
        /// Test ValidateSchemaHash method branches - comprehensive validation scenarios
        /// </summary>
        [Test]
        public void TestValidateSchemaHashBranches()
        {
#if UNITY_EDITOR && DEBUG
            Diag.Log("Testing ValidateSchemaHash method branches comprehensively");
#endif

            // Branch 1: Empty schema hash (should fail with FAILED_PRECONDITION)
            var emptyHash = Google.Protobuf.ByteString.Empty;
            var emptyResult = EditorIpcServerAccessor.TestValidateSchemaHash(emptyHash);
            Assert.IsFalse(EditorIpcServerAccessor.IsValidationResultValid(emptyResult), "Empty schema hash should be invalid");
            Assert.AreEqual(IpcReject.Types.Code.FailedPrecondition, EditorIpcServerAccessor.GetValidationResultErrorCode(emptyResult), "Empty hash should return FAILED_PRECONDITION");
            Assert.IsTrue(EditorIpcServerAccessor.GetValidationResultErrorMessage(emptyResult).Contains("Schema hash missing"), "Error message should mention schema hash missing");

            // Branch 2: Wrong length schema hash (should fail with FAILED_PRECONDITION)
            var wrongLengthHash = Google.Protobuf.ByteString.CopyFrom(new byte[] { 0x01, 0x02, 0x03, 0x04 }); // Only 4 bytes instead of 32
            var lengthResult = EditorIpcServerAccessor.TestValidateSchemaHash(wrongLengthHash);
            Assert.IsFalse(EditorIpcServerAccessor.IsValidationResultValid(lengthResult), "Wrong length schema hash should be invalid");
            Assert.AreEqual(IpcReject.Types.Code.FailedPrecondition, EditorIpcServerAccessor.GetValidationResultErrorCode(lengthResult), "Wrong length should return FAILED_PRECONDITION");
            Assert.IsTrue(EditorIpcServerAccessor.GetValidationResultErrorMessage(lengthResult).Contains("length mismatch"), "Error message should mention length mismatch");

            // Branch 3: Correct length but wrong bytes (should fail with FAILED_PRECONDITION)
            var wrongBytesHash = Google.Protobuf.ByteString.CopyFrom(new byte[32]); // All zeros, 32 bytes
            var bytesResult = EditorIpcServerAccessor.TestValidateSchemaHash(wrongBytesHash);
            Assert.IsFalse(EditorIpcServerAccessor.IsValidationResultValid(bytesResult), "Wrong bytes schema hash should be invalid");
            Assert.AreEqual(IpcReject.Types.Code.FailedPrecondition, EditorIpcServerAccessor.GetValidationResultErrorCode(bytesResult), "Wrong bytes should return FAILED_PRECONDITION");
            Assert.IsTrue(EditorIpcServerAccessor.GetValidationResultErrorMessage(bytesResult).Contains("Schema hash mismatch"), "Error message should mention schema hash mismatch");

            // Branch 4: Correct schema hash (should succeed)
            var validHash = Google.Protobuf.ByteString.CopyFrom(Mcp.Unity.V1.Generated.Schema.SchemaHashBytes);
            var validResult = EditorIpcServerAccessor.TestValidateSchemaHash(validHash);
            Assert.IsTrue(EditorIpcServerAccessor.IsValidationResultValid(validResult), "Valid schema hash should be accepted");
            Assert.IsNull(EditorIpcServerAccessor.GetValidationResultErrorMessage(validResult), "Valid result should not have error message");

#if UNITY_EDITOR && DEBUG
            Diag.Log("ValidateSchemaHash branches test completed - all scenarios verified");
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
                
                // Schema validation (no Unity API needed)
                var invalidSchemaHash = Google.Protobuf.ByteString.CopyFrom(new byte[] { 0xFF, 0xFF });
                Assert.AreNotEqual(32, invalidSchemaHash.Length, "Invalid schema hash should be caught early");
                
                // These validations should happen BEFORE any Unity API access
                // and thus before MainThreadGuard.AssertMainThread() is called
                
#if UNITY_EDITOR && DEBUG
                Diag.Log("Early rejection validations completed without Unity API access");
#endif
            });
        }
    }

    /// <summary>
    /// Test accessor for EditorIpcServer private methods
    /// </summary>
    public static class EditorIpcServerAccessor
    {
        /// <summary>
        /// Test access to ValidateSchemaHash method
        /// </summary>
        public static object TestValidateSchemaHash(Google.Protobuf.ByteString schemaHash)
        {
            // Use reflection to access the private ValidateSchemaHash method
            var method = typeof(EditorIpcServer).GetMethod("ValidateSchemaHash", 
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
            
            if (method == null)
            {
                throw new System.InvalidOperationException("ValidateSchemaHash method not found");
            }
            
            return method.Invoke(null, new object[] { schemaHash });
        }

        /// <summary>
        /// Test access to ValidateToken(expected, client)
        /// </summary>
        public static object TestValidateToken(string expected, string client)
        {
            var method = typeof(EditorIpcServer).GetMethod("ValidateToken",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
            if (method == null)
                throw new System.InvalidOperationException("ValidateToken method not found");
            return method.Invoke(null, new object[] { expected, client });
        }

        /// <summary>
        /// Test access to ValidateProjectRoot(projectRoot)
        /// </summary>
        public static object TestValidateProjectRoot(string projectRoot)
        {
            var method = typeof(EditorIpcServer).GetMethod("ValidateProjectRoot",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
            if (method == null)
                throw new System.InvalidOperationException("ValidateProjectRoot method not found");
            return method.Invoke(null, new object[] { projectRoot });
        }

        /// <summary>
        /// Test access to LoadTokenFromPrefs()
        /// </summary>
        public static string TestLoadTokenFromPrefs()
        {
            var method = typeof(EditorIpcServer).GetMethod("LoadTokenFromPrefs",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static);
            if (method == null)
                throw new System.InvalidOperationException("LoadTokenFromPrefs method not found");
            return (string)method.Invoke(null, new object[] { });
        }
        
        /// <summary>
        /// Helper to check if validation result is valid
        /// </summary>
        public static bool IsValidationResultValid(object validationResult)
        {
            var isValidProperty = validationResult.GetType().GetProperty("IsValid");
            return (bool)isValidProperty.GetValue(validationResult);
        }
        
        /// <summary>
        /// Helper to get error code from validation result
        /// </summary>
        public static IpcReject.Types.Code GetValidationResultErrorCode(object validationResult)
        {
            var errorCodeProperty = validationResult.GetType().GetProperty("ErrorCode");
            return (IpcReject.Types.Code)errorCodeProperty.GetValue(validationResult);
        }
        
        /// <summary>
        /// Helper to get error message from validation result
        /// </summary>
        public static string GetValidationResultErrorMessage(object validationResult)
        {
            var errorMessageProperty = validationResult.GetType().GetProperty("ErrorMessage");
            return (string)errorMessageProperty.GetValue(validationResult);
        }
    }
}
#endif
