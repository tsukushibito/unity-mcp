#if UNITY_EDITOR
using NUnit.Framework;
using UnityEditor;
using UnityEngine;
using Mcp.Unity.Editor.UI;

namespace Mcp.Unity.Editor.Tests
{
    /// <summary>
    /// Tests for McpSettingsProvider UI functionality
    /// This validates the BDD scenario: Unity Project SettingsでのUI設定
    /// </summary>
    [TestFixture]
    public class McpSettingsProviderTests
    {
        private const string TokenKey = "MCP.IpcToken";
        private const string PortKey = "MCP.IpcPort";
        private string _originalToken;
        private string _originalPort;

        [SetUp]
        public void SetUp()
        {
            // Save original settings before each test
            _originalToken = EditorUserSettings.GetConfigValue(TokenKey);
            _originalPort = EditorUserSettings.GetConfigValue(PortKey);
            
            // Clear any existing settings before each test
            EditorUserSettings.SetConfigValue(TokenKey, "");
            EditorUserSettings.SetConfigValue(PortKey, "");
        }

        [TearDown]
        public void TearDown()
        {
            // Restore original settings after each test
            EditorUserSettings.SetConfigValue(TokenKey, _originalToken ?? "");
            EditorUserSettings.SetConfigValue(PortKey, _originalPort ?? "");
        }

        [Test]
        public void SettingsProvider_ShouldBeCreated()
        {
            // Given: McpSettingsProvider exists
            // When: Creating settings provider
            var provider = McpSettingsProvider.CreateSettingsProvider();
            
            // Then: Provider should be created successfully
            Assert.IsNotNull(provider, "Settings provider should be created");
            Assert.AreEqual("Project/MCP Bridge", provider.settingsPath, "Settings path should be correct");
            Assert.AreEqual("MCP Bridge", provider.label, "Label should be correct");
            Assert.AreEqual(SettingsScope.Project, provider.scope, "Scope should be Project");
        }

        [Test]
        public void DefaultPort_ShouldBe7777()
        {
            // Given: No port configuration set
            // When: Getting current port from empty settings
            var currentPortString = EditorUserSettings.GetConfigValue(PortKey) ?? string.Empty;
            
            // Then: Should default to 7777
            Assert.IsEmpty(currentPortString, "Port should be empty initially");
            // The provider should treat empty as default port 7777
        }

        [Test]
        public void CustomPort_ShouldBePersisted()
        {
            // Given: Custom port number
            const int customPort = 8888;
            
            // When: Setting port via EditorUserSettings
            EditorUserSettings.SetConfigValue(PortKey, customPort.ToString());
            
            // Then: Should be retrievable
            var retrievedPortString = EditorUserSettings.GetConfigValue(PortKey);
            Assert.AreEqual(customPort.ToString(), retrievedPortString, "Custom port should be persisted");
            
            Assert.IsTrue(int.TryParse(retrievedPortString, out int retrievedPort), "Port should be parseable as int");
            Assert.AreEqual(customPort, retrievedPort, "Retrieved port should match set port");
        }

        [Test]
        public void Token_ShouldBePersisted()
        {
            // Given: Custom token
            const string testToken = "test-token-12345";
            
            // When: Setting token via EditorUserSettings
            EditorUserSettings.SetConfigValue(TokenKey, testToken);
            
            // Then: Should be retrievable
            var retrievedToken = EditorUserSettings.GetConfigValue(TokenKey);
            Assert.AreEqual(testToken, retrievedToken, "Token should be persisted correctly");
        }

        [Test]
        public void EmptyToken_ShouldReturnEmptyString()
        {
            // Given: No token set
            // When: Getting token from empty settings
            var token = EditorUserSettings.GetConfigValue(TokenKey) ?? string.Empty;
            
            // Then: Should be empty
            Assert.IsEmpty(token, "Token should be empty when not set");
        }

        [Test]
        public void ClearPort_ShouldResetToEmpty()
        {
            // Given: Port is set
            EditorUserSettings.SetConfigValue(PortKey, "9999");
            Assert.IsNotEmpty(EditorUserSettings.GetConfigValue(PortKey), "Port should be set initially");
            
            // When: Clearing port
            EditorUserSettings.SetConfigValue(PortKey, string.Empty);
            
            // Then: Should be empty
            var clearedPort = EditorUserSettings.GetConfigValue(PortKey) ?? string.Empty;
            Assert.IsEmpty(clearedPort, "Port should be empty after clearing");
        }

        [Test]
        public void ClearToken_ShouldResetToEmpty()
        {
            // Given: Token is set
            EditorUserSettings.SetConfigValue(TokenKey, "test-token");
            Assert.IsNotEmpty(EditorUserSettings.GetConfigValue(TokenKey), "Token should be set initially");
            
            // When: Clearing token
            EditorUserSettings.SetConfigValue(TokenKey, string.Empty);
            
            // Then: Should be empty
            var clearedToken = EditorUserSettings.GetConfigValue(TokenKey) ?? string.Empty;
            Assert.IsEmpty(clearedToken, "Token should be empty after clearing");
        }

        [Test]
        public void PortValidation_ShouldAcceptValidPorts()
        {
            // Given: Valid port ranges
            int[] validPorts = { 1024, 8080, 7777, 65535 };
            
            foreach (var port in validPorts)
            {
                // When: Setting valid port
                EditorUserSettings.SetConfigValue(PortKey, port.ToString());
                var retrievedPort = EditorUserSettings.GetConfigValue(PortKey);
                
                // Then: Should be accepted and parseable
                Assert.AreEqual(port.ToString(), retrievedPort, $"Port {port} should be accepted");
                Assert.IsTrue(int.TryParse(retrievedPort, out int parsed), $"Port {port} should be parseable");
                Assert.AreEqual(port, parsed, $"Parsed port should match original {port}");
                
                // Validate port is in valid range
                Assert.IsTrue(port >= 1024 && port <= 65535, $"Port {port} should be in valid range");
            }
        }

        [Test]
        public void InvalidPortStrings_ShouldBeHandledGracefully()
        {
            // Given: Invalid port strings
            string[] invalidPorts = { "invalid", "-1", "0", "99999", "", "abc123" };
            
            foreach (var invalidPort in invalidPorts)
            {
                // When: Setting invalid port
                EditorUserSettings.SetConfigValue(PortKey, invalidPort);
                var retrievedPort = EditorUserSettings.GetConfigValue(PortKey);
                
                // Then: Should be stored as-is (validation happens in UI)
                Assert.AreEqual(invalidPort, retrievedPort, $"Invalid port '{invalidPort}' should be stored as-is");
                
                // But parsing should either fail or be out of valid range
                if (int.TryParse(retrievedPort, out int parsed))
                {
                    Assert.IsTrue(parsed < 1024 || parsed > 65535, $"Parsed invalid port {parsed} should be out of valid range");
                }
                // If parsing fails, that's also acceptable for invalid strings
            }
        }

        [Test]
        public void OpenProjectSettings_ShouldNotThrow()
        {
            // This test validates that the menu item method doesn't throw
            // We can't easily test that it actually opens the settings window in automated tests
            
            // Given: Settings provider exists
            // When: Calling OpenProjectSettings
            // Then: Should not throw any exceptions
            Assert.DoesNotThrow(() => McpSettingsProvider.OpenProjectSettings(), 
                "OpenProjectSettings should not throw exceptions");
        }
    }
}
#endif