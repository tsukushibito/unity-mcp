# T01 Phase3: Error Handling実装

**Status:** Ready for implementation  
**Priority:** High  
**Estimated effort:** 2-3 hours  
**Depends on:** Phase2 (Basic Handshake)

## 目標と成果物

T01仕様のerror handling mechanismを実装し、様々な失敗シナリオに対する適切な`IpcReject`レスポンスとクライアント側のerror処理を確立する。

### 成果物
- [ ] 全ての`IpcReject.Code`に対応したUnity側validation
- [ ] Rust側の詳細なerror handling
- [ ] Token validation mechanism
- [ ] Version compatibility checking
- [ ] Comprehensive error testing

## 前提条件

- Phase2完了（Basic handshake動作確認済み）
- T01仕様のSection 7 (Error Mapping)理解
- 既存のIpcConfig構造理解

## 実装手順

### Step 1: Unity側 - 詳細なHandshake Validation

`EditorIpcServer.cs`のhandshake処理を拡張：

```csharp
private async Task<bool> HandleHandshake(Stream stream)
{
    try
    {
        var controlBytes = await ReadFrameAsync(stream);
        var control = IpcControl.Parser.ParseFrom(controlBytes);
        
        if (control.Hello == null)
        {
            await SendReject(stream, IpcReject.Types.Code.Internal, "expected hello");
            return false;
        }

        var hello = control.Hello;
        
        // 1. Token validation
        var tokenValidation = ValidateToken(hello.Token);
        if (!tokenValidation.IsValid)
        {
            await SendReject(stream, tokenValidation.ErrorCode, tokenValidation.ErrorMessage);
            return false;
        }

        // 2. Version compatibility check
        var versionValidation = ValidateVersion(hello.IpcVersion);
        if (!versionValidation.IsValid)
        {
            await SendReject(stream, versionValidation.ErrorCode, versionValidation.ErrorMessage);
            return false;
        }

        // 3. Editor state validation
        var editorValidation = ValidateEditorState();
        if (!editorValidation.IsValid)
        {
            await SendReject(stream, editorValidation.ErrorCode, editorValidation.ErrorMessage);
            return false;
        }

        // 4. Project root validation (basic PathPolicy check)
        var pathValidation = ValidateProjectRoot(hello.ProjectRoot);
        if (!pathValidation.IsValid)
        {
            await SendReject(stream, pathValidation.ErrorCode, pathValidation.ErrorMessage);
            return false;
        }

        // Success path - send welcome
        var welcome = CreateWelcome(hello);
        var welcomeControl = new IpcControl { Welcome = welcome };
        await SendControlFrame(stream, welcomeControl);
        
        Debug.Log($"Handshake successful: session={welcome.SessionId}, client={hello.ClientName}");
        return true;
    }
    catch (Exception ex)
    {
        Debug.LogError($"Handshake failed with exception: {ex.Message}");
        await SendReject(stream, IpcReject.Types.Code.Internal, "unexpected error");
        return false;
    }
}

private ValidationResult ValidateToken(string token)
{
    // Check if token is empty
    if (string.IsNullOrEmpty(token))
    {
        return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated, "missing token");
    }

    // Get expected token from configuration
    var expectedToken = GetConfiguredToken();
    if (string.IsNullOrEmpty(expectedToken))
    {
        // Development mode - accept any non-empty token
        return ValidationResult.Success();
    }

    // Production mode - exact match required
    if (token != expectedToken)
    {
        return ValidationResult.Error(IpcReject.Types.Code.Unauthenticated, "invalid token");
    }

    return ValidationResult.Success();
}

private ValidationResult ValidateVersion(string clientVersion)
{
    const string ServerVersion = "1.0"; // Current server version
    
    if (string.IsNullOrEmpty(clientVersion))
    {
        return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition, "missing ipc_version");
    }

    // Parse major.minor
    var clientParts = clientVersion.Split('.');
    var serverParts = ServerVersion.Split('.');
    
    if (clientParts.Length < 2 || serverParts.Length < 2)
    {
        return ValidationResult.Error(IpcReject.Types.Code.OutOfRange, "invalid version format");
    }

    if (!int.TryParse(clientParts[0], out int clientMajor) || 
        !int.TryParse(serverParts[0], out int serverMajor))
    {
        return ValidationResult.Error(IpcReject.Types.Code.OutOfRange, "invalid version numbers");
    }

    // Major version must match exactly
    if (clientMajor != serverMajor)
    {
        return ValidationResult.Error(
            IpcReject.Types.Code.OutOfRange, 
            $"ipc_version {clientVersion} not supported; server={ServerVersion}"
        );
    }

    return ValidationResult.Success();
}

private ValidationResult ValidateEditorState()
{
    // Check if Unity Editor is in a valid state
    if (EditorApplication.isCompiling)
    {
        return ValidationResult.Error(IpcReject.Types.Code.Unavailable, "editor compiling");
    }

    if (EditorApplication.isUpdating)
    {
        return ValidationResult.Error(IpcReject.Types.Code.Unavailable, "editor updating");
    }

    // Could add more checks for play mode, domain reload, etc.
    return ValidationResult.Success();
}

private ValidationResult ValidateProjectRoot(string projectRoot)
{
    if (string.IsNullOrEmpty(projectRoot))
    {
        return ValidationResult.Error(IpcReject.Types.Code.FailedPrecondition, "missing project_root");
    }

    try
    {
        var normalizedRoot = Path.GetFullPath(projectRoot);
        var actualProjectPath = Path.GetFullPath(Directory.GetCurrentDirectory());
        
        if (!normalizedRoot.Equals(actualProjectPath, StringComparison.OrdinalIgnoreCase))
        {
            return ValidationResult.Error(
                IpcReject.Types.Code.FailedPrecondition, 
                "project_root mismatch"
            );
        }
    }
    catch (Exception ex)
    {
        return ValidationResult.Error(
            IpcReject.Types.Code.FailedPrecondition, 
            "invalid project_root path"
        );
    }

    return ValidationResult.Success();
}

private string GetConfiguredToken()
{
    // Try environment variable first
    var envToken = Environment.GetEnvironmentVariable("MCP_IPC_TOKEN");
    if (!string.IsNullOrEmpty(envToken))
    {
        return envToken;
    }

    // Try EditorPrefs
    var prefKey = "MCP.IpcToken";
    if (EditorPrefs.HasKey(prefKey))
    {
        return EditorPrefs.GetString(prefKey);
    }

    // No token configured - development mode
    return null;
}

private struct ValidationResult
{
    public bool IsValid { get; }
    public IpcReject.Types.Code ErrorCode { get; }
    public string ErrorMessage { get; }

    private ValidationResult(bool isValid, IpcReject.Types.Code errorCode, string errorMessage)
    {
        IsValid = isValid;
        ErrorCode = errorCode;
        ErrorMessage = errorMessage;
    }

    public static ValidationResult Success() => new ValidationResult(true, default, null);
    public static ValidationResult Error(IpcReject.Types.Code code, string message) => 
        new ValidationResult(false, code, message);
}
```

