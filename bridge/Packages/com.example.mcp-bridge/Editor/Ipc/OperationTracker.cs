// Unity MCP Bridge - Operation Tracker
// Simple API to wrap long tasks and emit START/PROGRESS/COMPLETE events
using System;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc
{
    internal static class OperationTracker
    {
        public static string Start(string kind, string message)
        {
            string id = Guid.NewGuid().ToString("n");
            Publish(new Pb.OperationEvent 
            { 
                OpId = id, 
                Kind = Pb.OperationEvent.Types.Kind.Start, 
                Message = message 
            });
            return id;
        }

        public static void Progress(string id, int pct, string msg = "")
        {
            Publish(new Pb.OperationEvent 
            { 
                OpId = id, 
                Kind = Pb.OperationEvent.Types.Kind.Progress, 
                Progress = pct, 
                Message = msg 
            });
        }

        public static void Complete(string id, int code, string msg = "")
        {
            Publish(new Pb.OperationEvent 
            { 
                OpId = id, 
                Kind = Pb.OperationEvent.Types.Kind.Complete, 
                Progress = 100, 
                Code = code, 
                Message = msg 
            });
        }

        private static void Publish(Pb.OperationEvent op)
        {
            var ev = new Pb.IpcEvent 
            { 
                MonotonicTsNs = NowNs(), 
                Op = op 
            };
            IpcEventSender.TryEnqueue(ev);
        }

        private static long NowNs()
        {
            return (long)(System.Diagnostics.Stopwatch.GetTimestamp() * (1e9 / System.Diagnostics.Stopwatch.Frequency));
        }
    }
}