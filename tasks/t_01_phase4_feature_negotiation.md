# T01 Phase4: Feature Negotiation実装

**Status:** Ready for implementation  
**Priority:** Medium  
**Estimated effort:** 2-3 hours  
**Depends on:** Phase3 (Error Handling)

## 目標と成果物

T01仕様のfeature negotiation mechanismを実装し、クライアントとサーバー間でサポートされる機能セットを動的に決定できるようにする。

### 成果物
- [ ] Feature flag定義と管理システム
- [ ] クライアント側feature proposal logic
- [ ] サーバー側feature intersection logic
- [ ] Feature-based conditional execution
- [ ] Feature compatibility testing

## 前提条件

- Phase3完了（Error handling動作確認済み）
- T01仕様のSection 6 (Feature Flags)理解
- 既存機能とfeature flagsのmapping理解

## Feature Flag定義

### 初期サポート対象Features (T01 Section 6)

```rust
// server/src/ipc/features.rs - 新規作成
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
    // Asset management
    AssetsBasic,     // "assets.basic" - path↔guid, import, refresh
    
    // Build system
    BuildMin,        // "build.min" - minimal player build with operation events
    
    // Events and logging
    EventsLog,       // "events.log" - Unity log events
    
    // Operations
    OpsProgress,     // "ops.progress" - generic progress events for long-running operations
    
    // Future extensions
    AssetsAdvanced,  // "assets.advanced" - asset streaming, dependencies
    BuildFull,       // "build.full" - full build pipeline with addressables
    EventsFull,      // "events.full" - detailed Unity events (scene, play mode, etc.)
    
    // Unknown/unsupported
    Unknown(String),
}

impl FeatureFlag {
    pub fn from_string(s: &str) -> Self {
        match s {
            "assets.basic" => Self::AssetsBasic,
            "build.min" => Self::BuildMin,
            "events.log" => Self::EventsLog,
            "ops.progress" => Self::OpsProgress,
            "assets.advanced" => Self::AssetsAdvanced,
            "build.full" => Self::BuildFull,
            "events.full" => Self::EventsFull,
            _ => Self::Unknown(s.to_string()),
        }
    }
    
    pub fn to_string(&self) -> String {
        match self {
            Self::AssetsBasic => "assets.basic".to_string(),
            Self::BuildMin => "build.min".to_string(),
            Self::EventsLog => "events.log".to_string(),
            Self::OpsProgress => "ops.progress".to_string(),
            Self::AssetsAdvanced => "assets.advanced".to_string(),
            Self::BuildFull => "build.full".to_string(),
            Self::EventsFull => "events.full".to_string(),
            Self::Unknown(s) => s.clone(),
        }
    }
    
    pub fn is_supported_by_client() -> Vec<Self> {
        vec![
            Self::AssetsBasic,
            Self::BuildMin,
            Self::EventsLog,
            Self::OpsProgress,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct FeatureSet {
    features: std::collections::HashSet<FeatureFlag>,
}

impl FeatureSet {
    pub fn new() -> Self {
        Self {
            features: std::collections::HashSet::new(),
        }
    }
    
    pub fn from_strings(strings: &[String]) -> Self {
        let features = strings
            .iter()
            .map(|s| FeatureFlag::from_string(s))
            .collect();
        Self { features }
    }
    
    pub fn to_strings(&self) -> Vec<String> {
        self.features
            .iter()
            .map(|f| f.to_string())
            .collect()
    }
    
    pub fn intersect(&self, other: &Self) -> Self {
        let features = self.features
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
        let features = FeatureFlag::is_supported_by_client()
            .into_iter()
            .collect();
        Self { features }
    }
}
```

## 実装手順

### Step 1: Rust Client - Feature Proposal

`client.rs`のhandshake送信部分を更新：

```rust
// IpcClient内にnegotiated featuresを保存
#[derive(Debug)]
struct Inner {
    cfg: IpcConfig,
    corr: AtomicU64,
    pending: Mutex<HashMap<String, oneshot::Sender<pb::IpcResponse>>>,
    events_tx: broadcast::Sender<pb::IpcEvent>,
    tx: mpsc::Sender<Bytes>,
    negotiated_features: Mutex<FeatureSet>, // 新規追加
}

impl IpcClient {
    async fn spawn_io(/* ... */) -> Result<(), IpcError> {
        // ... existing connection code ...
        
        // Send IpcControl(hello) with client's desired features
        let desired_features = FeatureSet::supported_by_client();
        let hello = pb::IpcHello {
            token: inner.cfg.token.clone().unwrap_or_default(),
            ipc_version: "1.0".to_string(),
            features: desired_features.to_strings(),
            schema_hash: codec::schema_hash().into_bytes(),
            project_root: inner.cfg.project_root.clone().unwrap_or_default(),
            client_name: "unity-mcp-rs".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            meta: create_default_meta(),
        };
        
        // ... send hello and receive welcome ...
        
        // Process negotiated features
        let negotiated = FeatureSet::from_strings(&welcome.accepted_features);
        {
            let mut features = inner.negotiated_features.lock().await;
            *features = negotiated.clone();
        }
        
        tracing::info!(
            "Handshake OK: version={}, features={:?}, session={}",
            welcome.ipc_version,
            negotiated.to_strings(),
            welcome.session_id
        );
        
        // ... continue with normal processing ...
    }
    
    pub async fn has_feature(&self, feature: FeatureFlag) -> bool {
        let features = self.inner.negotiated_features.lock().await;
        features.contains(&feature)
    }
    
    pub async fn get_negotiated_features(&self) -> FeatureSet {
        self.inner.negotiated_features.lock().await.clone()
    }
}
```

### Step 2: Unity Server - Feature Intersection

Unity側でのfeature negotiation logic：

