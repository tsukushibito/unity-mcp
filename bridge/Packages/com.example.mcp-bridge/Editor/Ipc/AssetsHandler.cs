// Unity MCP Bridge - Assets Handler
// Handles all AssetDatabase operations via IPC
using UnityEditor;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;
using System.Collections.Generic;

namespace Mcp.Unity.V1.Ipc
{
    internal static class AssetsHandler
    {
        /// <summary>
        /// Validate Unity-relative project path
        /// </summary>
        private static bool IsValidUnityPath(string p)
            => !string.IsNullOrEmpty(p) && !p.StartsWith("..") && !System.IO.Path.IsPathRooted(p) && p.StartsWith("Assets/");

        /// <summary>
        /// Main dispatch handler for Assets requests
        /// </summary>
        public static Pb.AssetsResponse Handle(Pb.AssetsRequest req, Bridge.Editor.Ipc.FeatureGuard features)
        {
            // Require assets.basic feature for all assets operations
            features.RequireFeature(Bridge.Editor.Ipc.FeatureFlag.AssetsBasic);
            
            switch (req.PayloadCase)
            {
                case Pb.AssetsRequest.PayloadOneofCase.Import:  return Import(req.Import);
                case Pb.AssetsRequest.PayloadOneofCase.Move:    return Move(req.Move);
                case Pb.AssetsRequest.PayloadOneofCase.Delete:  return Delete(req.Delete);
                case Pb.AssetsRequest.PayloadOneofCase.Refresh: return Refresh(req.Refresh);
                case Pb.AssetsRequest.PayloadOneofCase.G2P:     return G2P(req.G2P);
                case Pb.AssetsRequest.PayloadOneofCase.P2G:     return P2G(req.P2G);
                default: return new Pb.AssetsResponse { StatusCode = 2, Message = "invalid request" };
            }
        }

        /// <summary>
        /// Handle Import Asset request with progress tracking
        /// </summary>
        private static Pb.AssetsResponse Import(Pb.ImportAssetRequest r)
        {
            // Long operation: track progress via OperationTracker
            string op = OperationTracker.Start("Import", $"Import {r.Paths.Count} items");
            try 
            {
                var results = new List<Pb.ImportAssetResult>(r.Paths.Count);
                int done = 0;
                foreach (var p in r.Paths)
                {
                    if (!IsValidUnityPath(p)) 
                    { 
                        results.Add(new Pb.ImportAssetResult{ Path = p, Ok = false, Message = "invalid path"}); 
                        continue; 
                    }
                    string guidBefore = AssetDatabase.AssetPathToGUID(p);
                    try 
                    {
                        if (r.Recursive)
                            AssetDatabase.ImportAsset(p, ImportAssetOptions.ImportRecursive);
                        else
                            AssetDatabase.ImportAsset(p);
                        if (r.AutoRefresh) AssetDatabase.Refresh();
                        string guid = AssetDatabase.AssetPathToGUID(p);
                        results.Add(new Pb.ImportAssetResult{ Path = p, Guid = guid, Ok = true});
                    } 
                    catch (Exception ex) 
                    {
                        results.Add(new Pb.ImportAssetResult{ Path = p, Ok = false, Message = ex.Message});
                    }
                    done++;
                    OperationTracker.Progress(op, (int)(100.0 * done / Math.Max(1, r.Paths.Count)));
                }
                OperationTracker.Complete(op, 0, "OK");
                return new Pb.AssetsResponse { StatusCode = 0, Import = new Pb.ImportAssetResponse { Results = { results } } };
            } 
            catch (Exception ex) 
            {
                OperationTracker.Complete(op, 13, ex.Message);
                return new Pb.AssetsResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        /// <summary>
        /// Handle Move Asset request
        /// </summary>
        private static Pb.AssetsResponse Move(Pb.MoveAssetRequest r)
        {
            if (!IsValidUnityPath(r.FromPath) || !IsValidUnityPath(r.ToPath))
                return new Pb.AssetsResponse { StatusCode = 2, Message = "invalid path" };
            
            string error = AssetDatabase.MoveAsset(r.FromPath, r.ToPath);
            if (!string.IsNullOrEmpty(error))
                return new Pb.AssetsResponse { StatusCode = 13, Message = error };
            
            return new Pb.AssetsResponse { StatusCode = 0, Move = new Pb.MoveAssetResponse { Ok = true, NewGuid = AssetDatabase.AssetPathToGUID(r.ToPath) } };
        }

        /// <summary>
        /// Handle Delete Asset request
        /// </summary>
        private static Pb.AssetsResponse Delete(Pb.DeleteAssetRequest r)
        {
            var deleted = new List<string>();
            var failed = new List<string>();
            foreach (var p in r.Paths)
            {
                if (!IsValidUnityPath(p)) 
                { 
                    failed.Add(p); 
                    continue; 
                }
                bool ok = r.Soft ? AssetDatabase.MoveAssetToTrash(p) : AssetDatabase.DeleteAsset(p);
                (ok ? deleted : failed).Add(p);
            }
            return new Pb.AssetsResponse { StatusCode = 0, Delete = new Pb.DeleteAssetResponse { Deleted = { deleted }, Failed = { failed } } };
        }

        /// <summary>
        /// Handle Refresh request
        /// </summary>
        private static Pb.AssetsResponse Refresh(Pb.RefreshRequest r)
        {
            AssetDatabase.Refresh(r.Force ? ImportAssetOptions.ForceUpdate : ImportAssetOptions.Default);
            return new Pb.AssetsResponse { StatusCode = 0, Refresh = new Pb.RefreshResponse { Ok = true } };
        }

        /// <summary>
        /// Handle GUID to Path conversion
        /// </summary>
        private static Pb.AssetsResponse G2P(Pb.GuidToPathRequest r)
        {
            var map = new Pb.GuidToPathResponse();
            foreach (var g in r.Guids)
                map.Map[g] = AssetDatabase.GUIDToAssetPath(g);
            return new Pb.AssetsResponse { StatusCode = 0, G2P = map };
        }

        /// <summary>
        /// Handle Path to GUID conversion
        /// </summary>
        private static Pb.AssetsResponse P2G(Pb.PathToGuidRequest r)
        {
            var map = new Pb.PathToGuidResponse();
            foreach (var p in r.Paths)
                map.Map[p] = AssetDatabase.AssetPathToGUID(p);
            return new Pb.AssetsResponse { StatusCode = 0, P2G = map };
        }
    }
}