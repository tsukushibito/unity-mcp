# Step 3 — Hosting Layer (GrpcHost / BridgeServer)

**Objective**
Stand up a minimal gRPC host inside the Unity Bridge package with start/stop, service registration, and Editor integration (menu + optional auto‑start). The host must log clear start/stop messages and be idempotent.

---

## Prerequisites
- Unity project is located under `bridge/`.
- Bridge is delivered as a UPM package: `bridge/Packages/com.example.mcp-bridge`.
- gRPC **C# server stubs** have been generated into:
  `bridge/Packages/com.example.mcp-bridge/Editor/Generated/Proto`.
- Required assemblies are present for Editor‑only use (e.g., `Google.Protobuf`, `Grpc.Core`).
- Rust side acts as a gRPC client; Unity side is the gRPC server.

---

## Directory Layout (Package)
```
bridge/
└─ Packages/
   └─ com.example.mcp-bridge/
      └─ Editor/
         ├─ BridgeServer.cs                     # Editor integration (menu, auto‑start)
         ├─ Services/
         │  ├─ GrpcHost.cs                      # gRPC server lifecycle manager
         │  └─ Impl/
         │     └─ EditorControlService.cs       # Minimal Health implementation for smoke test
         └─ Generated/Proto/                    # protoc‑generated messages + service bases
```

> If you use a generator script, keep it at: `bridge/Tools/generate_csharp.sh` and target the `Generated/Proto` folder above.

---

## Task A — Implement `GrpcHost`
**Path:** `bridge/Packages/com.example.mcp-bridge/Editor/Services/GrpcHost.cs`

**Responsibilities**
- Hold the `Grpc.Core.Server` instance.
- Register services (`AddService(ServerServiceDefinition)`).
- Add optional interceptors (`UseInterceptor(ServerInterceptor)`).
- Provide idempotent `Start()` / `Stop()` with thread‑safe guards.

**Suggested API**
```csharp
public sealed class GrpcHost
{
    public bool IsRunning { get; private set; }
    public int Port { get; }

    public GrpcHost(int port = 50061);
    public GrpcHost UseInterceptor(ServerInterceptor interceptor);
    public GrpcHost AddService(ServerServiceDefinition service);

    public void Start();   // no‑op if already started
    public void Stop();    // safe if already stopped
}
```

**Implementation Notes**
- Bind to loopback first: `127.0.0.1:Port` with `ServerCredentials.Insecure` (upgrade to TLS later).
- Maintain a private lock and short‑circuit duplicate starts/stops.
- Log messages (DoD):
  - Start: `[Bridge] Started on 127.0.0.1:<Port>`
  - Stop:  `[Bridge] Stopped`
  - Double start attempt: `[Bridge] Already running` (warning)
- Add interceptors for logging/auth later (see Task D).

---

## Task B — Minimal Service Implementation (Health)
**Path:** `bridge/Packages/com.example.mcp-bridge/Editor/Services/Impl/EditorControlService.cs`

**Goal**: Provide a working `Health` RPC for connectivity checks.

**Sketch**
```csharp
using Grpc.Core;
using mcp.unity.v1;

public sealed class EditorControlService : EditorControl.EditorControlBase
{
    public override Task<HealthResponse> Health(HealthRequest request, ServerCallContext context)
    {
        return Task.FromResult(new HealthResponse
        {
            Version = "bridge-0.1",
            Ready = true,
        });
    }
}
```

**Registration** (called before `Start()`):
```csharp
host.AddService(EditorControl.BindService(new EditorControlService()));
```

---

## Task C — Editor Integration (`BridgeServer`)
**Path:** `bridge/Packages/com.example.mcp-bridge/Editor/BridgeServer.cs`

**Responsibilities**
- Own a single static `GrpcHost` instance.
- Auto‑start on Editor load (opt‑in via `EditorPrefs`).
- Provide `Bridge/Start` and `Bridge/Stop` menu items.
- Ensure clean shutdown on Editor quit.