```csharp
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
        return StringToFlag.TryGetValue(str, out var flag) ? flag : FeatureFlag.Unknown;
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
}

// EditorIpcServer.cs内のfeature negotiation
private IpcWelcome CreateWelcome(IpcHello hello)
{
    var clientFeatures = hello.Features
        .Select(FeatureFlagExtensions.FromString)
        .Where(f => f != FeatureFlag.Unknown)
        .ToHashSet();
    
    var serverFeatures = FeatureFlagExtensions.GetServerSupportedFeatures();
    
    // Intersection - only features supported by both sides
    var acceptedFeatures = clientFeatures
        .Intersect(serverFeatures)
        .Select(f => f.ToString())
        .ToList();
    
    Debug.Log($"Feature negotiation: client requested {clientFeatures.Count}, " +
              $"server supports {serverFeatures.Count}, accepted {acceptedFeatures.Count}");
    
    return new IpcWelcome
    {
        IpcVersion = hello.IpcVersion,
        AcceptedFeatures = { acceptedFeatures },
        SchemaHash = hello.SchemaHash, // Will be implemented in Phase 5
        ServerName = "unity-editor-bridge",
        ServerVersion = GetPackageVersion(),
        EditorVersion = Application.unityVersion,
        SessionId = Guid.NewGuid().ToString(),
        Meta = { { "platform", Application.platform.ToString() } }
    };
}
```

### Step 3: Feature-Based Conditional Logic

クライアント側でのfeature-dependent behavior：

```rust
impl IpcClient {
    pub async fn assets_import(
        &self,
        paths: Vec<String>,
        recursive: bool,
        auto_refresh: bool,
        timeout: Duration,
    ) -> Result<pb::ImportAssetResponse, IpcError> {
        // Check if assets.basic feature is negotiated
        if !self.has_feature(FeatureFlag::AssetsBasic).await {
            return Err(IpcError::Handshake(
                "assets.basic feature not available".into()
            ));
        }
        
        // Proceed with existing implementation
        let req = pb::IpcRequest {
            payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest {
                payload: Some(pb::assets_request::Payload::Import(pb::ImportAssetRequest {
                    paths,
                    recursive,
                    auto_refresh,
                })),
            })),
        };
        
        // ... rest of implementation ...
    }
    
    pub async fn build_player(
        &self, 
        req: pb::BuildPlayerRequest, 
        timeout: Duration
    ) -> Result<pb::BuildPlayerResponse, IpcError> {
        if !self.has_feature(FeatureFlag::BuildMin).await {
            return Err(IpcError::Handshake(
                "build.min feature not available".into()
            ));
        }
        
        // ... rest of implementation ...
    }
}
```

### Step 4: Unity側のFeature Guard

Unity側でのfeature-based request validation：

```csharp
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
}

// Request handling with feature guards
private async Task<IpcResponse> HandleAssetsRequest(AssetsRequest request, FeatureGuard features)
{
    features.RequireFeature(FeatureFlag.AssetsBasic);
    
    // ... proceed with assets request handling ...
}

private async Task<IpcResponse> HandleBuildRequest(BuildRequest request, FeatureGuard features)
{
    features.RequireFeature(FeatureFlag.BuildMin);
    
    // ... proceed with build request handling ...
}
```

### Step 5: Configuration-Based Feature Control

サーバー側でのfeature enablement configuration：

```csharp
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
        
        return features;
    }
    
    private static bool IsBuildSystemAvailable()
    {
        // Check if build system is properly configured
        return !EditorApplication.isPlayingOrWillChangePlaymode;
    }
}
```

## テスト要件

### Feature Negotiation Tests

```rust
#[tokio::test]
async fn test_feature_negotiation_intersection() {
    let server = MockUnityServer::new()
        .with_supported_features(vec!["assets.basic", "events.log"]);
    
    let client = IpcClient::connect(test_config(&server)).await.unwrap();
    
    let features = client.get_negotiated_features().await;
    assert!(features.contains(&FeatureFlag::AssetsBasic));
    assert!(features.contains(&FeatureFlag::EventsLog));
    assert!(!features.contains(&FeatureFlag::BuildMin)); // Not supported by mock server
}

#[tokio::test]
async fn test_feature_dependent_operation_rejection() {
    let server = MockUnityServer::new()
        .with_supported_features(vec!["events.log"]); // No assets.basic
    
    let client = IpcClient::connect(test_config(&server)).await.unwrap();
    
    let result = client.assets_import(vec![], false, false, Duration::from_secs(1)).await;
    assert!(matches!(result, Err(IpcError::Handshake(_))));
}
```

## 期待される変更ファイル

- `server/src/ipc/features.rs` (新規 - feature flag definitions)
- `server/src/ipc/client.rs` (feature negotiation, conditional logic)
- `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/FeatureFlag.cs` (新規)
- `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/EditorIpcServer.cs` (feature intersection)
- `server/tests/ipc_integration.rs` (feature negotiation tests)

## Definition of Done

- [ ] クライアントが desired features を正しく提案
- [ ] サーバーが supported features との intersection を返す
- [ ] クライアントが negotiated features を保存・確認
- [ ] Feature-dependent operationsが適切に guard される
- [ ] Configuration-based feature enablement が動作
- [ ] Feature negotiation integration tests が pass
- [ ] Unknown features が gracefully handle される

## 次のフェーズへの引き継ぎ

Phase 5で必要となる要素：
- Schema hash calculation infrastructure
- Feature set impact on schema compatibility
- Performance considerations for feature checking

## パフォーマンス考慮事項

- Feature checkingはhot pathで実行されるため効率的な実装が必要
- Negotiated featuresのcachingとthread safety
- Feature flag lookupの最適化（HashMap vs HashSet vs bitflags）