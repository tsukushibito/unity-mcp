# Step 2 — Unity-side **EditorIpcServer** (Handshake + Health over IPC)

**Objective:** Implement a minimal, production-worthy IPC server inside the Unity Editor that speaks the same framed Protobuf protocol as the Rust MCP server. After this step, Rust can connect, complete a handshake, and perform a `Health` round-trip via IPC.

> This step assumes Step 0 (message-only codegen) and Step 1 (Rust IpcClient) are complete, and that envelope/handshake messages exist in your `.proto` set (`IpcEnvelope`, `IpcRequest`, `IpcResponse`, `IpcEvent`, `IpcHello`, `IpcWelcome`).

---

## 0) Goals & Constraints

- **Transport**
  - Windows: **Named Pipe** (server in Unity)
  - Unix/macOS: Prefer **Unix Domain Socket** if your Unity .NET profile supports it; otherwise **loopback TCP** fallback for development
- **Framing**: 4‑byte big‑endian length prefix + Protobuf bytes (must match Rust `LengthDelimitedCodec::new()` defaults)
- **Contract**: Protobuf **messages only** (no gRPC runtime). Messages generated from the same `.proto` files as Rust.
- **MVP scope**: Handshake + `Health` request/response only. Event stream (Logs/Operations) will be wired in Step 3.

---

## 1) Unity project layout (UPM package inside `bridge/`)

```
bridge/
  Packages/
    com.example.mcp-bridge/
      package.json
      Editor/
        Generated/           # C# generated Protobuf message classes
        Plugins/Protobuf/    # Google.Protobuf.dll (Editor only)
        Ipc/
          EditorIpcServer.cs
          Framing.cs
          EnvelopeCodec.cs
          NamedPipeTransport.cs
          TcpTransport.cs         # Unix fallback if UDS not available
          // (Optional) UnixDomainSocketTransport.cs
```

**Assembly definition**: place an `asmdef` under `Editor/` to ensure everything is **Editor-only** and compiles with the correct .NET profile.

---

## 2) Generate C# Protobuf messages (messages-only)

Generate the same messages as Rust. Do **not** generate service stubs.

Example invocation from repo root (adjust paths):

```bash
protoc \
  -I=proto \
  --csharp_out=bridge/Packages/com.example.mcp-bridge/Editor/Generated \
  proto/mcp/unity/v1/common.proto \
  proto/mcp/unity/v1/editor_control.proto \
  proto/mcp/unity/v1/assets.proto \
  proto/mcp/unity/v1/build.proto \
  proto/mcp/unity/v1/operations.proto \
  proto/mcp/unity/v1/events.proto
```

> Optionally add `--csharp_opt=base_namespace=Mcp.Unity.V1` to force a predictable namespace.

Add **Google.Protobuf.dll** to `Editor/Plugins/Protobuf/` and mark it Editor-only in the Plugin Inspector.

---

## 3) Framing (length‑delimited)

Implement big‑endian 4‑byte length prefix to mirror Rust defaults.

```csharp
// Editor/Ipc/Framing.cs
using System;
using System.Buffers;
using System.IO;
using System.Threading.Tasks;

internal static class Framing
{
    public static async Task WriteFrameAsync(Stream s, ReadOnlyMemory<byte> payload)
    {
        Span<byte> len = stackalloc byte[4];
        var n = payload.Length;
        len[0] = (byte)((n >> 24) & 0xFF);
        len[1] = (byte)((n >> 16) & 0xFF);
        len[2] = (byte)((n >> 8) & 0xFF);
        len[3] = (byte)(n & 0xFF);
        await s.WriteAsync(len);
        await s.WriteAsync(payload);
        await s.FlushAsync();
    }

    public static async Task<byte[]?> ReadFrameAsync(Stream s)
    {
        byte[] header = new byte[4];
        int r = await s.ReadAsync(header, 0, 4);
        if (r == 0) return null; // closed
        if (r < 4) throw new IOException("short read on length header");
        int n = (header[0] << 24) | (header[1] << 16) | (header[2] << 8) | header[3];
        if (n < 0 || n > (64 * 1024 * 1024)) throw new IOException("frame too large");
        byte[] payload = ArrayPool<byte>.Shared.Rent(n);
        try {
            int off = 0;
            while (off < n) {
                int got = await s.ReadAsync(payload, off, n - off);
                if (got <= 0) throw new IOException("unexpected EOF");
                off += got;
            }
            var result = new byte[n];
            Buffer.BlockCopy(payload, 0, result, 0, n);
            return result;
        } finally { ArrayPool<byte>.Shared.Return(payload); }
    }
}
```

---

## 4) Envelope encoding/decoding helpers

```csharp
// Editor/Ipc/EnvelopeCodec.cs
using Google.Protobuf;
using Pb = Mcp.Unity.V1; // or your generated namespace

internal static class EnvelopeCodec
{
    public static byte[] Encode(Pb.IpcEnvelope env) => env.ToByteArray();
    public static Pb.IpcEnvelope Decode(byte[] bytes) => Pb.IpcEnvelope.Parser.ParseFrom(bytes);
}
```

---

## 5) Transports — server side

### 5.1 Named Pipe (Windows)

```csharp
// Editor/Ipc/NamedPipeTransport.cs
#if UNITY_EDITOR_WIN
using System.IO.Pipes;
using System.Threading.Tasks;

internal sealed class NamedPipeTransport
{
    private readonly string _name; // e.g. \\.\pipe\unity-mcp\default (Unity acts as server)
    public NamedPipeTransport(string name) { _name = name; }

    public async Task<Stream> AcceptAsync()
    {
        var server = new NamedPipeServerStream(_name, PipeDirection.InOut, 1, PipeTransmissionMode.Byte, PipeOptions.Asynchronous);
        await server.WaitForConnectionAsync();
        return server;
    }
}
#endif
```

