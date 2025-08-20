# Step 5B — Unity Build Handler 実装: Player & AssetBundles ビルド機能

**目的:** Unity Editor 側で Build リクエストを処理する `BuildHandler` を実装し、`EditorIpcServer` に統合する。

**前提条件:** Step 5A (Protocol 更新) 完了済み

**所要時間:** 4-5時間（ハンドラー実装 + パス検証 + 統合 + テスト）

---

## 実装対象

1. **BuildHandler.cs** - Build リクエスト処理の中核
2. **EditorIpcServer.cs** - Build dispatch 追加
3. **PathPolicy.cs** - セキュリティポリシー (パス検証)

---

## 1) BuildHandler.cs 実装

### 1.1 ファイル作成場所
```
bridge/Packages/com.example.mcp-bridge/Editor/Ipc/BuildHandler.cs
```

### 1.2 基本構造
```csharp
// Editor/Ipc/BuildHandler.cs
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace Mcp.Unity.V1.Ipc
{
    internal static class BuildHandler
    {
        public static Pb.BuildResponse Handle(Pb.BuildRequest req)
        {
            return req.PayloadCase switch
            {
                Pb.BuildRequest.PayloadOneofCase.Player  => new Pb.BuildResponse { Player  = BuildPlayer(req.Player) },
                Pb.BuildRequest.PayloadOneofCase.Bundles => new Pb.BuildResponse { Bundles = BuildBundles(req.Bundles) },
                _ => new Pb.BuildResponse { 
                    Player = new Pb.BuildPlayerResponse { 
                        StatusCode = 2, 
                        Message = "invalid build request" 
                    } 
                },
            };
        }
    }
}
```

### 1.3 プラットフォームマッピング
```csharp
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
```

### 1.4 Build Player 実装
```csharp
private static Pb.BuildPlayerResponse BuildPlayer(Pb.BuildPlayerRequest r)
{
    try
    {
        // 1. プラットフォーム/バリアント処理
        var (group, target) = MapTarget(r.Platform, r.Variants);
        
        // 2. パス検証・正規化
        if (!PathPolicy.TryResolvePlayerOutput(r.OutputPath, out var outPath, out var pathError))
            return new Pb.BuildPlayerResponse { StatusCode = 7, Message = pathError };

        // 3. シーン検証
        string[] scenes = (r.Scenes.Count > 0) ? 
            r.Scenes.ToArray() : 
            EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();
        
        if (scenes.Length == 0) 
            return new Pb.BuildPlayerResponse { StatusCode = 2, Message = "no scenes" };

        // 4. プラットフォーム切替
        if (EditorUserBuildSettings.activeBuildTarget != target)
        {
            if (!EditorUserBuildSettings.SwitchActiveBuildTarget(group, target))
                return new Pb.BuildPlayerResponse { 
                    StatusCode = 9, 
                    Message = $"failed to switch build target to {target}" 
                };
        }

        // 5. ビルドオプション設定
        var buildOptions = BuildOptions.None;
        if (r.Variants?.Development == true) buildOptions |= BuildOptions.Development;

        var bpo = new BuildPlayerOptions
        {
            scenes = scenes,
            target = target,
            locationPathName = outPath,
            options = buildOptions,
        };

        // 6. OperationTracker でビルド実行
        string op = OperationTracker.Start("BuildPlayer", $"{target} -> {outPath}");
        var t0 = DateTime.UtcNow;
        
        BuildReport report = BuildPipeline.BuildPlayer(bpo);
        var summary = report.summary;
        var ok = summary.result == BuildResult.Succeeded;
        
        OperationTracker.Complete(op, ok ? 0 : 13, summary.result.ToString());
        
        return new Pb.BuildPlayerResponse
        {
            StatusCode = ok ? 0 : 13,
            Message = ok ? "OK" : summary.result.ToString(),
            OutputPath = outPath,
            BuildTimeMs = (ulong)summary.totalTime.TotalMilliseconds,
            SizeBytes = (ulong)Math.Max(0, (long)summary.totalSize),
            // Warnings = { /* optionally collect from report */ },
        };
    }
    catch (Exception ex)
    {
        return new Pb.BuildPlayerResponse { StatusCode = 13, Message = ex.Message };
    }
}
```

