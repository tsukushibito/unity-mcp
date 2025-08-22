using System;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;

namespace Bridge.Editor.Ipc
{
    public static class ServerFeatureConfig
    {
        public static HashSet<FeatureFlag> GetEnabledFeatures()
        {
            var features = new HashSet<FeatureFlag>();
            
            // Always enabled features
            features.Add(FeatureFlag.AssetsBasic);
            features.Add(FeatureFlag.EventsLog);
            features.Add(FeatureFlag.OpsProgress);
            
            // Conditionally enabled features
            if (IsBuildSystemAvailable())
            {
                features.Add(FeatureFlag.BuildMin);
            }
            
            // Environment-based feature flags
            if (Environment.GetEnvironmentVariable("MCP_ENABLE_ADVANCED_ASSETS") == "true")
            {
                features.Add(FeatureFlag.AssetsAdvanced);
            }
            
            if (Environment.GetEnvironmentVariable("MCP_ENABLE_FULL_BUILD") == "true")
            {
                features.Add(FeatureFlag.BuildFull);
            }
            
            if (Environment.GetEnvironmentVariable("MCP_ENABLE_FULL_EVENTS") == "true")
            {
                features.Add(FeatureFlag.EventsFull);
            }
            
            return features;
        }
        
        private static bool IsBuildSystemAvailable()
        {
            // Check if build system is properly configured
            return !EditorApplication.isPlayingOrWillChangePlaymode;
        }
    }
}