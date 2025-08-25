// Unity MCP Bridge - Thread Diagnostics Helper
// Tracks main thread ID and provides thread-aware logging for cross-thread debugging
#if UNITY_EDITOR && DEBUG
using System;
using System.Threading;
using UnityEngine;

namespace Mcp.Unity.V1.Ipc.Infra
{
    /// <summary>
    /// Thread diagnostics helper for tracking Unity API cross-thread issues
    /// </summary>
    internal static class Diag
    {
        public static int MainThreadId { get; private set; }

        [UnityEditor.InitializeOnLoadMethod]
        private static void Init()
        {
            MainThreadId = Thread.CurrentThread.ManagedThreadId;
            Log($"MainThreadId={MainThreadId}");
        }

        /// <summary>
        /// Get current thread tag for logging
        /// </summary>
        public static string ThreadTag() =>
            Thread.CurrentThread.ManagedThreadId == MainThreadId ? "MAIN" : "BG";

        /// <summary>
        /// Log message with thread information
        /// </summary>
        public static void Log(string msg)
            => Debug.Log($"[BRIDGE.THREAD {ThreadTag()}] {msg}");

        /// <summary>
        /// Log warning with thread information
        /// </summary>
        public static void LogWarning(string msg)
            => Debug.LogWarning($"[BRIDGE.THREAD {ThreadTag()}] {msg}");

        /// <summary>
        /// Log error with thread information
        /// </summary>
        public static void LogError(string msg)
            => Debug.LogError($"[BRIDGE.THREAD {ThreadTag()}] {msg}");

        /// <summary>
        /// Check if currently on Unity main thread
        /// </summary>
        public static bool IsMainThread() =>
            Thread.CurrentThread.ManagedThreadId == MainThreadId;

        /// <summary>
        /// Log Unity API access with caller info
        /// </summary>
        public static void LogUnityApiAccess(string api, string caller = null)
        {
            var threadInfo = IsMainThread() ? "SAFE" : "UNSAFE";
            var message = $"Unity API Access: {api} [{threadInfo}]";
            if (!string.IsNullOrEmpty(caller))
                message += $" from {caller}";
            
            if (IsMainThread())
                Log(message);
            else
                LogError($"CROSS-THREAD VIOLATION: {message}");
        }
    }
}
#endif