### 5.2 TCP fallback (Unix/macOS when UDS not available)

```csharp
// Editor/Ipc/TcpTransport.cs
using System.Net;
using System.Net.Sockets;
using System.Threading.Tasks;

internal sealed class TcpTransport
{
    private readonly IPEndPoint _ep; // 127.0.0.1:7777 by default
    private readonly TcpListener _listener;
    public TcpTransport(IPEndPoint ep) { _ep = ep; _listener = new TcpListener(_ep); }
    public void Start() => _listener.Start();
    public void Stop() => _listener.Stop();
    public async Task<NetworkStream> AcceptAsync()
    {
        var client = await _listener.AcceptTcpClientAsync();
        return client.GetStream();
    }
}
```

> If your Unity/.NET profile supports Unix Domain Sockets (`AddressFamily.Unix` + `UnixDomainSocketEndPoint`), implement `UnixDomainSocketTransport` equivalently and prefer it over TCP.

---

## 6) EditorIpcServer — handshake + Health handler

```csharp
// Editor/Ipc/EditorIpcServer.cs
using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using Google.Protobuf;
using Pb = Mcp.Unity.V1;
using UnityEditor;

[InitializeOnLoad]
internal static class EditorIpcServer
{
    static EditorIpcServer()
    {
        // Autostart in Editor context
        _ = RunAsync();
    }

    private static CancellationTokenSource _cts;

    public static async Task RunAsync()
    {
        _cts?.Cancel();
        _cts = new CancellationTokenSource();
        var ct = _cts.Token;

#if UNITY_EDITOR_WIN
        var transport = new NamedPipeTransport(@"\\.\pipe\unity-mcp\default");
        while (!ct.IsCancellationRequested)
        {
            using var stream = await transport.AcceptAsync();
            await HandleConnection(stream, ct);
        }
#else
        var transport = new TcpTransport(new System.Net.IPEndPoint(System.Net.IPAddress.Loopback, 7777));
        transport.Start();
        try {
            while (!ct.IsCancellationRequested)
            {
                using var stream = await transport.AcceptAsync();
                await HandleConnection(stream, ct);
            }
        } finally { transport.Stop(); }
#endif
    }

    private static async Task HandleConnection(Stream s, CancellationToken ct)
    {
        // 1) Expect IpcHello wrapped in IpcRequest
        var first = await Framing.ReadFrameAsync(s); if (first == null) return;
        var env = EnvelopeCodec.Decode(first);
        if (env.KindCase != Pb.IpcEnvelope.KindOneofCase.Request) return;
        var req = env.Request;
        if (req.PayloadCase != Pb.IpcRequest.PayloadOneofCase.Hello)
        {
            await SendErrorAsync(s, env.CorrelationId, 400, "expected hello");
            return;
        }
        var hello = req.Hello;
        // TODO: validate token and schema_hash

        // 2) Reply Welcome
        await SendWelcomeAsync(s);

        // 3) Enter request loop
        while (!ct.IsCancellationRequested)
        {
            var frame = await Framing.ReadFrameAsync(s);
            if (frame == null) break;
            var e = EnvelopeCodec.Decode(frame);
            if (e.KindCase != Pb.IpcEnvelope.KindOneofCase.Request) continue;
            await DispatchAsync(s, e.CorrelationId, e.Request);
        }
    }

    private static async Task DispatchAsync(Stream s, string cid, Pb.IpcRequest req)
    {
        switch (req.PayloadCase)
        {
            case Pb.IpcRequest.PayloadOneofCase.Health:
                // Execute on main thread if touching Unity API
                var ready = true; // compute actual state if needed
                var version = Application.unityVersion;
                var health = new Pb.HealthResponse { Ready = ready, Version = version };
                await SendResponseAsync(s, cid, 0, "OK", new Pb.IpcResponse { Health = health });
                break;
            default:
                await SendErrorAsync(s, cid, 404, "unknown request");
                break;
        }
    }

    private static async Task SendWelcomeAsync(Stream s)
    {
        var welcome = new Pb.IpcWelcome { Ok = true, ServerInfo = "Unity Editor" };
        var resp = new Pb.IpcResponse { Welcome = welcome, CorrelationId = "" };
        var env = new Pb.IpcEnvelope { Response = resp };
        await Framing.WriteFrameAsync(s, EnvelopeCodec.Encode(env));
    }

    private static async Task SendResponseAsync(Stream s, string cid, int code, string msg, Pb.IpcResponse body)
    {
        body.StatusCode = code;
        body.Message = msg ?? string.Empty;
        body.CorrelationId = cid ?? string.Empty;
        var env = new Pb.IpcEnvelope { Response = body };
        await Framing.WriteFrameAsync(s, EnvelopeCodec.Encode(env));
    }

    private static Task SendErrorAsync(Stream s, string cid, int code, string msg)
        => SendResponseAsync(s, cid, code, msg, new Pb.IpcResponse());
}
```

> **Threading note**: If a handler must touch Unity API, marshal to the main thread using `EditorApplication.update` or a dedicated main‑thread dispatcher. The Health sample reads `Application.unityVersion` (safe).

---

## 7) Configuration

- Endpoint string (Pipe name / TCP port) may be set in a ScriptableObject or via `ProjectSettings`. For now, use the defaults shown above to match Rust `IpcConfig` defaults.
- Optional **token** for handshake: store in EditorPrefs for the current user; do not log it.

---

## 8) Testing (manual & automated)

1. Start Unity Editor with this p
