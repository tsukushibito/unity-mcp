using System;
using System.Collections.Generic;
using System.Linq;

namespace Bridge.Editor.Ipc
{
    // Unity C# equivalent of Rust FeatureFlag
    public enum FeatureFlag
    {
        AssetsBasic,
        BuildMin,
        EventsLog,
        OpsProgress,
        AssetsAdvanced,
        BuildFull,
        EventsFull,
        Unknown
    }

    public static class FeatureFlagExtensions
    {
        private static readonly Dictionary<string, FeatureFlag> StringToFlag = new Dictionary<string, FeatureFlag>
        {
            { "assets.basic", FeatureFlag.AssetsBasic },
            { "build.min", FeatureFlag.BuildMin },
            { "events.log", FeatureFlag.EventsLog },
            { "ops.progress", FeatureFlag.OpsProgress },
            { "assets.advanced", FeatureFlag.AssetsAdvanced },
            { "build.full", FeatureFlag.BuildFull },
            { "events.full", FeatureFlag.EventsFull },
        };
        
        private static readonly Dictionary<FeatureFlag, string> FlagToString = 
            StringToFlag.ToDictionary(kvp => kvp.Value, kvp => kvp.Key);
        
        public static FeatureFlag FromString(string str)
        {
            var normalized = str.Trim().ToLowerInvariant();
            return StringToFlag.TryGetValue(normalized, out var flag) ? flag : FeatureFlag.Unknown;
        }
        
        public static string ToString(this FeatureFlag flag)
        {
            return FlagToString.TryGetValue(flag, out var str) ? str : "unknown";
        }
        
        public static HashSet<FeatureFlag> GetServerSupportedFeatures()
        {
            return new HashSet<FeatureFlag>
            {
                FeatureFlag.AssetsBasic,
                FeatureFlag.BuildMin,
                FeatureFlag.EventsLog,
                FeatureFlag.OpsProgress,
                // Note: AssetsAdvanced, BuildFull, EventsFull not yet implemented
            };
        }
        
        /// <summary>
        /// Get intersection of client and server features
        /// </summary>
        public static List<string> NegotiateFeatures(IEnumerable<string> clientFeatures)
        {
            var clientFlags = clientFeatures
                .Select(FromString)
                .Where(f => f != FeatureFlag.Unknown)
                .ToHashSet();
            
            var serverFlags = GetServerSupportedFeatures();
            
            // Intersection - only features supported by both sides
            var acceptedFeatures = clientFlags
                .Intersect(serverFlags)
                .Select(f => f.ToString())
                .ToList();
            
            return acceptedFeatures;
        }
    }
}