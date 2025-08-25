#if UNITY_EDITOR
using UnityEditor;
using UnityEngine;

namespace Bridge.Editor.Ipc.Infra
{
    internal static class EditorStateMirror
    {
        public static volatile bool IsCompiling;
        public static volatile bool IsUpdating;
        public static volatile string UnityVersion = "unknown";

        [InitializeOnLoadMethod]
        private static void Init()
        {
            // Immediate first refresh to avoid "unknown" at startup
            RefreshOnce();
            EditorApplication.update += () => RefreshOnce();
        }

        private static void RefreshOnce()
        {
            IsCompiling = EditorApplication.isCompiling;
            IsUpdating  = EditorApplication.isUpdating;
            UnityVersion = Application.unityVersion;
        }
    }
}
#endif