// Unity MCP Bridge - IPC Event Sender
// Thread-safe event queue with backpressure control
using System.Collections.Concurrent;
using System.Threading;
using System.Threading.Tasks;
using System.IO;
using UnityEngine;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc
{
    internal static class IpcEventSender
    {
        private static readonly ConcurrentQueue<Pb.IpcEvent> Q = new();
        private static int _started;

        public static void TryEnqueue(Pb.IpcEvent ev)
        {
            // Simple drop policy for low-importance events when congested
            if (Q.Count > 5000 && 
                ev.PayloadCase == Pb.IpcEvent.PayloadOneofCase.Log && 
                ev.Log.Level < Pb.LogEvent.Types.Level.Warn)
            {
                return;
            }
            
            Q.Enqueue(ev);
            
            if (Interlocked.Exchange(ref _started, 1) == 0) 
            {
                _ = PumpAsync();
            }
        }

        private static async Task PumpAsync()
        {
            try
            {
                // Assume EditorIpcServer manages a per-connection Stream reference
                while (EditorIpcServer.TryGetActiveStream(out var stream))
                {
                    while (Q.TryDequeue(out var ev))
                    {
                        try
                        {
                            var env = new Pb.IpcEnvelope { Event = ev };
                            var bytes = EnvelopeCodec.Encode(env);
                            await Framing.WriteFrameAsync(stream, bytes);
                        }
                        catch (System.Exception ex)
                        {
                            Debug.LogWarning($"[IpcEventSender] Failed to send event: {ex.Message}");
                            break; // Connection lost, exit pump loop
                        }
                    }
                    
                    await Task.Delay(10); // light pacing
                }
            }
            catch (System.Exception ex)
            {
                Debug.LogError($"[IpcEventSender] Pump error: {ex.Message}");
            }
            finally
            {
                Interlocked.Exchange(ref _started, 0);
            }
        }
    }
}