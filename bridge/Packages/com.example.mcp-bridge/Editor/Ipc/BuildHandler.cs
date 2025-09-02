// Unity MCP Bridge - Build Handler
// Handles Player and AssetBundles build operations via IPC
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Mcp.Unity.V1.Ipc
{
    internal static class BuildHandler
    {
        /// <summary>
        /// Main dispatch handler for Build requests
        /// </summary>
        public static Pb.BuildResponse Handle(Pb.BuildRequest req, Bridge.Editor.Ipc.FeatureGuard features)
        {
            // Require build.min feature for all build operations
            features.RequireFeature(Bridge.Editor.Ipc.FeatureFlag.BuildMin);
            
            return req.PayloadCase switch
            {
                Pb.BuildRequest.PayloadOneofCase.Player  => new Pb.BuildResponse { Player  = BuildPlayer(req.Player) },
                Pb.BuildRequest.PayloadOneofCase.Bundles => new Pb.BuildResponse { Bundles = BuildBundles(req.Bundles) },
                _ => new Pb.BuildResponse { 
                    Player = new Pb.BuildPlayerResponse { 
                        StatusCode = 2, 
                        Message = "invalid build request" 
                    } 
                },
            };
        }

        /// <summary>
        /// Map protocol buffer platform to Unity build target
        /// </summary>
        private static (BuildTargetGroup, BuildTarget) MapTarget(Pb.BuildPlatform p, Pb.BuildVariants v)
        {
            return p switch
            {
                Pb.BuildPlatform.BpStandaloneWindows64 => (BuildTargetGroup.Standalone, BuildTarget.StandaloneWindows64),
                Pb.BuildPlatform.BpStandaloneOsx      => (BuildTargetGroup.Standalone, BuildTarget.StandaloneOSX),
                Pb.BuildPlatform.BpStandaloneLinux64  => (BuildTargetGroup.Standalone, BuildTarget.StandaloneLinux64),
                Pb.BuildPlatform.BpAndroid            => (BuildTargetGroup.Android,    BuildTarget.Android),
                Pb.BuildPlatform.BpIos                => (BuildTargetGroup.iOS,        BuildTarget.iOS),
                _ => throw new ArgumentOutOfRangeException(nameof(p), "unsupported platform"),
            };
        }

        /// <summary>
        /// Build Player implementation
        /// </summary>
        private static Pb.BuildPlayerResponse BuildPlayer(Pb.BuildPlayerRequest r)
        {
            try
            {
                // 1. Platform/variant processing
                var (group, target) = MapTarget(r.Platform, r.Variants);
                
                // 2. Path validation and normalization
                if (!PathPolicy.TryResolvePlayerOutput(r.OutputPath, out var outPath, out var pathError))
                    return new Pb.BuildPlayerResponse { StatusCode = 7, Message = pathError };

                // 3. Scene validation
                // TODO(UNITY_API): touches EditorBuildSettings — must run on main via EditorDispatcher
                string[] scenes = (r.Scenes.Count > 0) ? 
                    r.Scenes.ToArray() : 
                    EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();
                
                if (scenes.Length == 0) 
                    return new Pb.BuildPlayerResponse { StatusCode = 2, Message = "no scenes" };

                // 4. Platform switching
                // TODO(UNITY_API): touches EditorUserBuildSettings — must run on main via EditorDispatcher
                if (EditorUserBuildSettings.activeBuildTarget != target)
                {
                    if (!EditorUserBuildSettings.SwitchActiveBuildTarget(group, target))
                        return new Pb.BuildPlayerResponse {
                            StatusCode = 9,
                            Message = $"failed to switch build target to {target}"
                        };
                }

                // 4.5. Variant-specific settings
                // TODO(UNITY_API): touches PlayerSettings/EditorUserBuildSettings — must run on main via EditorDispatcher
                if (r.Variants?.Il2Cpp == true)
                    PlayerSettings.SetScriptingBackend(group, ScriptingImplementation.IL2CPP);
                else
                    PlayerSettings.SetScriptingBackend(group, ScriptingImplementation.Mono2x);

                if (r.Variants?.StripSymbols == true)
                    EditorUserBuildSettings.stripEngineCode = true;
                else
                    EditorUserBuildSettings.stripEngineCode = false;

                // 5. Build options setup
                var buildOptions = BuildOptions.None;
                if (r.Variants?.Development == true) buildOptions |= BuildOptions.Development;

                var bpo = new BuildPlayerOptions
                {
                    scenes = scenes,
                    target = target,
                    locationPathName = outPath,
                    options = buildOptions,
                };

                // 6. Execute build with OperationTracker
                string op = OperationTracker.Start("BuildPlayer", $"{target} -> {outPath}");
                var t0 = DateTime.UtcNow;
                
                // TODO(UNITY_API): touches BuildPipeline — must run on main via EditorDispatcher
                BuildReport report = BuildPipeline.BuildPlayer(bpo);
                var summary = report.summary;
                var ok = summary.result == BuildResult.Succeeded;
                
                OperationTracker.Complete(op, ok ? 0 : 13, summary.result.ToString());
                
                return new Pb.BuildPlayerResponse
                {
                    StatusCode = ok ? 0 : 13,
                    Message = ok ? "OK" : summary.result.ToString(),
                    OutputPath = outPath,
                    BuildTimeMs = (ulong)summary.totalTime.TotalMilliseconds,
                    SizeBytes = (ulong)Math.Max(0, (long)summary.totalSize),
                    // Warnings = { /* optionally collect from report */ },
                };
            }
            catch (Exception ex)
            {
                return new Pb.BuildPlayerResponse { StatusCode = 13, Message = ex.Message };
            }
        }

        /// <summary>
        /// Build Asset Bundles implementation
        /// </summary>
        private static Pb.BuildAssetBundlesResponse BuildBundles(Pb.BuildAssetBundlesRequest r)
        {
            try
            {
                // 1. Path validation and normalization  
                if (!PathPolicy.TryResolveBundlesOutput(r.OutputDirectory, out var outDir, out var pathError))
                    return new Pb.BuildAssetBundlesResponse { StatusCode = 7, Message = pathError };

                // 2. Build options setup
                var opts = BuildAssetBundleOptions.None;
                // Note: DeterministicAssetBundle is always enabled in Unity 5.0+ BuildAssetBundles API
                if (r.ChunkBased)   opts |= BuildAssetBundleOptions.ChunkBasedCompression;
                if (r.ForceRebuild) opts |= BuildAssetBundleOptions.ForceRebuildAssetBundle;

                // 3. Execute build with OperationTracker
                string op = OperationTracker.Start("BuildBundles", outDir);
                var t0 = DateTime.UtcNow;
                
                // Use current active target (as per investigation results)
                // TODO(UNITY_API): touches EditorUserBuildSettings/BuildPipeline — must run on main via EditorDispatcher
                var target = EditorUserBuildSettings.activeBuildTarget;
                BuildPipeline.BuildAssetBundles(outDir, opts, target);
                
                OperationTracker.Complete(op, 0, "OK");
                
                return new Pb.BuildAssetBundlesResponse
                {
                    StatusCode = 0,
                    Message = "OK",
                    OutputDirectory = outDir,
                    BuildTimeMs = (ulong)(DateTime.UtcNow - t0).TotalMilliseconds,
                };
            }
            catch (Exception ex)
            {
                return new Pb.BuildAssetBundlesResponse { StatusCode = 13, Message = ex.Message };
            }
        }
    }
}