### 1.5 Build Asset Bundles 実装
```csharp
private static Pb.BuildAssetBundlesResponse BuildBundles(Pb.BuildAssetBundlesRequest r)
{
    try
    {
        // 1. パス検証・正規化  
        if (!PathPolicy.TryResolveBundlesOutput(r.OutputDirectory, out var outDir, out var pathError))
            return new Pb.BuildAssetBundlesResponse { StatusCode = 7, Message = pathError };

        // 2. ビルドオプション設定
        var opts = BuildAssetBundleOptions.None;
        if (r.Deterministic) opts |= BuildAssetBundleOptions.DeterministicAssetBundle;
        if (r.ChunkBased)   opts |= BuildAssetBundleOptions.ChunkBasedCompression;
        if (r.ForceRebuild) opts |= BuildAssetBundleOptions.ForceRebuildAssetBundle;

        // 3. OperationTracker でビルド実行
        string op = OperationTracker.Start("BuildBundles", outDir);
        var t0 = DateTime.UtcNow;
        
        // 現在のアクティブターゲット使用（調査結果に基づく）
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
        return new Pb.BuildAssetBundlesResponse { StatusCode = 13, Message = ex.Message };
    }
}
```

---

## 2) PathPolicy.cs 実装

### 2.1 ファイル作成場所
```
bridge/Packages/com.example.mcp-bridge/Editor/Ipc/PathPolicy.cs
```

### 2.2 セキュリティポリシー実装
```csharp
// Editor/Ipc/PathPolicy.cs
using System;
using System.IO;

namespace Mcp.Unity.V1.Ipc
{
    internal static class PathPolicy
    {
        static readonly string ProjectRoot = Directory.GetCurrentDirectory().Replace('\\','/');
        static readonly string BuildsRoot  = Path.Combine(ProjectRoot, "Builds").Replace('\\','/');
        static readonly string AbRoot      = Path.Combine(ProjectRoot, "AssetBundles").Replace('\\','/');

        static bool IsUnder(string child, string parent)
            => child.StartsWith(parent.EndsWith("/") ? parent : parent + "/", StringComparison.OrdinalIgnoreCase)
            || string.Equals(child, parent, StringComparison.OrdinalIgnoreCase);

        static bool IsSystemPath(string full)
        {
#if UNITY_EDITOR_WIN
            full = full.ToLowerInvariant();
            if (full.StartsWith(@"c:\windows\")) return true;
            if (full.StartsWith(@"c:\program files\")) return true;
            if (full.StartsWith(@"\\")) return true; // UNC
            var root = Path.GetPathRoot(full);
            if (string.Equals(full.TrimEnd('\\','/'), root?.TrimEnd('\\','/'), StringComparison.OrdinalIgnoreCase)) return true;
            return false;
#else
            if (full == "/" || full.StartsWith("/usr/") || full.StartsWith("/bin/") || full.StartsWith("/etc/")) return true;
            return false;
#endif
        }

        public static bool TryResolvePlayerOutput(string input, out string resolved, out string error)
        {
            resolved = error = null;
            if (string.IsNullOrWhiteSpace(input)) { error = "output_path required"; return false; }

            var full = Path.GetFullPath(Path.IsPathRooted(input) ? input : Path.Combine(ProjectRoot, input)).Replace('\\','/');
            if (IsSystemPath(full)) { error = "system path not allowed"; return false; }

            // 禁止: Assets/ と Library/
            if (IsUnder(full, Path.Combine(ProjectRoot, "Assets").Replace('\\','/')) ||
                IsUnder(full, Path.Combine(ProjectRoot, "Library").Replace('\\','/')))
            { error = "output under Assets/Library is forbidden"; return false; }

            // 許可: Builds/ 以下 or プロジェクト外
            var allowed = IsUnder(full, BuildsRoot) || !IsUnder(full, ProjectRoot);
            if (!allowed) { error = $"must be under {BuildsRoot}/ or outside project root"; return false; }

            // 親ディレクトリ作成
            var parent = Path.GetDirectoryName(full);
            if (string.IsNullOrEmpty(parent)) { error = "invalid output path"; return false; }
            Directory.CreateDirectory(parent);

            resolved = full;
            return true;
        }

        public static bool TryResolveBundlesOutput(string input, out string resolved, out string error)
        {
            resolved = error = null;
            if (string.IsNullOrWhiteSpace(input)) { error = "output_directory required"; return false; }

            var full = Path.GetFullPath(Path.IsPathRooted(input) ? input : Path.Combine(ProjectRoot, input)).Replace('\\','/');
            if (IsSystemPath(full)) { error = "system path not allowed"; return false; }

            // 許可: AssetBundles/ または Builds/AssetBundles/ または プロジェクト外
            var buildsAb = Path.Combine(BuildsRoot, "AssetBundles").Replace('\\','/');
            var allowed = IsUnder(full, AbRoot) || IsUnder(full, buildsAb) || !IsUnder(full, ProjectRoot);
            if (!allowed) { error = $"must be under {AbRoot}/ or {buildsAb}/ or outside project root"; return false; }

            Directory.CreateDirectory(full);
            resolved = full;
            return true;
        }
    }
}
```

