using System.Threading.Tasks;
using Bridge.Editor.Ipc.Infra;
using Mcp.Unity.V1;

namespace Bridge.Editor.Ipc.Handlers
{
    internal static class HealthHandler
    {
#if HEALTH_STRICT
        public static async Task<IpcResponse> HandleAsync(HealthRequest req)
        {
            var snap = await EditorDispatcher.RunOnMainAsync(() => new
            {
                compiling = UnityEditor.EditorApplication.isCompiling,
                updating = UnityEditor.EditorApplication.isUpdating,
                version = UnityEngine.Application.unityVersion,
            });

            var ready = !snap.compiling && !snap.updating;
            var status = ready ? "OK" : "BUSY";

            return new IpcResponse
            {
                Health = new HealthResponse
                {
                    Ready = ready,
                    Version = snap.version,
                    Status = status,
                }
            };
        }
#else
        public static Task<IpcResponse> HandleAsync(HealthRequest req)
        {
            var ready = !EditorStateMirror.IsCompiling && !EditorStateMirror.IsUpdating;
            var status = ready ? "OK" : "BUSY";

            var resp = new IpcResponse
            {
                Health = new HealthResponse
                {
                    Ready = ready,
                    Version = EditorStateMirror.UnityVersion,
                    Status = status,
                }
            };
            return Task.FromResult(resp);
        }
#endif
    }
}