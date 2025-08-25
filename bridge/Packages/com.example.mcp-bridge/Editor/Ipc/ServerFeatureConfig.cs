using System;
using System.Collections.Generic;
using UnityEditor;
using UnityEngine;

namespace Bridge.Editor.Ipc
{
    [InitializeOnLoad]
    public static class ServerFeatureConfig
    {
        private static bool _isBuildSystemAvailable = true; // Cache for build system availability
        
        static ServerFeatureConfig()
        {
            // Initialize cache on main thread
            UpdateBuildSystemAvailabilityCache();
            
            // Update cache when play mode changes
            EditorApplication.playModeStateChanged += OnPlayModeStateChanged;
        }
        
        private static void OnPlayModeStateChanged(PlayModeStateChange state)
        {
            // Update cache when play mode state changes
            UpdateBuildSystemAvailabilityCache();
        }
        
        private static void UpdateBuildSystemAvailabilityCache()
        {
            // This runs on main thread, safe to access Unity APIs
            _isBuildSystemAvailable = !EditorApplication.isPlayingOrWillChangePlaymode;
        }

        public static HashSet<FeatureFlag> GetEnabledFeatures()
        {
            var features = new HashSet<FeatureFlag>();
            
            // Always enabled features
            features.Add(FeatureFlag.AssetsBasic);
            features.Add(FeatureFlag.EventsLog);
            features.Add(FeatureFlag.OpsProgress);
            
            // Conditionally enabled features (using cached value)
            if (_isBuildSystemAvailable)
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
    }
}