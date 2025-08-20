# Step 4 — First Real Tool over IPC: **Assets** (Import / Move / Delete / Refresh / GUID↔Path)

**Objective:** Replace the gRPC-based Assets tool with a direct IPC implementation. Use the envelope protocol from Steps 1–3 for request/response and `OperationEvent` for progress on long‑running tasks (e.g., folder imports). After this step, Rust tools can drive Unity’s `AssetDatabase` via IPC.

> Prereqs: Step 0 (message‑only Protobuf), Step 1 (Rust `IpcClient`), Step 2 (Unity `EditorIpcServer`), Step 3 (Events: Logs/Operations & reconnect). This step extends both sides with typed Assets handlers.

---

## 0) Protocol — messages & shape
If not already present in `assets.proto`, add/confirm these messages and regenerate for Rust/C#.

```proto
// assets.proto (excerpt)
message ImportAssetRequest { repeated string paths = 1; bool recursive = 2; bool auto_refresh = 3; }
message ImportAssetResult { string path = 1; string guid = 2; bool ok = 3; string message = 4; }
message ImportAssetResponse { repeated ImportAssetResult results = 1; }

message MoveAssetRequest { string from_path = 1; string to_path = 2; }
message MoveAssetResponse { bool ok = 1; string message = 2; string new_guid = 3; }

message DeleteAssetRequest { repeated string paths = 1; bool soft = 2; }
message DeleteAssetResponse { repeated string deleted = 1; repeated string failed = 2; }

message RefreshRequest { bool force = 1; }
message RefreshResponse { bool ok = 1; }

message GuidToPathRequest { repeated string guids = 1; }
message GuidToPathResponse { map<string,string> map = 1; }

message PathToGuidRequest { repeated string paths = 1; }
message PathToGuidResponse { map<string,string> map = 1; }

message AssetsRequest {
  oneof payload {
    ImportAssetRequest  import  = 1;
    MoveAssetRequest    move    = 2;
    DeleteAssetRequest  delete  = 3;
    RefreshRequest      refresh = 4;
    GuidToPathRequest   g2p     = 5;
    PathToGuidRequest   p2g     = 6;
  }
}

message AssetsResponse {
  int32 status_code = 1; // 0=OK, 2=INVALID_ARGUMENT, 5=NOT_FOUND, 13=INTERNAL
  string message     = 2;
  oneof payload {
    ImportAssetResponse import  = 10;
    MoveAssetResponse   move    = 11;
    DeleteAssetResponse delete  = 12;
    RefreshResponse     refresh = 13;
    GuidToPathResponse  g2p     = 14;
    PathToGuidResponse  p2g     = 15;
  }
}
```

Envelope mapping:
- `IpcRequest.payload = AssetsRequest`
- `IpcResponse.payload = AssetsResponse`
- Long tasks: emit `OperationEvent{op_id, START/PROGRESS/COMPLETE}` concurrently.

**Path rules:**
- All paths must be Unity‑relative under the project (e.g., `Assets/…`). Reject absolute/parent‑escaping paths.

---

## 1) Unity — Assets handler (Editor‑only)
Add a new dispatcher in the `EditorIpcServer` request loop.

