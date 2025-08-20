# Step 5 — Build Tool over IPC: **Build Player** & **Build Asset Bundles** (with progress & reports)

**Objective:** Port the gRPC-based Build tool to **direct IPC**. Unity executes Player/AssetBundle builds on the Editor main thread; Rust orchestrates requests and consumes `OperationEvent` progress + a concise `Build*Response` report.

> Prereqs: Step 0–4 done (messages-only, IpcClient, EditorIpcServer, Logs/Operations, Assets). This step introduces Build requests/responses, handlers, and typed client helpers.

---

## 0) Protocol — messages & shape (build.proto)
Add/confirm these in `proto/mcp/unity/v1/build.proto`, then regenerate Rust/C# code.

```proto
// Supported platforms (subset; extend as needed)
enum BuildPlatform {
  BP_UNSPECIFIED = 0;
  BP_STANDALONE_WINDOWS64 = 1;
  BP_STANDALONE_OSX = 2;   // macOS
  BP_STANDALONE_LINUX64 = 3;
  BP_ANDROID = 10;
  BP_IOS = 11;
}

// Architecture or variant knobs (optional where relevant)
message BuildVariants {
  string architecture = 1;   // e.g., "x86_64", "arm64" (macOS), "universal"
  repeated string abis = 2;  // e.g., ["arm64-v8a","armeabi-v7a"] (Android)
  bool development = 3;      // Development build flag
  bool il2cpp = 4;           // Force IL2CPP if applicable
  bool strip_symbols = 5;    // Strip build
}

message BuildPlayerRequest {
  BuildPlatform platform = 1;
  string output_path = 2;            // absolute or project-relative path; server validates
  repeated string scenes = 3;        // project-relative e.g., "Assets/Scenes/Main.unity"; empty = EditorBuildSettings
  BuildVariants variants = 4;        // arch/abi/dev flags
  map<string,string> define_symbols = 5; // scripting define symbols per group (optional)
}

message BuildPlayerResponse {
  int32 status_code = 1;     // 0 OK; nonzero = failure
  string message = 2;
  string output_path = 3;     // final file/dir
  uint64 build_time_ms = 4;
  uint64 size_bytes = 5;      // if available from report
  repeated string warnings = 6;
}

message BuildAssetBundlesRequest {
  string output_directory = 1; // absolute or project-relative
  bool deterministic = 2;      // BuildAssetBundleOptions.DeterministicAssetBundle
  bool chunk_based = 3;        // ChunkBasedCompression
  bool force_rebuild = 4;      // ForceRebuildAssetBundle
}

message BuildAssetBundlesResponse {
  int32 status_code = 1;
  string message = 2;
  string output_directory = 3;
  uint64 build_time_ms = 4;
}

message BuildRequest {
  oneof payload {
    BuildPlayerRequest        player = 1;
    BuildAssetBundlesRequest  bundles = 2;
  }
}

message BuildResponse {
  oneof payload {
    BuildPlayerResponse        player = 1;
    BuildAssetBundlesResponse  bundles = 2;
  }
}
```

Envelope mapping:
- `IpcRequest.payload = BuildRequest`
- `IpcResponse.payload = BuildResponse`
- Long tasks stream `OperationEvent{START/PROGRESS/COMPLETE}` with `op_id`.

**Path policy:** Allow absolute paths **or** project-relative. Normalize, ensure parent escapes are blocked for project-relative inputs.

---

## 1) Unity — Build handler (Editor-only, main-thread)
Create an Editor-only handler and wire it in `EditorIpcServer.DispatchAsync`.

```
Editor/Ipc/BuildHandler.cs
```

### 1.1 Build target mapping helper
```csharp
// Editor/Ipc/BuildHandler.cs (excerpt)
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;
using System.Collections.Generic;
using System.IO;

internal static class BuildHandler
{
    private static (BuildTargetGroup, BuildTarget) MapTarget(Pb.BuildPlatform p, Pb.BuildVariants v)
    {
        return p switch
        {
            Pb.BuildPlatform.BpStandaloneWindows64 => (BuildTargetGroup.Standalone, BuildTarget.StandaloneWindows64),
            Pb.BuildPlatform.BpStandaloneOsx      => (BuildTargetGroup.Standalone, BuildTarget.StandaloneOSX),
            Pb.BuildPlatform.BpStandaloneLinux64  => (BuildTargetGroup.Standalone, BuildTarget.StandaloneLinux64),
            Pb.BuildPlatform.BpAndroid            => (BuildTargetGroup.Android,    BuildTarget.Android),
            Pb.BuildPlatform.BpIos                => (BuildTargetGroup.iOS,        BuildTarget.iOS),
            _ => throw new ArgumentOutOfRangeException(nameof(p), "unsupported platform"),
        };
    }

    private static string NormalizeOutput(string path)
    {
        if (string.IsNullOrEmpty(path)) throw new ArgumentException("output_path required");
        if (!Path.IsPathRooted(path)) path = Path.GetFullPath(Path.Combine(Directory.GetCurrentDirectory(), path));
        Directory.CreateDirectory(Path.GetDirectoryName(path));
        return path;
    }

    public static Pb.BuildResponse Handle(Pb.BuildRequest req)
    {
        return req.PayloadCase switch
        {
            Pb.BuildRequest.PayloadOneofCase.Player  => new Pb.BuildResponse { Player  = BuildPlayer(req.Player) },
            Pb.BuildRequest.PayloadOneofCase.Bundles => new Pb.BuildResponse { Bundles = BuildBundles(req.Bundles) },
            _ => new Pb.BuildResponse { Player = new Pb.BuildPlayerResponse{ statusCode = 2, message = "invalid build request" } },
        };
    }
```

