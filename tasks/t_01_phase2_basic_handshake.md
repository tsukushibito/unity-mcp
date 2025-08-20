# T01 Phase2: Basic Handshake実装

**Status:** Ready for implementation  
**Priority:** High  
**Estimated effort:** 3-4 hours  
**Depends on:** Phase1 (Proto定義)

## 目標と成果物

T01仕様のbasic handshakeフロー（hello → welcome）を実装し、最小限の成功パスを確立する。この段階ではerror handling、feature negotiation、schema validationは実装せず、happy pathのみに集中する。

### 成果物
- [ ] Rust側の基本handshake実装
- [ ] Unity側の基本handshake処理
- [ ] 成功パスのintegration test
- [ ] 基本的なlogging

## 前提条件

- Phase1完了（ipc_control.proto定義済み）
- 既存のIPC transport layer動作確認
- T01仕様のSection 3 (State Machine)理解

## 実装手順

### Step 1: Rust Client - Basic Handshake送信

`server/src/ipc/client.rs`の`spawn_io`関数を更新：

```rust
async fn spawn_io(
    inner: Arc<Inner>,
    endpoint: Endpoint,
    mut writer_rx: mpsc::Receiver<Bytes>,
) -> Result<(), IpcError> {
    // 1) connect
    let io = connect_endpoint(&endpoint, inner.cfg.connect_timeout).await?;
    let mut framed = framing::into_framed(io);

    // 2) T01 handshake - send IpcControl(hello)
    let hello = pb::IpcHello {
        token: inner.cfg.token.clone().unwrap_or_default(),
        ipc_version: "1.0".to_string(),
        features: vec![
            "assets.basic".to_string(),
            "events.log".to_string(),
            "build.min".to_string(),
            "ops.progress".to_string(),
        ],
        schema_hash: codec::schema_hash().into_bytes(),
        project_root: inner.cfg.project_root.clone().unwrap_or_default(),
        client_name: "unity-mcp-rs".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        meta: create_default_meta(),
    };
    
    let control = pb::IpcControl {
        kind: Some(pb::ipc_control::Kind::Hello(hello)),
    };
    
    let control_bytes = codec::encode_control(&control)?;
    framed.send(control_bytes).await.map_err(IpcError::Io)?;

    // 3) Read welcome/reject response
    let welcome = time::timeout(Duration::from_secs(2), async {
        while let Some(frame) = framed.next().await {
            let bytes = frame.map_err(IpcError::Io)?;
            let control = codec::decode_control(bytes.freeze())?;
            
            match control.kind {
                Some(pb::ipc_control::Kind::Welcome(w)) => return Ok(w),
                Some(pb::ipc_control::Kind::Reject(r)) => {
                    return Err(IpcError::Handshake(format!("{:?}: {}", r.code, r.message)));
                }
                _ => continue,
            }
        }
        Err(IpcError::Handshake("no response".into()))
    })
    .await
    .map_err(|_| IpcError::ConnectTimeout)??;

    tracing::info!(
        "Handshake OK: version={}, features={:?}, session={}",
        welcome.ipc_version,
        welcome.accepted_features,
        welcome.session_id
    );

    // 4) Continue with normal envelope processing...
    // (既存のコードを継続)
}

fn create_default_meta() -> std::collections::HashMap<String, String> {
    let mut meta = std::collections::HashMap::new();
    meta.insert("os".to_string(), std::env::consts::OS.to_string());
    meta.insert("arch".to_string(), std::env::consts::ARCH.to_string());
    meta
}
```

### Step 2: Codec拡張

`server/src/ipc/codec.rs`にcontrol message用の関数を追加：

```rust
pub fn encode_control(control: &pb::IpcControl) -> Result<Bytes, CodecError> {
    let mut buf = bytes::BytesMut::with_capacity(control.encoded_len());
    control.encode(&mut buf)?;
    Ok(buf.freeze())
}

pub fn decode_control(b: Bytes) -> Result<pb::IpcControl, CodecError> {
    pb::IpcControl::decode(b).map_err(CodecError::Decode)
}
```

### Step 3: Unity Server - Basic Handshake処理

`bridge/Packages/com.example.mcp-bridge/Editor/Ipc/EditorIpcServer.cs`にhandshake処理を追加：

