#if UNITY_EDITOR
using System.Threading;
using UnityEngine;
using UnityEditor;

namespace Bridge.Editor.Ipc.Infra
{
    internal static class MainThreadGuard
    {
        private static int _mainId;

        [InitializeOnLoadMethod]
        private static void Init()
        {
            _mainId = Thread.CurrentThread.ManagedThreadId;
        }

        [System.Diagnostics.Conditional("UNITY_EDITOR"), System.Diagnostics.Conditional("DEBUG")]
        public static void AssertMainThread()
        {
            if (Thread.CurrentThread.ManagedThreadId != _mainId)
            {
#if BRIDGE_THREAD_GUARD_STRICT
                throw new System.InvalidOperationException($"Unity API on BG thread. Expected main={_mainId}, got={Thread.CurrentThread.ManagedThreadId}");
#else
                Debug.LogError($"Unity API on BG thread. Expected main={_mainId}, got={Thread.CurrentThread.ManagedThreadId}");
#endif
            }
        }
    }
}
#endif