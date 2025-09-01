using UnityEditor;
using UnityEngine;

namespace Mcp.Unity.Editor.UI
{
    /// <summary>
    /// Project Settings page for configuring MCP Bridge IPC settings stored in EditorUserSettings.
    /// Manages MCP.IpcToken and MCP.IpcPort configuration.
    /// </summary>
    public static class McpSettingsProvider
    {
        private const string SettingsPath = "Project/MCP Bridge";
        private const string TokenKey = "MCP.IpcToken";
        private const string PortKey = "MCP.IpcPort";
        private const int DefaultPort = 7777;

        [SettingsProvider]
        public static SettingsProvider CreateSettingsProvider()
        {
            var provider = new SettingsProvider(SettingsPath, SettingsScope.Project)
            {
                label = "MCP Bridge",
                guiHandler = _ => DrawGui()
            };
            return provider;
        }

        private static void DrawGui()
        {
            var currentToken = EditorUserSettings.GetConfigValue(TokenKey) ?? string.Empty;
            var currentPortString = EditorUserSettings.GetConfigValue(PortKey) ?? string.Empty;
            var currentPort = ParsePortFromString(currentPortString);

            EditorGUILayout.Space(10);

            // IPC Token Section
            EditorGUILayout.LabelField("IPC Configuration", EditorStyles.boldLabel);
            EditorGUILayout.HelpBox(
                "These values are stored in EditorUserSettings (per-user, per-project).\n" +
                "Environment variables and EditorPrefs are ignored by design.",
                MessageType.Info);

            EditorGUILayout.Space(5);

            // Token Configuration
            EditorGUILayout.LabelField("Authentication Token", EditorStyles.miniBoldLabel);
            using (new EditorGUILayout.HorizontalScope())
            {
                var newToken = EditorGUILayout.TextField("Token", currentToken);
                if (newToken != currentToken)
                {
                    EditorUserSettings.SetConfigValue(TokenKey, newToken ?? string.Empty);
                    currentToken = newToken ?? string.Empty;
                }

                if (GUILayout.Button("Clear", GUILayout.Width(80)))
                {
                    EditorUserSettings.SetConfigValue(TokenKey, string.Empty);
                    currentToken = string.Empty;
                }
            }

            if (string.IsNullOrEmpty(currentToken))
            {
                EditorGUILayout.HelpBox(
                    "Token is empty. Connections will be rejected with UNAUTHENTICATED.",
                    MessageType.Warning);
            }
            else
            {
                EditorGUILayout.HelpBox(
                    "Token is set. Keep it secret and do not commit to VCS.",
                    MessageType.None);
            }

            EditorGUILayout.Space(10);

            // Port Configuration
            EditorGUILayout.LabelField("Port Configuration", EditorStyles.miniBoldLabel);
            using (new EditorGUILayout.HorizontalScope())
            {
                var newPort = EditorGUILayout.IntField("Port", currentPort);
                if (newPort != currentPort && IsValidPort(newPort))
                {
                    EditorUserSettings.SetConfigValue(PortKey, newPort.ToString());
                    currentPort = newPort;
                }

                if (GUILayout.Button("Default", GUILayout.Width(80)))
                {
                    EditorUserSettings.SetConfigValue(PortKey, DefaultPort.ToString());
                    currentPort = DefaultPort;
                }

                if (GUILayout.Button("Clear", GUILayout.Width(80)))
                {
                    EditorUserSettings.SetConfigValue(PortKey, string.Empty);
                    currentPort = DefaultPort;
                }
            }

            if (currentPort == DefaultPort && string.IsNullOrEmpty(currentPortString))
            {
                EditorGUILayout.HelpBox(
                    $"Using default port {DefaultPort}. Configure a custom port if needed.",
                    MessageType.Info);
            }
            else if (IsValidPort(currentPort))
            {
                EditorGUILayout.HelpBox(
                    $"IPC server will use port {currentPort}. Restart server to apply changes.",
                    MessageType.Info);
            }
            else
            {
                EditorGUILayout.HelpBox(
                    $"Invalid port {currentPort}. Must be between 1024 and 65535. Using default {DefaultPort}.",
                    MessageType.Warning);
            }

            EditorGUILayout.Space(5);

            // Server Status Section
            var serverStatus = GetServerStatus();
            EditorGUILayout.LabelField("Server Status", EditorStyles.miniBoldLabel);
            EditorGUILayout.HelpBox(serverStatus, MessageType.None);
        }

        private static int ParsePortFromString(string portString)
        {
            if (string.IsNullOrEmpty(portString))
            {
                return DefaultPort;
            }

            if (int.TryParse(portString, out int port) && IsValidPort(port))
            {
                return port;
            }

            return DefaultPort;
        }

        private static bool IsValidPort(int port)
        {
            return port >= 1024 && port <= 65535;
        }

        private static string GetServerStatus()
        {
            try
            {
                var serverType = System.Type.GetType("Bridge.Editor.Ipc.EditorIpcServer, Bridge.Editor");
                if (serverType == null)
                {
                    return "EditorIpcServer not found";
                }

                var isRunningProperty = serverType.GetProperty("IsRunning", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);
                var isReadyProperty = serverType.GetProperty("IsReady", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);
                var currentPortProperty = serverType.GetProperty("CurrentPort", System.Reflection.BindingFlags.Public | System.Reflection.BindingFlags.Static);

                var isRunning = isRunningProperty?.GetValue(null) as bool? ?? false;
                var isReady = isReadyProperty?.GetValue(null) as bool? ?? false;
                var currentPort = currentPortProperty?.GetValue(null) as int? ?? 0;

                if (isReady)
                {
                    return $"Server: Ready on port {currentPort}";
                }
                else if (isRunning)
                {
                    return $"Server: Starting on port {currentPort}...";
                }
                else
                {
                    return "Server: Not running";
                }
            }
            catch (System.Exception ex)
            {
                return $"Server status unavailable: {ex.Message}";
            }
        }

        [MenuItem("MCP Bridge/Setup/Open Project Settings")]
        public static void OpenProjectSettings()
        {
            SettingsService.OpenProjectSettings(SettingsPath);
        }
    }
}