```csharp
// Editor/Ipc/AssetsHandler.cs
using UnityEditor;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;
using System.Collections.Generic;

internal static class AssetsHandler
{
    // Validate Unity-relative project path
    private static bool IsValidUnityPath(string p)
        => !string.IsNullOrEmpty(p) && !p.StartsWith("..") && !System.IO.Path.IsPathRooted(p) && p.StartsWith("Assets/");

    public static Pb.AssetsResponse Handle(Pb.AssetsRequest req)
    {
        switch (req.PayloadCase)
        {
            case Pb.AssetsRequest.PayloadOneofCase.Import:  return Import(req.Import);
            case Pb.AssetsRequest.PayloadOneofCase.Move:    return Move(req.Move);
            case Pb.AssetsRequest.PayloadOneofCase.Delete:  return Delete(req.Delete);
            case Pb.AssetsRequest.PayloadOneofCase.Refresh: return Refresh(req.Refresh);
            case Pb.AssetsRequest.PayloadOneofCase.G2p:     return G2P(req.G2p);
            case Pb.AssetsRequest.PayloadOneofCase.P2g:     return P2G(req.P2g);
            default: return new Pb.AssetsResponse { StatusCode = 2, Message = "invalid request" };
        }
    }

    private static Pb.AssetsResponse Import(Pb.ImportAssetRequest r)
    {
        // Long operation: track progress via OperationTracker
        string op = OperationTracker.Start("Import", $"Import {r.Paths.Count} items");
        try {
            var results = new List<Pb.ImportAssetResult>(r.Paths.Count);
            int done = 0;
            foreach (var p in r.Paths)
            {
                if (!IsValidUnityPath(p)) { results.Add(new Pb.ImportAssetResult{ path=p, ok=false, message="invalid path"}); continue; }
                string guidBefore = AssetDatabase.AssetPathToGUID(p);
                try {
                    if (r.Recursive)
                        AssetDatabase.ImportAsset(p, ImportAssetOptions.ImportRecursive);
                    else
                        AssetDatabase.ImportAsset(p);
                    if (r.AutoRefresh) AssetDatabase.Refresh();
                    string guid = AssetDatabase.AssetPathToGUID(p);
                    results.Add(new Pb.ImportAssetResult{ path=p, guid=guid, ok=true});
                } catch (Exception ex) {
                    results.Add(new Pb.ImportAssetResult{ path=p, ok=false, message=ex.Message});
                }
                done++;
                OperationTracker.Progress(op, (int)(100.0 * done / Math.Max(1, r.Paths.Count)));
            }
            OperationTracker.Complete(op, 0, "OK");
            return new Pb.AssetsResponse { StatusCode = 0, Import = new Pb.ImportAssetResponse { Results = { results } } };
        } catch (Exception ex) {
            OperationTracker.Complete(op, 13, ex.Message);
            return new Pb.AssetsResponse { StatusCode = 13, Message = ex.Message };
        }
    }

    private static Pb.AssetsResponse Move(Pb.MoveAssetRequest r)
    {
        if (!IsValidUnityPath(r.FromPath) || !IsValidUnityPath(r.ToPath))
            return new Pb.AssetsResponse { StatusCode = 2, Message = "invalid path" };
        string error = AssetDatabase.MoveAsset(r.FromPath, r.ToPath);
        if (!string.IsNullOrEmpty(error))
            return new Pb.AssetsResponse { StatusCode = 13, Message = error };
        return new Pb.AssetsResponse { StatusCode = 0, Move = new Pb.MoveAssetResponse { Ok = true, NewGuid = AssetDatabase.AssetPathToGUID(r.ToPath) } };
    }

    private static Pb.AssetsResponse Delete(Pb.DeleteAssetRequest r)
    {
        var deleted = new List<string>();
        var failed = new List<string>();
        foreach (var p in r.Paths)
        {
            if (!IsValidUnityPath(p)) { failed.Add(p); continue; }
            bool ok = r.Soft ? AssetDatabase.MoveAssetToTrash(p) : AssetDatabase.DeleteAsset(p);
            (ok ? deleted : failed).Add(p);
        }
        return new Pb.AssetsResponse { StatusCode = 0, Delete = new Pb.DeleteAssetResponse { Deleted = { deleted }, Failed = { failed } } };
    }

    private static Pb.AssetsResponse Refresh(Pb.RefreshRequest r)
    {
        AssetDatabase.Refresh(r.Force ? ImportAssetOptions.ForceUpdate : ImportAssetOptions.Default);
        return new Pb.AssetsResponse { StatusCode = 0, Refresh = new Pb.RefreshResponse { Ok = true } };
    }

    private static Pb.AssetsResponse G2P(Pb.GuidToPathRequest r)
    {
        var map = new Pb.GuidToPathResponse();
        foreach (var g in r.Guids)
            map.Map[g] = AssetDatabase.GUIDToAssetPath(g);
        return new Pb.AssetsResponse { StatusCode = 0, G2p = map };
    }

    private static Pb.AssetsResponse P2G(Pb.PathToGuidRequest r)
    {
        var map = new Pb.PathToGuidResponse();
        foreach (var p in r.Paths)
            map.Map[p] = AssetDatabase.AssetPathToGUID(p);
        return new Pb.AssetsResponse { StatusCode = 0, P2g = map };
    }
}
```

**Wire into `EditorIpcServer`**:

```csharp
// in EditorIpcServer.DispatchAsync(...)
case Pb.IpcRequest.PayloadOneofCase.Assets:
{
    var res = AssetsHandler.Handle(req.Assets);
    await SendResponseAsync(s, cid, res.StatusCode, res.Message, new Pb.IpcResponse { Assets = res });
    break;
}
```