### Step 2: Rust側 - Enhanced Error Processing

`client.rs`でのreject処理を改善：

```rust
// IpcError enumを拡張
#[derive(Debug, Error)]
pub enum IpcError {
    #[error("connect timeout")]
    ConnectTimeout,
    #[error("handshake failed: {0}")]
    Handshake(String),
    #[error("authentication failed: {0}")]
    Authentication(String),
    #[error("version incompatible: {0}")]
    VersionIncompatible(String),
    #[error("schema mismatch: {0}")]
    SchemaMismatch(String),
    #[error("server unavailable: {0}")]
    ServerUnavailable(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),           // 交渉されていない機能の使用時
    #[error("failed precondition: {0}")]
    FailedPrecondition(String),           // 一般的な前提条件不満
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("codec: {0}")]
    Codec(#[from] super::codec::CodecError),
    #[error("request timeout")]
    RequestTimeout,
    #[error("closed")]
    Closed,
}

// Handshake処理でのreject handling
async fn handle_handshake_response(
    control: pb::IpcControl,
) -> Result<pb::IpcWelcome, IpcError> {
    match control.kind {
        Some(pb::ipc_control::Kind::Welcome(w)) => Ok(w),
        Some(pb::ipc_control::Kind::Reject(r)) => {
            use pb::ipc_reject::Code;
            
            let error_msg = format!("{:?}: {}", r.code, r.message);
            
            match r.code() {
                Code::Unauthenticated => Err(IpcError::Authentication(r.message)),
                Code::OutOfRange => Err(IpcError::VersionIncompatible(r.message)),
                Code::FailedPrecondition => {
                    if r.message.contains("schema") {
                        Err(IpcError::SchemaMismatch(r.message))
                    } else {
                        Err(IpcError::Handshake(r.message))
                    }
                }
                Code::Unavailable => Err(IpcError::ServerUnavailable(r.message)),
                Code::PermissionDenied => Err(IpcError::PermissionDenied(r.message)),
                Code::Internal => Err(IpcError::Handshake(format!("server error: {}", r.message))),
            }
        }
        _ => Err(IpcError::Handshake("unexpected control response".into())),
    }
}
```

### Step 3: Reconnection Policy with Backoff

Handshake failureに基づくintelligentな再接続戦略：

```rust
impl IpcClient {
    async fn connect_with_retry(cfg: IpcConfig) -> Result<Self, IpcError> {
        let mut backoff_ms = 250u64;
        const MAX_BACKOFF_MS: u64 = 5000;
        const MAX_ATTEMPTS: u32 = 10;
        
        for attempt in 1..=MAX_ATTEMPTS {
            match Self::connect(cfg.clone()).await {
                Ok(client) => return Ok(client),
                Err(e) => {
                    let should_retry = match &e {
                        IpcError::ServerUnavailable(_) => true,
                        IpcError::ConnectTimeout => true,
                        IpcError::Io(_) => true,
                        IpcError::Authentication(_) => false, // Don't retry auth errors
                        IpcError::VersionIncompatible(_) => false, // Don't retry version errors
                        IpcError::SchemaMismatch(_) => false, // Don't retry schema errors
                        IpcError::UnsupportedFeature(_) => false, // Don't retry feature errors
                        _ => false,
                    };
                    
                    if !should_retry || attempt == MAX_ATTEMPTS {
                        return Err(e);
                    }
                    
                    tracing::warn!(
                        "Connection attempt {} failed: {}. Retrying in {}ms", 
                        attempt, e, backoff_ms
                    );
                    
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    
                    // Exponential backoff with jitter
                    let jitter = rand::random::<u64>() % (backoff_ms / 4);
                    backoff_ms = std::cmp::min(backoff_ms * 2, MAX_BACKOFF_MS) + jitter;
                }
            }
        }
        
        unreachable!()
    }
}
```

