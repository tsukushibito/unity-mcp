using System.IO;
using NUnit.Framework;
using UnityEngine;

namespace Bridge.Editor.Tests
{
    [TestFixture]
    public class McpFilePathManagerTests
    {
        private string testProjectPath;
        
        [SetUp]
        public void SetUp()
        {
            // Mock Application.dataPath for testing
            testProjectPath = Path.Combine(Path.GetTempPath(), "UnityMCPTest");
        }
        
        [TearDown]
        public void TearDown()
        {
            // Clean up any test directories
            if (Directory.Exists(testProjectPath))
            {
                Directory.Delete(testProjectPath, true);
            }
        }
        
        [Test]
        public void GetUnityMcpBasePath_ShouldReturnCorrectPath()
        {
            // Given: Base Unity project path
            // When: Getting UnityMCP base path
            var basePath = McpFilePathManager.GetUnityMcpBasePath();
            
            // Then: Should return ../UnityMCP relative to Assets
            Assert.IsNotNull(basePath, "Base path should not be null");
            Assert.IsTrue(basePath.EndsWith("UnityMCP"), "Base path should end with 'UnityMCP'");
        }
        
        [Test]
        public void GetDiagnosticsDirectory_ShouldReturnCorrectPath()
        {
            // Given: UnityMCP base path exists
            // When: Getting diagnostics directory
            var diagnosticsPath = McpFilePathManager.GetDiagnosticsDirectory();
            
            // Then: Should return UnityMCP/diagnostics/
            Assert.IsNotNull(diagnosticsPath, "Diagnostics path should not be null");
            Assert.IsTrue(diagnosticsPath.Contains("diagnostics"), "Path should contain 'diagnostics'");
        }
        
        [Test]
        public void GetTestsDirectory_ShouldReturnCorrectPath()
        {
            // Given: UnityMCP base path exists
            // When: Getting tests directory
            var testsPath = McpFilePathManager.GetTestsDirectory();
            
            // Then: Should return UnityMCP/tests/
            Assert.IsNotNull(testsPath, "Tests path should not be null");
            Assert.IsTrue(testsPath.Contains("tests"), "Path should contain 'tests'");
        }
        
        [Test]
        public void GetTestsRequestsDirectory_ShouldReturnCorrectPath()
        {
            // Given: UnityMCP base path exists
            // When: Getting tests requests directory
            var requestsPath = McpFilePathManager.GetTestsRequestsDirectory();
            
            // Then: Should return UnityMCP/tests/requests/
            Assert.IsNotNull(requestsPath, "Requests path should not be null");
            Assert.IsTrue(requestsPath.Contains("tests"), "Path should contain 'tests'");
            Assert.IsTrue(requestsPath.Contains("requests"), "Path should contain 'requests'");
        }
        
        [Test]
        public void EnsureDirectoryExists_ShouldCreateDirectory()
        {
            // Given: Non-existing directory path
            var testDir = Path.Combine(testProjectPath, "TestDirectory");
            Assert.IsFalse(Directory.Exists(testDir), "Directory should not exist initially");
            
            // When: Ensuring directory exists
            McpFilePathManager.EnsureDirectoryExists(testDir);
            
            // Then: Directory should be created
            Assert.IsTrue(Directory.Exists(testDir), "Directory should be created");
        }
        
        [Test]
        public void EnsureDirectoryExists_ShouldNotThrowIfDirectoryExists()
        {
            // Given: Existing directory
            var testDir = Path.Combine(testProjectPath, "ExistingDirectory");
            Directory.CreateDirectory(testDir);
            Assert.IsTrue(Directory.Exists(testDir), "Directory should exist initially");
            
            // When: Ensuring directory exists on existing directory
            // Then: Should not throw
            Assert.DoesNotThrow(() => McpFilePathManager.EnsureDirectoryExists(testDir),
                "Should not throw when directory already exists");
        }
        
        [Test]
        public void GetLatestJsonPath_ShouldReturnCorrectPath()
        {
            // Given: Base directory path
            var baseDir = "/some/test/path";
            
            // When: Getting latest.json path
            var latestPath = McpFilePathManager.GetLatestJsonPath(baseDir);
            
            // Then: Should return baseDir/latest.json
            Assert.IsNotNull(latestPath, "Latest path should not be null");
            Assert.AreEqual(Path.Combine(baseDir, "latest.json"), latestPath, 
                "Should return correct latest.json path");
        }
    }
}