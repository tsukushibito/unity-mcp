using System;
using System.IO;
using UnityEngine;

namespace Bridge.Editor
{
    /// <summary>
    /// Manages unified file paths for Unity MCP temporary files and directories.
    /// Provides centralized path management to replace hardcoded paths across the codebase.
    /// </summary>
    public static class McpFilePathManager
    {
        private static readonly string UnityMcpBasePath = Path.Combine(Application.dataPath, "../UnityMCP");
        private static readonly string DiagnosticsDirectoryPath = Path.Combine(UnityMcpBasePath, "diagnostics");
        private static readonly string TestsDirectoryPath = Path.Combine(UnityMcpBasePath, "tests");
        private static readonly string TestsRequestsDirectoryPath = Path.Combine(TestsDirectoryPath, "requests");
        
        /// <summary>
        /// Gets the base UnityMCP directory path.
        /// </summary>
        /// <returns>Path to ../UnityMCP relative to Assets</returns>
        public static string GetUnityMcpBasePath()
        {
            return UnityMcpBasePath;
        }
        
        /// <summary>
        /// Gets the diagnostics directory path for McpDiagnosticsReporter.
        /// </summary>
        /// <returns>Path to ../UnityMCP/diagnostics/</returns>
        public static string GetDiagnosticsDirectory()
        {
            return DiagnosticsDirectoryPath;
        }
        
        /// <summary>
        /// Gets the tests directory path for McpTestRunner.
        /// </summary>
        /// <returns>Path to ../UnityMCP/tests/</returns>
        public static string GetTestsDirectory()
        {
            return TestsDirectoryPath;
        }
        
        /// <summary>
        /// Gets the tests requests directory path for McpTestRunner.
        /// </summary>
        /// <returns>Path to ../UnityMCP/tests/requests/</returns>
        public static string GetTestsRequestsDirectory()
        {
            return TestsRequestsDirectoryPath;
        }
        
        /// <summary>
        /// Ensures the specified directory exists, creating it if necessary.
        /// </summary>
        /// <param name="directoryPath">The directory path to ensure exists</param>
        /// <returns>True if directory exists or was created successfully, false otherwise</returns>
        public static bool EnsureDirectoryExists(string directoryPath)
        {
            try
            {
                if (string.IsNullOrEmpty(directoryPath))
                {
                    Debug.LogError("[McpFilePathManager] Directory path is null or empty");
                    return false;
                }

                if (Directory.Exists(directoryPath))
                {
                    return true;
                }

                // Create directory with explicit error handling
                DirectoryInfo dirInfo = Directory.CreateDirectory(directoryPath);
                
                // Verify creation was successful
                if (dirInfo.Exists)
                {
                    Debug.Log($"[McpFilePathManager] Successfully created directory: {directoryPath}");
                    return true;
                }
                else
                {
                    Debug.LogError($"[McpFilePathManager] Failed to verify directory creation: {directoryPath}");
                    return false;
                }
            }
            catch (UnauthorizedAccessException e)
            {
                Debug.LogError($"[McpFilePathManager] Access denied creating directory: {directoryPath}. Error: {e.Message}");
                return false;
            }
            catch (DirectoryNotFoundException e)
            {
                Debug.LogError($"[McpFilePathManager] Parent directory not found: {directoryPath}. Error: {e.Message}");
                return false;
            }
            catch (IOException e)
            {
                Debug.LogError($"[McpFilePathManager] IO error creating directory: {directoryPath}. Error: {e.Message}");
                return false;
            }
            catch (Exception e)
            {
                Debug.LogError($"[McpFilePathManager] Unexpected error creating directory: {directoryPath}. Error: {e.Message}");
                return false;
            }
        }
        
        /// <summary>
        /// Gets the path for latest.json file in the specified directory.
        /// </summary>
        /// <param name="baseDirectory">The base directory</param>
        /// <returns>Path to latest.json file</returns>
        public static string GetLatestJsonPath(string baseDirectory)
        {
            return Path.Combine(baseDirectory, "latest.json");
        }
    }
}