using System;
using System.IO;
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
            if (req == null)
                throw new ArgumentNullException(nameof(req));
                
            var snap = await EditorDispatcher.RunOnMainAsync(() => new
            {
                compiling = UnityEditor.EditorApplication.isCompiling,
                updating = UnityEditor.EditorApplication.isUpdating,
                version = UnityEngine.Application.unityVersion,
            });

            var ready = !snap.compiling && !snap.updating;
            var status = ready ? "OK" : "BUSY";

            // Derive project path/name from Application.dataPath (Assets directory)
            var assetsPath = UnityEngine.Application.dataPath; // .../<Project>/Assets
            var projectRoot = Path.GetDirectoryName(assetsPath) ?? string.Empty;
            var projectName = string.IsNullOrEmpty(projectRoot) ? string.Empty : Path.GetFileName(projectRoot);

            return new IpcResponse
            {
                Health = new HealthResponse
                {
                    Ready = ready,
                    Version = snap.version,
                    Status = status,
                    ProjectName = projectName ?? string.Empty,
                    ProjectPath = projectRoot ?? string.Empty,
                }
            };
        }
#else
        public static Task<IpcResponse> HandleAsync(HealthRequest req)
        {
            if (req == null)
                throw new ArgumentNullException(nameof(req));
                
            var ready = !EditorStateMirror.IsCompiling && !EditorStateMirror.IsUpdating;
            var status = ready ? "OK" : "BUSY";

            var assetsPath = UnityEngine.Application.dataPath; // .../<Project>/Assets
            var projectRoot = Path.GetDirectoryName(assetsPath) ?? string.Empty;
            var projectName = string.IsNullOrEmpty(projectRoot) ? string.Empty : Path.GetFileName(projectRoot);

            var resp = new IpcResponse
            {
                Health = new HealthResponse
                {
                    Ready = ready,
                    Version = EditorStateMirror.UnityVersion,
                    Status = status,
                    ProjectName = projectName ?? string.Empty,
                    ProjectPath = projectRoot ?? string.Empty,
                }
            };
            return Task.FromResult(resp);
        }
#endif
    }
}
