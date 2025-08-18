# Unity Bridge — README

This README documents the **Unity Bridge** that runs a local gRPC server **inside the Unity Editor**. It lives at the repository path: `bridge/` (this file at `bridge/README.md`).

> **Target**: Unity **6** (6000.x) or later — Editor-only gRPC hosting.

---

## 1) Overview

* The Unity Editor hosts a **local gRPC server** (the *Bridge*).
* The Rust process acts as the **gRPC client**.
* The Bridge exposes at minimum the following endpoints via the generated `EditorControl` service:

  * `Health` → `{ version, ready }`
  * `GetPlayMode` → `{ is_playing }`
  * `SetPlayMode` → toggles play/stop and returns an `Operation` (in-memory store)

---

## 2) Requirements

* Unity **6** (6000.x) or later.
* C# gRPC dependencies available to the Editor-only assembly:

  * `Grpc.Core` (C-core) server in the Editor process (recommended)
  * Matching native **libgrpc** for each OS, marked **Editor only** in Import Settings
* Protobuf-generated C# sources present under `Assets/Editor/Generated/Proto/` (vendored).

---

## 3) Endpoint & Binding

**Policy**: loopback-only (no external exposure).

* **Default endpoint**: `127.0.0.1:50061`
* **Bind address**: `127.0.0.1`

**Port collision check**

* Windows (PowerShell):

  ```powershell
  netstat -ano | findstr LISTENING | findstr 50061
  ```
* macOS/Linux:

  ```bash
  lsof -iTCP -sTCP:LISTEN -nP | grep 50061 || true
  ```

If the port is taken, choose another (e.g., `50062`) and keep the Rust client configuration in sync.

---

## 4) Authentication Token (optional by default)

* **Header name**: `x-bridge-token`
* **Unity-side storage**: Do **not** use environment variables. Store in a `ScriptableObject` (e.g., `BridgeSettings.asset`) or `EditorPrefs`.
* **Generate a token**

  * macOS/Linux:

    ```bash
    openssl rand -hex 16    # 32 hex chars
    ```
  * Windows (PowerShell):

    ```powershell
    -join (1..32 | ForEach-Object { '{0:X}' -f (Get-Random -Max 16) })
    ```
* **Rust client usage**: Pass as metadata header — `x-bridge-token: <value>` (CLI flag or config file). Using an env var **on the client** is acceptable for convenience.

---

## 5) Directory & Assembly Policy

```
bridge/
  Assets/
    Editor/
      Bridge/                       # Bridge sources (Editor-only)
        BridgeServer.cs
        Hosting/GrpcHost.cs
        Hosting/TokenAuthInterceptor.cs
        Services/EditorControlService.cs
        Services/OperationsService.cs        # (may be minimal)
        Events/EventsService.cs              # (optional)
        Operations/Operation.cs
        Operations/OperationStore.cs
        Abstractions/IUnityEditorFacade.cs
        Abstractions/UnityEditorFacade.cs
        Abstractions/IVersionProvider.cs
        Abstractions/SystemClock.cs
      Generated/Proto/               # C# from .proto (vendored)
      Bridge.asmdef                  # Editor-only assembly definition
  Packages/
  ProjectSettings/
```

* Keep all Bridge code in an **Editor-only** assembly (`Bridge.asmdef`).
* Vendor generated C# under `Assets/Editor/Generated/Proto/` and commit them.

---

## 6) Protobuf → C# Codegen (one-shot vendor)

Generate outside Unity and vendor the results:

```bash
# From repo root
PROTO_ROOT=proto
OUT=bridge/Assets/Editor/Generated/Proto
mkdir -p "$OUT"

protoc \
  -I"$PROTO_ROOT" \
  --csharp_out="$OUT" \
  --grpc_out="$OUT" \
  --plugin=protoc-gen-grpc=/path/to/grpc_csharp_plugin \
  $(find "$PROTO_ROOT/mcp/unity/v1" -name '*.proto')
```

Re-run when `.proto` changes. **Commit** generated sources so Unity can compile without MSBuild.

---

## 7) Hosting & Startup

* \`\` wraps the gRPC server lifecycle:

  * `Start()` binds to `127.0.0.1:<Port>`
  * `AddService(ServerServiceDefinition)` registers services
  * `UseInterceptor(TokenAuthInterceptor)` when a token is configured
* \`\` orchestrates startup/shutdown:

  * Start/Stop from a menu (e.g., *Bridge ▸ Start/Stop*) or auto-start on Editor load
  * Idempotent stop; clear logs and dispose resources
* **Expected Unity logs**

  * Start: `[Bridge] Started on 127.0.0.1:50061`
  * Stop:  `[Bridge] Stopped`
  * Port in use: `[Bridge][Error] Port in use: 50061`

---

## 8) Rust Client — Minimal Calls (examples)

Make sure host/port match §3.

```bash
# Health
mcp-cli --host 127.0.0.1 --port 50061 health
# GetPlayMode
mcp-cli --host 127.0.0.1 --port 50061 get-playmode
# SetPlayMode
mcp-cli --host 127.0.0.1 --port 50061 set-playmode --play true
mcp-cli --host 127.0.0.1 --port 50061 set-playmode --play false
# If token is enabled, add:
--header "x-bridge-token: <YOUR_TOKEN_HERE>"
```

---

## 9) Logging & Troubleshooting

**Common issues**

* **Unauthorized** → Missing or wrong `x-bridge-token` while auth is enabled
* **Port collision** → Choose a different port; update both Bridge settings and Rust client
* **Native library load** → Ensure the correct libgrpc binary is present and marked Editor-only

**Rust-side guidance**

* On connection failures, log details and apply a bounded reconnect policy (document retry interval and maximum attempts).

---

## 10) Security Posture

* Local loopback only (`127.0.0.1`).
* If remote access is ever required, plan a dedicated iteration to enforce token auth, TLS, and certificate management.

---

## 11) Setup Checklist

*

---

## Appendix — CLI Snippets

```md
### Port Check
- Windows:
  netstat -ano | findstr LISTENING | findstr 50061
- macOS/Linux:
  lsof -iTCP -sTCP:LISTEN -nP | grep 50061

### Rust Client (smoke)
- Health:
  mcp-cli --host 127.0.0.1 --port 50061 health
- GetPlayMode:
  mcp-cli --host 127.0.0.1 --port 50061 get-playmode
- SetPlayMode:
  mcp-cli --host 127.0.0.1 --port 50061 set-playmode --play true

- (if token enabled) add:
  --header "x-bridge-token: <YOUR_TOKEN_HERE>"
```