```csharp
private async Task<bool> HandleHandshake(Stream stream)
{
    try
    {
        // Read control frame
        var controlBytes = await ReadFrameAsync(stream);
        var control = IpcControl.Parser.ParseFrom(controlBytes);
        
        if (control.Hello == null)
        {
            await SendReject(stream, IpcReject.Types.Code.Internal, "expected hello");
            return false;
        }

        var hello = control.Hello;
        
        // Basic validation (Phase 2 - minimal checks only)
        if (string.IsNullOrEmpty(hello.Token))
        {
            await SendReject(stream, IpcReject.Types.Code.Unauthenticated, "missing token");
            return false;
        }

        // TODO: Token validation (Phase 3)
        // TODO: Version validation (Phase 4)
        // TODO: Schema validation (Phase 5)

        // Send welcome
        var welcome = new IpcWelcome
        {
            IpcVersion = hello.IpcVersion, // Echo back for now
            AcceptedFeatures = { hello.Features }, // Accept all for now
            SchemaHash = hello.SchemaHash, // Echo back for now
            ServerName = "unity-editor-bridge",
            ServerVersion = "0.1.0", // TODO: Get from package
            EditorVersion = Application.unityVersion,
            SessionId = Guid.NewGuid().ToString(),
            Meta = { { "platform", Application.platform.ToString() } }
        };

        var welcomeControl = new IpcControl { Welcome = welcome };
        await SendControlFrame(stream, welcomeControl);
        
        Debug.Log($"Handshake successful: session={welcome.SessionId}");
        return true;
    }
    catch (Exception ex)
    {
        Debug.LogError($"Handshake failed: {ex.Message}");
        await SendReject(stream, IpcReject.Types.Code.Internal, "handshake error");
        return false;
    }
}

private async Task SendReject(Stream stream, IpcReject.Types.Code code, string message)
{
    var reject = new IpcReject { Code = code, Message = message };
    var control = new IpcControl { Reject = reject };
    await SendControlFrame(stream, control);
}

private async Task SendControlFrame(Stream stream, IpcControl control)
{
    var bytes = control.ToByteArray();
    await WriteFrameAsync(stream, bytes);
}
```

### Step 4: 既存IPC構造の調整

現在の`IpcHello`/`IpcWelcome`がIpcRequestに含まれている構造から、separate control messageへの移行：

- `ipc.proto`から古いhandshakeメッセージ定義を削除
- Transport layerでcontrol framesとregular envelopesを区別

### Step 5: Basic Integration Test

`server/tests/ipc_integration.rs`に基本handshakeテストを追加：

```rust
#[tokio::test]
async fn test_basic_handshake_success() {
    let server = start_mock_unity_server().await;
    let cfg = IpcConfig {
        endpoint: Some(server.endpoint()),
        token: Some("test-token".to_string()),
        ..Default::default()
    };
    
    let client = IpcClient::connect(cfg).await.expect("handshake should succeed");
    
    // Verify handshake completed
    let health = client.health(Duration::from_secs(1)).await.expect("health check should work");
    assert!(health.ok);
}
```

## テスト要件

### Unit Tests
- [ ] Handshakeメッセージのserialize/deserialize
- [ ] Basic validation logic
- [ ] Error message formatting

### Integration Tests
- [ ] Successful handshake flow
- [ ] Basic token rejection
- [ ] Timeout scenarios
- [ ] Connection after handshake

## 期待される変更ファイル

- `server/src/ipc/client.rs` (handshake logic)
- `server/src/ipc/codec.rs` (control message encoding)
- `bridge/Packages/com.example.mcp-bridge/Editor/Ipc/EditorIpcServer.cs` (handshake handling)
- `server/tests/ipc_integration.rs` (basic tests)

## Definition of Done

- [ ] RustクライアントがT01のbasic handshakeフローを送信
- [ ] UnityサーバーがIpcControlメッセージを処理してwelcome返信
- [ ] Successful handshakeでIPC sessionが確立
- [ ] Basic integration testがpass
- [ ] ログでhandshake successが確認できる
- [ ] 既存のhealth checkなどが正常動作

## Phase 2の制限事項

以下はPhase 3以降で実装：
- 詳細なtoken validation
- Version compatibility checking
- Schema hash validation
- Feature negotiation logic
- 詳細なerror handling

## 次のフェーズへの引き継ぎ

Phase 3で必要となる要素：
- Token validation mechanismの設計
- Error scenario handling
- Proper connection cleanup on handshake failure

## トラブルシューティング

よくある問題：
- Transport layer与existing envelope交互运用
- Control frameとregular envelopeの区別
- Unity C# Protocol Bufferコンパイル問題
- Timeoutとdeadlock回避