### 1.2 Build Player
```csharp
    private static Pb.BuildPlayerResponse BuildPlayer(Pb.BuildPlayerRequest r)
    {
        var (group, target) = MapTarget(r.Platform, r.Variants);
        var outPath = NormalizeOutput(r.OutputPath);

        // Scenes
        string[] scenes = (r.ScoutsCount > 0) ? r.Scenes.ToArray() : EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();
        if (scenes.Length == 0) return new Pb.BuildPlayerResponse{ StatusCode = 2, Message = "no scenes" };

        // Switch platform if needed
        if (EditorUserBuildSettings.activeBuildTarget != target)
        {
            if (!EditorUserBuildSettings.SwitchActiveBuildTarget(group, target))
                return new Pb.BuildPlayerResponse{ StatusCode = 13, Message = $"failed to switch build target to {target}" };
        }

        // Scripting define symbols (optional)
        // Note: multi-group handling omitted for brevity
        // Player settings variants (development, IL2CPP, etc.)
        var buildOptions = BuildOptions.None;
        if (r.Variants != null && r.Variants.Development) buildOptions |= BuildOptions.Development;

        var bpo = new BuildPlayerOptions
        {
            scenes = scenes,
            target = target,
            locationPathName = outPath,
            options = buildOptions,
        };

        string op = OperationTracker.Start("BuildPlayer", $"{target} -> {outPath}");
        var t0 = DateTime.UtcNow;
        try
        {
            BuildReport report = BuildPipeline.BuildPlayer(bpo);
            var summary = report.summary;
            var ok = summary.result == BuildResult.Succeeded;
            OperationTracker.Complete(op, ok ? 0 : 13, summary.result.ToString());
            return new Pb.BuildPlayerResponse
            {
                StatusCode = ok ? 0 : 13,
                Message = ok ? "OK" : summary.result.ToString(),
                OutputPath = outPath,
                BuildTimeMs = (ulong)(summary.totalTime.TotalMilliseconds),
                SizeBytes = (ulong)Math.Max(0, (long)summary.totalSize),
                Warnings = { /* optionally collect summary.report or Analyzer warnings */ },
            };
        }
        catch (Exception ex)
        {
            OperationTracker.Complete(op, 13, ex.Message);
            return new Pb.BuildPlayerResponse { StatusCode = 13, Message = ex.Message };
        }
    }
```

### 1.3 Build Asset Bundles
```csharp
    private static Pb.BuildAssetBundlesResponse BuildBundles(Pb.BuildAssetBundlesRequest r)
    {
        var outDir = NormalizeOutput(r.OutputDirectory);
        string op = OperationTracker.Start("BuildBundles", outDir);
        var t0 = DateTime.UtcNow;
        try
        {
            var opts = BuildAssetBundleOptions.None;
            if (r.Deterministic) opts |= BuildAssetBundleOptions.DeterministicAssetBundle;
            if (r.ChunkBased)   opts |= BuildAssetBundleOptions.ChunkBasedCompression;
            if (r.ForceRebuild) opts |= BuildAssetBundleOptions.ForceRebuildAssetBundle;

            // NOTE: you may need to specify BuildTarget; we reuse current active target
            var target = EditorUserBuildSettings.activeBuildTarget;
            BuildPipeline.BuildAssetBundles(outDir, opts, target);

            OperationTracker.Complete(op, 0, "OK");
            return new Pb.BuildAssetBundlesResponse
            {
                StatusCode = 0,
                Message = "OK",
                OutputDirectory = outDir,
                BuildTimeMs = (ulong)(DateTime.UtcNow - t0).TotalMilliseconds,
            };
        }
        catch (Exception ex)
        {
            OperationTracker.Complete(op, 13, ex.Message);
            return new Pb.BuildAssetBundlesResponse { StatusCode = 13, Message = ex.Message };
        }
    }
}
```

