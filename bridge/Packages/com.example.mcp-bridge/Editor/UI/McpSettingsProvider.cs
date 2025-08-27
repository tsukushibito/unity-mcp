using UnityEditor;
using UnityEngine;

namespace Mcp.Unity.Editor.UI
{
    /// <summary>
    /// Project Settings page for configuring the IPC token stored in EditorUserSettings.
    /// Writes/reads only EditorUserSettings["MCP.IpcToken"].
    /// </summary>
    public static class McpSettingsProvider
    {
        private const string SettingsPath = "Project/MCP Bridge";
        private const string TokenKey = "MCP.IpcToken";

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
            var current = EditorUserSettings.GetConfigValue(TokenKey) ?? string.Empty;

            EditorGUILayout.LabelField("IPC Token", EditorStyles.boldLabel);
            EditorGUILayout.HelpBox(
                "This value is stored in EditorUserSettings (per-user, per-project) under 'MCP.IpcToken'.\n" +
                "Environment variables and EditorPrefs are ignored by design.",
                MessageType.Info);

            using (new EditorGUILayout.HorizontalScope())
            {
                var newValue = EditorGUILayout.TextField("Token", current);
                if (newValue != current)
                {
                    EditorUserSettings.SetConfigValue(TokenKey, newValue ?? string.Empty);
                    current = newValue ?? string.Empty;
                }

                if (GUILayout.Button("Clear", GUILayout.Width(80)))
                {
                    EditorUserSettings.SetConfigValue(TokenKey, string.Empty);
                    current = string.Empty;
                }
            }

            if (string.IsNullOrEmpty(current))
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
        }

        [MenuItem("MCP Bridge/Setup/Open Project Settings")]
        public static void OpenProjectSettings()
        {
            SettingsService.OpenProjectSettings(SettingsPath);
        }
    }
}