### Step 4: Configuration Management

Token and endpoint configurationの改善：

```rust
// IpcConfig拡張
#[derive(Clone, Debug)]
pub struct IpcConfig {
    pub endpoint: Option<String>,
    pub token: Option<String>,
    pub project_root: Option<String>,
    pub connect_timeout: Duration,
    pub handshake_timeout: Duration,
    pub max_reconnect_attempts: Option<u32>,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            token: std::env::var("MCP_IPC_TOKEN").ok(),
            project_root: std::env::current_dir()
                .ok()
                .and_then(|p| p.to_str().map(String::from)),
            connect_timeout: Duration::from_secs(2),
            handshake_timeout: Duration::from_secs(2),
            max_reconnect_attempts: Some(10),
        }
    }
}
```

## テスト要件

### Error Scenario Tests

```rust
#[tokio::test]
async fn test_handshake_authentication_failure() {
    let server = MockUnityServer::new()
        .with_token_validation(true)
        .with_expected_token("correct-token");
    
    let cfg = IpcConfig {
        token: Some("wrong-token".to_string()),
        ..test_config(&server)
    };
    
    let result = IpcClient::connect(cfg).await;
    assert!(matches!(result, Err(IpcError::Authentication(_))));
}

#[tokio::test]
async fn test_handshake_version_incompatible() {
    let server = MockUnityServer::new()
        .with_supported_version("1.0");
    
    let cfg = IpcConfig {
        // Force client to send version 2.0
        ..test_config(&server)
    };
    
    let result = IpcClient::connect(cfg).await;
    assert!(matches!(result, Err(IpcError::VersionIncompatible(_))));
}

#[tokio::test]
async fn test_handshake_server_unavailable() {
    let server = MockUnityServer::new()
        .with_editor_state(EditorState::Compiling);
    
    let result = IpcClient::connect(test_config(&server)).await;
    assert!(matches!(result, Err(IpcError::ServerUnavailable(_))));
}

#[tokio::test]
async fn test_post_handshake_unsupported_feature() {
    let server = MockUnityServer::new()
        .with_supported_features(vec!["events.log"]); // No assets.basic
    
    let client = IpcClient::connect(test_config(&server)).await.unwrap();
    
    // Try to use unsupported feature after successful handshake
    let result = client.assets_import(vec![], false, false, Duration::from_secs(1)).await;
    assert!(matches!(result, Err(IpcError::UnsupportedFeature(_))));
}
```

## 期待される変更ファイル

- `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/EditorIpcServer.cs` (詳細validation)
- `server/src/ipc/client.rs` (error handling, retry logic)
- `server/src/ipc/path.rs` (IpcConfig拡張)
- `server/tests/ipc_integration.rs` (error scenario tests)

## Definition of Done

- [ ] 全ての`IpcReject.Code`が適切なシナリオで生成される
- [ ] Rust側が各errorを適切にhandle、retry logic含む
- [ ] Token validation mechanismが動作（env var/EditorPrefs対応）
- [ ] Version compatibility checkingが動作
- [ ] Editor state validationが動作
- [ ] Error scenario integration testsが全てpass
- [ ] Retry logicがtransient errorsで動作、permanent errorsで停止

## 次のフェーズへの引き継ぎ

Phase 4で必要となる要素：
- Feature negotiation基盤（現在は全accept）
- Configuration systemの拡張
- Logging framework integration

## Error Messageガイドライン

**一文・端的ルール：**
- 全てのRejectメッセージは1文で終える
- 最大100文字以内で簡潔に
- 機密情報（token値、内部パスなど）を含めない
- アクション可能な場合は具体的な改善方法を示す

**例：**
- 良い：`"missing token"`
- 悪い：`"authentication failed because the provided token 'abc123' does not match the expected value"`
- 良い：`"project_root mismatch"`
- 悪い：`"project_root '/home/user/wrong/path' does not match server project '/home/user/correct/path'"`

**未交渉機能エラーの例：**
- クライアント側：`UnsupportedFeature("assets.basic feature not negotiated")`
- サーバー応答：`FAILED_PRECONDITION: "feature 'build.min' not negotiated"`
- Unity guard例外：`InvalidOperationException("Feature build.min not negotiated")`

## セキュリティ考慮事項

- Tokenをログに出力しない（デバッグレベルでも禁止）
- Error messageで機密情報や内部構造を漏らさない
- Path traversal防止のvalidation（正規化済みパスで比較）
- Rate limiting考慮（将来実装）