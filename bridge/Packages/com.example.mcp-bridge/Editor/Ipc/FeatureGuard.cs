using System;
using System.Collections.Generic;
using System.Linq;

namespace Bridge.Editor.Ipc
{
    public class FeatureGuard
    {
        private readonly HashSet<FeatureFlag> negotiatedFeatures;
        
        public FeatureGuard(IEnumerable<string> negotiatedFeatureStrings)
        {
            negotiatedFeatures = negotiatedFeatureStrings
                .Select(FeatureFlagExtensions.FromString)
                .Where(f => f != FeatureFlag.Unknown)
                .ToHashSet();
        }
        
        public bool IsFeatureEnabled(FeatureFlag feature)
        {
            return negotiatedFeatures.Contains(feature);
        }
        
        public void RequireFeature(FeatureFlag feature)
        {
            if (!IsFeatureEnabled(feature))
            {
                throw new InvalidOperationException($"Feature {feature.ToString()} not negotiated");
            }
        }
        
        public HashSet<FeatureFlag> GetNegotiatedFeatures()
        {
            return new HashSet<FeatureFlag>(negotiatedFeatures);
        }
        
        public List<string> GetNegotiatedFeatureStrings()
        {
            return negotiatedFeatures.Select(f => FeatureFlagExtensions.ToWireString(f)).ToList();
        }
    }
}