---

## 3) EditorIpcServer.cs 統合

### 3.1 DispatchRequestAsync への追加
```csharp
// EditorIpcServer.cs の DispatchRequestAsync メソッドに追加
case IpcRequest.PayloadOneofCase.Build:
{
    // Build operations must run on the main thread
    BuildResponse buildResponse = null;
    await Task.Run(() =>
    {
        var tcs = new TaskCompletionSource<BuildResponse>();
        EditorApplication.delayCall += () =>
        {
            try
            {
                buildResponse = BuildHandler.Handle(request.Build);
                tcs.SetResult(buildResponse);
            }
            catch (Exception ex)
            {
                tcs.SetException(ex);
            }
        };
        return tcs.Task;
    });

    var response = new IpcResponse
    {
        CorrelationId = correlationId,
        Build = buildResponse
    };

    await SendResponseAsync(stream, response);
    Debug.Log($"[EditorIpcServer] Sent build response: status={buildResponse.Player?.StatusCode ?? buildResponse.Bundles?.StatusCode}");
    break;
}
```

---

## 4) 検証項目

### 4.1 コンパイル確認
- [ ] BuildHandler.cs がエラーなくコンパイルできる
- [ ] PathPolicy.cs がエラーなくコンパイルできる  
- [ ] EditorIpcServer.cs の変更部分がコンパイルできる

### 4.2 パス検証テスト
- [ ] `PathPolicy.TryResolvePlayerOutput("Builds/MyGame.exe")` が成功
- [ ] `PathPolicy.TryResolvePlayerOutput("Assets/bad.exe")` が失敗
- [ ] `PathPolicy.TryResolveBundlesOutput("AssetBundles/")` が成功

### 4.3 メインスレッド実行確認
- [ ] Build リクエストが `EditorApplication.delayCall` 経由で実行される
- [ ] Unity API への適切なアクセスができる

---

## 5) Definition of Done (Step 5B)

- [ ] BuildHandler.cs が完全実装されている
- [ ] PathPolicy.cs でセキュリティポリシーが実装されている
- [ ] EditorIpcServer.cs に Build dispatch が統合されている
- [ ] 全ファイルがエラーなくコンパイルできる
- [ ] Step 5C (Rust Client 実装) に進める状態である

---

## 6) 次のステップ

Step 5B 完了後は **Step 5C (Rust IpcClient 拡張)** に進む。