### 1.4 Wire in `EditorIpcServer`
```csharp
// EditorIpcServer.DispatchAsync(...)
case Pb.IpcRequest.PayloadOneofCase.Build:
{
    var res = BuildHandler.Handle(req.Build);
    await SendResponseAsync(s, cid, 0, "OK", new Pb.IpcResponse { Build = res });
    break;
}
```
> Ensure dispatch happens on the **main thread** when touching Unity APIs. If your IPC loop runs on a worker, marshal via a main-thread dispatcher (e.g., a queue serviced by `EditorApplication.update`).

---

## 2) Rust — typed client helpers & tool glue

### 2.1 IpcClient convenience methods
```rust
// server/src/ipc/client.rs (excerpt)
use crate::generated::mcp::unity::v1 as pb;
use std::time::Duration;

impl IpcClient {
    pub async fn build_player(&self, req: pb::BuildPlayerRequest, timeout: Duration)
        -> Result<pb::BuildPlayerResponse, IpcError>
    {
        let req = pb::IpcRequest{ payload: Some(pb::ipc_request::Payload::Build(pb::BuildRequest{ payload: Some(pb::build_request::Payload::Player(req)) })) };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Build(pb::BuildResponse{ payload: Some(pb::build_response::Payload::Player(r)) })) => Ok(r),
            _ => Err(IpcError::Handshake("unexpected build response".into()))
        }
    }

    pub async fn build_bundles(&self, req: pb::BuildAssetBundlesRequest, timeout: Duration)
        -> Result<pb::BuildAssetBundlesResponse, IpcError>
    {
        let req = pb::IpcRequest{ payload: Some(pb::ipc_request::Payload::Build(pb::BuildRequest{ payload: Some(pb::build_request::Payload::Bundles(req)) })) };
        let resp = self.request(req, timeout).await?;
        match resp.payload {
            Some(pb::ipc_response::Payload::Build(pb::BuildResponse{ payload: Some(pb::build_response::Payload::Bundles(r)) })) => Ok(r),
            _ => Err(IpcError::Handshake("unexpected build response".into()))
        }
    }
}
```

### 2.2 MCP tool facade (example)
```rust
// server/src/mcp/tools/build.rs
use crate::{ipc::client::IpcClient, generated::mcp::unity::v1 as pb};
use anyhow::Result;
use std::time::Duration;

pub struct BuildTool { pub ipc: IpcClient }

impl BuildTool {
    pub async fn build_player(&self, platform: pb::BuildPlatform, output: String, scenes: Vec<String>) -> Result<pb::BuildPlayerResponse> {
        let req = pb::BuildPlayerRequest{
            platform,
            output_path: output,
            scenes,
            variants: Some(pb::BuildVariants{ architecture: "".into(), abis: vec![], development: false, il2cpp: false, strip_symbols: false }),
            define_symbols: Default::default(),
        };
        Ok(self.ipc.build_player(req, Duration::from_secs(1800)).await?)
    }
}
```

---

## 3) Progress & logs
- Unity handler emits `OperationEvent` updates (`Start` → per-scene or per-phase `Progress` → `Complete`).
- Rust consumes via `IpcClient::events()`; WARN/ERROR logs bypass throttling; INFO logs are batched as per Step 3.

---

## 4) Error mapping
- `status_code = 0` on success; `13=INTERNAL` for exceptions; `2=INVALID_ARGUMENT` for bad inputs; `9=FAILED_PRECONDITION` if required modules/SDKs missing; `7=PERMISSION_DENIED` for path policy violations.
- Include `message` with a short cause; detailed text goes to logs.

---

## 5) Security & policy
- Validate/normalize `output_path` and `output_directory`. For project-relative inputs, block parent escapes. For absolute paths, allow only under user-writable locations.
- Never embed secrets (keystore passwords, provisioning profiles) in events/logs; surface only high-level errors.

---

## 6) Testing

### 6.1 Manual
- Windows/macOS: produce small development builds; verify the output path exists and is executable (where applicable).
- AssetBundles: build to a temp directory; load bundle manifest to confirm integrity.

### 6.2 Integration (Rust↔Unity)
- Fire a Player build and stream progress; assert `OperationEvent` reaches COMPLETE, response status is 0.
- Negative: invalid scenes, invalid path, unsupported platform.

### 6.3 Chaos
- Kill Unity mid-build → Rust should fail the request and reconnect later.

---

## 7) Definition of Done (Step 5)
- `BuildRequest`/`BuildResponse` wired end-to-end.
- Player and AssetBundle builds execute over IPC on the Editor main thread.
- Progress and logs stream concurrently; long timeouts handled; failure modes surfaced with consistent codes.

---

## 8) What’s next (Step 6 preview)
- Remove any remaining gRPC artifacts from the repo and CI; finalize docs.
- Optional: add **CancelOperation** support to terminate long builds (`IpcRequest.Cancel { op_id }`).

