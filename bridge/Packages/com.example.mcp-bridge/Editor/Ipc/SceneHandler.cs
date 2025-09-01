// Unity MCP Bridge - Scene Handler
// Handles scene management operations via IPC
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;
using System.Collections.Generic;

namespace Mcp.Unity.V1.Ipc
{
    internal static class SceneHandler
    {
        private static bool IsValidScenePath(string p)
            => !string.IsNullOrEmpty(p) && !System.IO.Path.IsPathRooted(p) && !p.StartsWith("..") && p.StartsWith("Assets/") && p.EndsWith(".unity");

        public static Pb.ScenesResponse Handle(Pb.ScenesRequest req, Bridge.Editor.Ipc.FeatureGuard features)
        {
            switch (req.PayloadCase)
            {
                case Pb.ScenesRequest.PayloadOneofCase.Open: return Open(req.Open);
                case Pb.ScenesRequest.PayloadOneofCase.Save: return Save(req.Save);
                case Pb.ScenesRequest.PayloadOneofCase.GetOpen: return GetOpen(req.GetOpen);
                case Pb.ScenesRequest.PayloadOneofCase.SetActive: return SetActive(req.SetActive);
                default: return new Pb.ScenesResponse { StatusCode = 2, Message = "invalid request" };
            }
        }

        private static Pb.ScenesResponse Open(Pb.OpenSceneRequest r)
        {
            if (!IsValidScenePath(r.Path))
                return new Pb.ScenesResponse { StatusCode = 2, Message = "invalid path" };
            try
            {
                var mode = r.Additive ? OpenSceneMode.Additive : OpenSceneMode.Single;
                EditorSceneManager.OpenScene(r.Path, mode);
                return new Pb.ScenesResponse { StatusCode = 0, Open = new Pb.OpenSceneResponse { Ok = true } };
            }
            catch (Exception ex)
            {
                return new Pb.ScenesResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        private static Pb.ScenesResponse Save(Pb.SaveSceneRequest r)
        {
            if (!IsValidScenePath(r.Path))
                return new Pb.ScenesResponse { StatusCode = 2, Message = "invalid path" };
            try
            {
                var scene = EditorSceneManager.GetSceneByPath(r.Path);
                bool ok;
                if (scene.IsValid())
                    ok = EditorSceneManager.SaveScene(scene);
                else
                    ok = EditorSceneManager.SaveScene(EditorSceneManager.GetActiveScene(), r.Path);
                return new Pb.ScenesResponse { StatusCode = 0, Save = new Pb.SaveSceneResponse { Ok = ok } };
            }
            catch (Exception ex)
            {
                return new Pb.ScenesResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        private static Pb.ScenesResponse GetOpen(Pb.GetOpenScenesRequest r)
        {
            var resp = new Pb.GetOpenScenesResponse();
            int count = EditorSceneManager.sceneCount;
            for (int i = 0; i < count; i++)
            {
                var s = EditorSceneManager.GetSceneAt(i);
                resp.Scenes.Add(s.path);
                if (s == EditorSceneManager.GetActiveScene())
                    resp.ActiveScene = s.path;
            }
            return new Pb.ScenesResponse { StatusCode = 0, GetOpen = resp };
        }

        private static Pb.ScenesResponse SetActive(Pb.SetActiveSceneRequest r)
        {
            if (!IsValidScenePath(r.Path))
                return new Pb.ScenesResponse { StatusCode = 2, Message = "invalid path" };
            try
            {
                var scene = EditorSceneManager.GetSceneByPath(r.Path);
                bool ok = EditorSceneManager.SetActiveScene(scene);
                return new Pb.ScenesResponse { StatusCode = ok ? 0 : 13, SetActive = new Pb.SetActiveSceneResponse { Ok = ok } };
            }
            catch (Exception ex)
            {
                return new Pb.ScenesResponse { StatusCode = 13, Message = ex.Message };
            }
        }
    }
}