> Threading: `AssetDatabase` must run on the main thread. If your request loop is on a worker thread, marshal to main thread via `EditorApplication.update` or a dedicated dispatcher.

---

## 2) Rust — typed client helpers & tool glue
Add Assets convenience methods on `IpcClient` and use them in your MCP tool.

```rust
// server/src/ipc/client.rs (excerpt)
use crate::generated::mcp::unity::v1 as pb;
use std::time::Duration;

impl IpcClient {
    pub async fn assets_import(&self, paths: Vec<String>, recursive: bool, auto_refresh: bool, timeout: Duration)
        -> Result<pb::ImportAssetResponse, IpcError>
    {
        let req = pb::IpcRequest { payload: Some(pb::ipc_request::Payload::Assets(pb::AssetsRequest{
            payload: Some(pb::assets_request::Payload::Import(pb::ImportAssetRequest{ paths, recursive, auto_refresh }))
        }))};
        let resp = self.request(req, timeout).await?;
        match resp.payload { Some(pb::ipc_response::Payload::Assets(pb::AssetsResponse{ status_code: 0, payload: Some(pb::assets_response::Payload::Import(r)), ..})) => Ok(r),
            Some(pb::ipc_response::Payload::Assets(res)) => Err(IpcError::Handshake(format!("assets import failed: {}", res.message))),
            _ => Err(IpcError::Handshake("unexpected response".into())) }
    }
}
```

Hook into the MCP tool handler (example):

```rust
// server/src/mcp/tools/assets.rs (new)
use crate::{ipc::client::IpcClient, generated::mcp::unity::v1 as pb};
use anyhow::Result;
use std::time::Duration;

pub struct AssetsTool { pub ipc: IpcClient }

impl AssetsTool {
    pub async fn import(&self, paths: Vec<String>, recursive: bool) -> Result<pb::ImportAssetResponse> {
        self.ipc.assets_import(paths, recursive, true, Duration::from_secs(10)).await.map_err(Into::into)
    }
}
```

Expose via your tool router as you did for Health.

---

## 3) Error mapping & validation
- Use `status_code` aligned with gRPC semantics for familiarity:
  - `0=OK`, `2=INVALID_ARGUMENT`, `5=NOT_FOUND`, `13=INTERNAL`.
- For per‑item operations (Import/Delete), return a list of successes/failures even when `status_code=0` at the envelope level.
- Validate **Unity‑relative path** and reject invalid inputs with `INVALID_ARGUMENT`.
- Never operate outside the project; block absolute paths and `..` segments.

---

## 4) Security & robustness
- Restrict server (Unity) to per‑user endpoints; do not accept remote TCP by accident.
- Rate‑limit INFO logs (Step 3). Always forward WARN/ERROR and operation events.
- Ensure `OperationTracker` is used for long tasks and always completes (`Complete(op, code, msg)`) in `finally`‑equivalent paths.

---

## 5) Testing

### 5.1 Unit (C#)
- Path validator accepts `Assets/...` and rejects absolute/parent‑escaping paths.
- Move: error when target already exists; success path returns new GUID.
- Import: fake/import small assets under `Assets/Tests/`.

### 5.2 Integration (Rust↔Unity)
- Import a folder with N files; assert: `N` results, op progress reaches 100, and final `status_code=0`.
- Move and Delete on temp assets; verify GUID changes and that deleted paths vanish.
- Refresh with `force=true` causes a reimport (measure by timestamp/hash if available).

### 5.3 Chaos
- Disconnect Unity mid‑import; verify Rust fails the request, reconnects, and continues receiving later events.

---

## 6) Definition of Done (Step 4)
- Unity handles `AssetsRequest` variants and returns `AssetsResponse` with appropriate `status_code` and per‑item results.
- Long operations emit `OperationEvent` progress updates; Rust receives them without blocking requests.
- MCP tool in Rust can call at least **Import** and **Move** end‑to‑end.
- All code paths run on Unity main thread when touching `AssetDatabase`.

---

## 7) What’s next (Step 5 preview)
- Convert **Build** tool to IPC (build player, build asset bundles) with operation tracking.
- Add a small Rust CLI to exercise Assets commands and tail events (`--tail-logs`).

