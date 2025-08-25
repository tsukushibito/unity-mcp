// Unity MCP Bridge - Editor Log Bridge
// Captures Unity logs and converts them to IPC events
using System;
using UnityEngine;
using UnityEditor;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc
{
    [InitializeOnLoad]
    internal static class EditorLogBridge
    {
        static EditorLogBridge()
        {
            // TODO(UNITY_API): touches Application.logMessageReceivedThreaded — must run on main via EditorDispatcher
            Application.logMessageReceivedThreaded += OnLog;
        }

        private static void OnLog(string condition, string stackTrace, LogType type)
        {
            var lvl = type switch
            {
                LogType.Error or LogType.Exception => Pb.LogEvent.Types.Level.Error,
                LogType.Assert or LogType.Warning => Pb.LogEvent.Types.Level.Warn,
                LogType.Log => Pb.LogEvent.Types.Level.Info,
                _ => Pb.LogEvent.Types.Level.Debug,
            };
            
            var ev = new Pb.IpcEvent
            {
                MonotonicTsNs = NowNs(),
                Log = new Pb.LogEvent
                {
                    MonotonicTsNs = NowNs(),
                    Level = lvl,
                    Message = condition ?? string.Empty,
                    Category = "Unity",
                    StackTrace = stackTrace ?? string.Empty,
                }
            };
            
            IpcEventSender.TryEnqueue(ev); // thread-safe queue (see next implementation)
        }

        private static long NowNs()
        {
            return (long)(System.Diagnostics.Stopwatch.GetTimestamp() * (1e9 / System.Diagnostics.Stopwatch.Frequency));
        }
    }
}