**Sketch**
```csharp
using UnityEditor;

[InitializeOnLoad]
public static class BridgeServer
{
    static GrpcHost _host;
    const int DefaultPort = 50061;
    const string AutoStartKey = "Bridge.AutoStart"; // default true

    static BridgeServer()
    {
        EditorApplication.quitting += OnQuitting;
        EditorApplication.delayCall += EnsureAutoStart;
    }

    static void EnsureHost()
    {
        if (_host != null) return;
        _host = new GrpcHost(DefaultPort)
            // .UseInterceptor(new AuthInterceptor(...))
            // .UseInterceptor(new LoggingInterceptor())
            ;
        _host.AddService(mcp.unity.v1.EditorControl.BindService(new EditorControlService()));
    }

    [MenuItem("Bridge/Start", priority = 10)]
    public static void StartMenu()
    {
        EnsureHost();
        _host.Start(); // idempotent
        Menu.SetChecked("Bridge/Start", _host.IsRunning);
    }

    [MenuItem("Bridge/Stop", priority = 11)]
    public static void StopMenu()
    {
        _host?.Stop();
        Menu.SetChecked("Bridge/Start", _host?.IsRunning == true);
    }

    static void EnsureAutoStart()
    {
        if (!EditorPrefs.HasKey(AutoStartKey)) EditorPrefs.SetBool(AutoStartKey, true);
        if (EditorPrefs.GetBool(AutoStartKey, true)) StartMenu();
    }

    static void OnQuitting() => _host?.Stop();
}
```

---

## Task D — (Optional) Interceptors
- **AuthInterceptor**: check a shared token (e.g., `x-bridge-token`) from `Metadata` and reject unauthenticated calls.
- **LoggingInterceptor**: log method name, duration, and status for each call.

Add with `host.UseInterceptor(new AuthInterceptor(...))` before `Start()`.

---

## Task E — Proto Codegen (C#)
**Script Path (recommended):** `bridge/Tools/generate_csharp.sh`

**Content (repo‑root independent)**
```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PROTO_ROOT="$REPO_ROOT/proto"
OUT="$REPO_ROOT/bridge/Packages/com.example.mcp-bridge/Editor/Generated/Proto"

mkdir -p "$OUT"

protoc-grpctools \
  -I"$PROTO_ROOT" \
  --csharp_out="$OUT" \
  --grpc_out="$OUT" \
  --plugin=protoc-gen-grpc=grpc_csharp_plugin \
  $(find "$PROTO_ROOT/mcp/unity/v1" -name '*.proto')

echo "[generate_csharp.sh] C# gRPC stubs generated into $OUT"
```

Run from anywhere:
```
./bridge/Tools/generate_csharp.sh
```

---

## Validation Checklist (DoD)
- [ ] Unity Console shows: `[Bridge] Started on 127.0.0.1:50061` when started.
- [ ] Stopping prints: `[Bridge] Stopped`.
- [ ] Calling Start twice does not spawn a second server and logs a warning or is ignored.
- [ ] `grpcurl -plaintext 127.0.0.1:50061 mcp.unity.v1.EditorControl/Health` returns `{ version: "bridge-0.1", ready: true }`.

---

## Troubleshooting
- **Port already in use**: choose a different port in `GrpcHost` or stop lingering processes.
- **Missing assemblies**: ensure `Google.Protobuf.dll` and `Grpc.Core.dll` are present in the Editor assembly load path.
- **Menu state not updating**: call `Menu.SetChecked` after each Start/Stop.
- **Not starting on Editor load**: confirm `EditorPrefs.GetBool("Bridge.AutoStart", true)` is true and `[InitializeOnLoad]` compiled in Editor.

---

## Next Steps
- Add Settings UI under `Bridge/Settings` to configure port and token.
- Implement `Events` server‑stream for logs; wire `Application.logMessageReceivedThreaded`.
- Add `Operations` tracking for long‑running actions (build/import) with progress updates.

