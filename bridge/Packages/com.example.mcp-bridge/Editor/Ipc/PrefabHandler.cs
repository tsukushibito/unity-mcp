// Unity MCP Bridge - Prefab Handler
// Handles prefab operations via IPC
using UnityEditor;
using UnityEngine;
using Pb = Mcp.Unity.V1;

namespace Mcp.Unity.V1.Ipc
{
    internal static class PrefabHandler
    {
        private static bool IsValidUnityPath(string p)
            => !string.IsNullOrEmpty(p) && !p.StartsWith("..") && !System.IO.Path.IsPathRooted(p) && p.StartsWith("Assets/");

        public static Pb.PrefabResponse Handle(Pb.PrefabRequest req, Bridge.Editor.Ipc.FeatureGuard features)
        {
            features.RequireFeature(Bridge.Editor.Ipc.FeatureFlag.PrefabsBasic);
            switch (req.PayloadCase)
            {
                case Pb.PrefabRequest.PayloadOneofCase.Create:
                    return Create(req.Create);
                case Pb.PrefabRequest.PayloadOneofCase.Update:
                    return Update(req.Update);
                case Pb.PrefabRequest.PayloadOneofCase.ApplyOverrides:
                    return ApplyOverrides(req.ApplyOverrides);
                default:
                    return new Pb.PrefabResponse { StatusCode = 2, Message = "invalid request" };
            }
        }

        private static Pb.PrefabResponse Create(Pb.CreatePrefabRequest r)
        {
            if (!IsValidUnityPath(r.PrefabPath))
                return new Pb.PrefabResponse { StatusCode = 2, Message = "invalid path" };
            var go = GameObject.Find(r.GameObjectPath);
            if (go == null)
                return new Pb.PrefabResponse { StatusCode = 5, Message = "game object not found" };
            try
            {
                var prefab = PrefabUtility.SaveAsPrefabAsset(go, r.PrefabPath);
                if (prefab == null)
                    return new Pb.PrefabResponse { StatusCode = 13, Message = "failed to save prefab" };
                string guid = AssetDatabase.AssetPathToGUID(r.PrefabPath);
                return new Pb.PrefabResponse
                {
                    StatusCode = 0,
                    Create = new Pb.CreatePrefabResponse { Ok = true, Guid = guid }
                };
            }
            catch (System.Exception ex)
            {
                return new Pb.PrefabResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        private static Pb.PrefabResponse Update(Pb.UpdatePrefabRequest r)
        {
            if (!IsValidUnityPath(r.PrefabPath))
                return new Pb.PrefabResponse { StatusCode = 2, Message = "invalid path" };
            var go = GameObject.Find(r.GameObjectPath);
            if (go == null)
                return new Pb.PrefabResponse { StatusCode = 5, Message = "game object not found" };
            try
            {
                var prefab = PrefabUtility.SaveAsPrefabAsset(go, r.PrefabPath);
                if (prefab == null)
                    return new Pb.PrefabResponse { StatusCode = 13, Message = "failed to save prefab" };
                return new Pb.PrefabResponse
                {
                    StatusCode = 0,
                    Update = new Pb.UpdatePrefabResponse { Ok = true }
                };
            }
            catch (System.Exception ex)
            {
                return new Pb.PrefabResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        private static Pb.PrefabResponse ApplyOverrides(Pb.ApplyPrefabOverridesRequest r)
        {
            var go = GameObject.Find(r.InstancePath);
            if (go == null)
                return new Pb.PrefabResponse { StatusCode = 5, Message = "instance not found" };
            try
            {
                PrefabUtility.ApplyPrefabInstance(go, InteractionMode.UserAction);
                return new Pb.PrefabResponse
                {
                    StatusCode = 0,
                    ApplyOverrides = new Pb.ApplyPrefabOverridesResponse { Ok = true }
                };
            }
            catch (System.Exception ex)
            {
                return new Pb.PrefabResponse { StatusCode = 13, Message = ex.Message };
            }
        }
    }
}
