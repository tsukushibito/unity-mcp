use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
    // Asset management
    AssetsBasic,  // "assets.basic" - path↔guid, import, refresh
    PrefabsBasic, // "prefabs.basic" - prefab create/update/overrides

    // Build system
    BuildMin, // "build.min" - minimal player build with operation events

    // Events and logging
    EventsLog, // "events.log" - Unity log events

    // Operations
    OpsProgress, // "ops.progress" - generic progress events for long-running operations

    // Future extensions
    AssetsAdvanced, // "assets.advanced" - asset streaming, dependencies
    BuildFull,      // "build.full" - full build pipeline with addressables
    EventsFull,     // "events.full" - detailed Unity events (scene, play mode, etc.)

    // Unknown/unsupported
    Unknown(String),
}

impl FeatureFlag {
    pub fn from_string(s: &str) -> Self {
        match s {
            "assets.basic" => Self::AssetsBasic,
            "prefabs.basic" => Self::PrefabsBasic,
            "build.min" => Self::BuildMin,
            "events.log" => Self::EventsLog,
            "ops.progress" => Self::OpsProgress,
            "assets.advanced" => Self::AssetsAdvanced,
            "build.full" => Self::BuildFull,
            "events.full" => Self::EventsFull,
            _ => Self::Unknown(s.to_string()),
        }
    }

    pub fn is_supported_by_client() -> Vec<Self> {
        vec![
            Self::AssetsBasic,
            Self::PrefabsBasic,
            Self::BuildMin,
            Self::EventsLog,
            Self::OpsProgress,
        ]
    }

    /// Normalize feature string (lowercase, trim)
    pub fn normalize_string(s: &str) -> String {
        s.trim().to_lowercase()
    }
}

impl fmt::Display for FeatureFlag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AssetsBasic => "assets.basic",
            Self::PrefabsBasic => "prefabs.basic",
            Self::BuildMin => "build.min",
            Self::EventsLog => "events.log",
            Self::OpsProgress => "ops.progress",
            Self::AssetsAdvanced => "assets.advanced",
            Self::BuildFull => "build.full",
            Self::EventsFull => "events.full",
            Self::Unknown(s) => s,
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone)]
pub struct FeatureSet {
    features: HashSet<FeatureFlag>,
}

impl FeatureSet {
    pub fn new() -> Self {
        Self {
            features: HashSet::new(),
        }
    }

    pub fn from_strings(strings: &[String]) -> Self {
        let features = strings
            .iter()
            .map(|s| FeatureFlag::from_string(&FeatureFlag::normalize_string(s)))
            .filter(|f| !matches!(f, FeatureFlag::Unknown(_))) // Filter unknown features during negotiation
            .collect();
        Self { features }
    }

    pub fn to_strings(&self) -> Vec<String> {
        self.features.iter().map(|f| f.to_string()).collect()
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let features = self
            .features
            .intersection(&other.features)
            .cloned()
            .collect();
        Self { features }
    }

    pub fn contains(&self, feature: &FeatureFlag) -> bool {
        self.features.contains(feature)
    }

    pub fn insert(&mut self, feature: FeatureFlag) {
        self.features.insert(feature);
    }

    pub fn supported_by_client() -> Self {
        let features = FeatureFlag::is_supported_by_client().into_iter().collect();
        Self { features }
    }
}

impl Default for FeatureSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_flag_from_string() {
        assert_eq!(
            FeatureFlag::from_string("assets.basic"),
            FeatureFlag::AssetsBasic
        );
        assert_eq!(
            FeatureFlag::from_string("prefabs.basic"),
            FeatureFlag::PrefabsBasic
        );
        assert_eq!(FeatureFlag::from_string("build.min"), FeatureFlag::BuildMin);
        assert_eq!(
            FeatureFlag::from_string("events.log"),
            FeatureFlag::EventsLog
        );
        assert_eq!(
            FeatureFlag::from_string("ops.progress"),
            FeatureFlag::OpsProgress
        );

        match FeatureFlag::from_string("unknown.feature") {
            FeatureFlag::Unknown(s) => assert_eq!(s, "unknown.feature"),
            _ => panic!("Expected Unknown variant"),
        }
    }

    #[test]
    fn test_feature_flag_to_string() {
        assert_eq!(FeatureFlag::AssetsBasic.to_string(), "assets.basic");
        assert_eq!(FeatureFlag::PrefabsBasic.to_string(), "prefabs.basic");
        assert_eq!(FeatureFlag::BuildMin.to_string(), "build.min");
        assert_eq!(FeatureFlag::EventsLog.to_string(), "events.log");
        assert_eq!(FeatureFlag::OpsProgress.to_string(), "ops.progress");
    }

    #[test]
    fn test_feature_string_normalization() {
        let normalized = FeatureFlag::normalize_string(" Assets.Basic ");
        assert_eq!(normalized, "assets.basic");

        let feature = FeatureFlag::from_string(&normalized);
        assert_eq!(feature, FeatureFlag::AssetsBasic);
    }

    #[test]
    fn test_feature_set_from_strings() {
        let client_features = vec![
            "assets.basic".to_string(),
            "prefabs.basic".to_string(),
            "unknown.feature".to_string(),
        ];
        let feature_set = FeatureSet::from_strings(&client_features);

        // Unknown features should be filtered out during negotiation
        assert!(
            !feature_set
                .to_strings()
                .contains(&"unknown.feature".to_string())
        );
        assert!(feature_set.contains(&FeatureFlag::AssetsBasic));
        assert!(feature_set.contains(&FeatureFlag::PrefabsBasic));
    }

    #[test]
    fn test_feature_set_intersection() {
        let mut client_features = FeatureSet::new();
        client_features.insert(FeatureFlag::AssetsBasic);
        client_features.insert(FeatureFlag::PrefabsBasic);
        client_features.insert(FeatureFlag::BuildMin);
        client_features.insert(FeatureFlag::EventsLog);

        let mut server_features = FeatureSet::new();
        server_features.insert(FeatureFlag::AssetsBasic);
        server_features.insert(FeatureFlag::PrefabsBasic);
        server_features.insert(FeatureFlag::EventsLog);
        server_features.insert(FeatureFlag::OpsProgress);

        let negotiated = client_features.intersect(&server_features);
        assert!(negotiated.contains(&FeatureFlag::AssetsBasic));
        assert!(negotiated.contains(&FeatureFlag::PrefabsBasic));
        assert!(negotiated.contains(&FeatureFlag::EventsLog));
        assert!(!negotiated.contains(&FeatureFlag::BuildMin));
        assert!(!negotiated.contains(&FeatureFlag::OpsProgress));
    }

    #[test]
    fn test_supported_by_client() {
        let client_features = FeatureSet::supported_by_client();
        assert!(client_features.contains(&FeatureFlag::AssetsBasic));
        assert!(client_features.contains(&FeatureFlag::PrefabsBasic));
        assert!(client_features.contains(&FeatureFlag::BuildMin));
        assert!(client_features.contains(&FeatureFlag::EventsLog));
        assert!(client_features.contains(&FeatureFlag::OpsProgress));
        assert!(!client_features.contains(&FeatureFlag::AssetsAdvanced));
    }
}
