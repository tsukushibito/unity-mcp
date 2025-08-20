// Unity Editor Console で実行する手動テスト用コード
// 使用方法: Unity Editor Console に以下のコードをコピー&ペーストして実行

using UnityEngine;
using UnityEditor;
using Mcp.Unity.V1.Ipc;
using Pb = Mcp.Unity.V1;
using System.Collections.Generic;

/// <summary>
/// Unity Editor Console で直接実行できる手動ビルドテスト
/// </summary>
public static class ManualBuildTests
{
    /// <summary>
    /// Windows Standalone Player ビルドテスト
    /// Unity Editor Console で実行: ManualBuildTests.TestBuildPlayer();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Build Player")]
    public static void TestBuildPlayer()
    {
        Debug.Log("=== Manual Build Player Test ===");
        
        var playerReq = new Pb.BuildPlayerRequest
        {
            Platform = Pb.BuildPlatform.BpStandaloneWindows64,
            OutputPath = "Builds/ManualTest/TestApp.exe",
            Scenes = { }, // デフォルトシーン使用
            Variants = new Pb.BuildVariants 
            { 
                Development = true,
                Il2Cpp = false,
                StripSymbols = false,
                Architecture = "x86_64"
            },
            DefineSymbols = { }
        };

        var buildReq = new Pb.BuildRequest { Player = playerReq };
        
        try 
        {
            var response = BuildHandler.Handle(buildReq);
            
            Debug.Log($"Build result: {response.Player.StatusCode} - {response.Player.Message}");
            
            if (response.Player.StatusCode == 0)
            {
                Debug.Log($"Success! Output: {response.Player.OutputPath}");
                Debug.Log($"Size: {response.Player.SizeBytes} bytes ({response.Player.SizeBytes / (1024.0 * 1024.0):F2} MB)");
                Debug.Log($"Time: {response.Player.BuildTimeMs} ms");
                Debug.Log($"Warnings: {response.Player.Warnings.Count}");
                
                // ファイル存在確認
                if (System.IO.File.Exists(response.Player.OutputPath))
                {
                    Debug.Log("✓ Output file exists");
                }
                else
                {
                    Debug.LogError("✗ Output file not found!");
                }
            }
            else
            {
                Debug.LogError($"Build failed: {response.Player.Message}");
            }
        }
        catch (System.Exception ex)
        {
            Debug.LogError($"Build exception: {ex.Message}");
            Debug.LogError($"Stack trace: {ex.StackTrace}");
        }
    }

    /// <summary>
    /// AssetBundles ビルドテスト
    /// Unity Editor Console で実行: ManualBuildTests.TestBuildAssetBundles();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Build AssetBundles")]
    public static void TestBuildAssetBundles()
    {
        Debug.Log("=== Manual AssetBundles Build Test ===");
        
        var bundlesReq = new Pb.BuildAssetBundlesRequest
        {
            OutputDirectory = "AssetBundles/ManualTest",
            Deterministic = true,
            ChunkBased = false,
            ForceRebuild = true
        };

        var buildReq = new Pb.BuildRequest { Bundles = bundlesReq };
        
        try 
        {
            var response = BuildHandler.Handle(buildReq);
            
            Debug.Log($"AssetBundles result: {response.Bundles.StatusCode} - {response.Bundles.Message}");
            
            if (response.Bundles.StatusCode == 0)
            {
                Debug.Log($"Success! Output: {response.Bundles.OutputDirectory}");
                Debug.Log($"Time: {response.Bundles.BuildTimeMs} ms");
                
                // ディレクトリ存在確認
                if (System.IO.Directory.Exists(response.Bundles.OutputDirectory))
                {
                    Debug.Log("✓ Output directory exists");
                    
                    // ファイル数確認
                    var files = System.IO.Directory.GetFiles(response.Bundles.OutputDirectory, "*", System.IO.SearchOption.AllDirectories);
                    Debug.Log($"Files in output directory: {files.Length}");
                }
                else
                {
                    Debug.LogError("✗ Output directory not found!");
                }
            }
            else
            {
                Debug.LogError($"AssetBundles build failed: {response.Bundles.Message}");
            }
        }
        catch (System.Exception ex)
        {
            Debug.LogError($"AssetBundles build exception: {ex.Message}");
            Debug.LogError($"Stack trace: {ex.StackTrace}");
        }
    }

    /// <summary>
    /// Android ビルドテスト
    /// Unity Editor Console で実行: ManualBuildTests.TestBuildAndroid();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Build Android")]
    public static void TestBuildAndroid()
    {
        Debug.Log("=== Manual Android Build Test ===");
        
        var playerReq = new Pb.BuildPlayerRequest
        {
            Platform = Pb.BuildPlatform.BpAndroid,
            OutputPath = "Builds/ManualTest/TestApp.apk",
            Scenes = { },
            Variants = new Pb.BuildVariants 
            { 
                Development = true,
                Il2Cpp = false,
                StripSymbols = false,
                Architecture = "arm64"
            },
            DefineSymbols = { }
        };
        
        // Android ABIs を追加
        playerReq.Variants.Abis.Add("arm64-v8a");

        var buildReq = new Pb.BuildRequest { Player = playerReq };
        
        try 
        {
            var response = BuildHandler.Handle(buildReq);
            
            Debug.Log($"Android build result: {response.Player.StatusCode} - {response.Player.Message}");
            
            if (response.Player.StatusCode == 0)
            {
                Debug.Log($"Success! Output: {response.Player.OutputPath}");
                Debug.Log($"Size: {response.Player.SizeBytes} bytes");
                Debug.Log($"Time: {response.Player.BuildTimeMs} ms");
            }
            else
            {
                Debug.LogWarning($"Android build failed (expected if SDK not configured): {response.Player.Message}");
            }
        }
        catch (System.Exception ex)
        {
            Debug.LogError($"Android build exception: {ex.Message}");
        }
    }

    /// <summary>
    /// 無効なパスでのエラーテスト
    /// Unity Editor Console で実行: ManualBuildTests.TestInvalidPath();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Invalid Path")]
    public static void TestInvalidPath()
    {
        Debug.Log("=== Manual Invalid Path Test ===");
        
        var playerReq = new Pb.BuildPlayerRequest
        {
            Platform = Pb.BuildPlatform.BpStandaloneWindows64,
            OutputPath = "Assets/InvalidLocation.exe", // 禁止されたパス
            Scenes = { },
            Variants = new Pb.BuildVariants { Development = true },
            DefineSymbols = { }
        };

        var buildReq = new Pb.BuildRequest { Player = playerReq };
        
        try 
        {
            var response = BuildHandler.Handle(buildReq);
            
            Debug.Log($"Invalid path test result: {response.Player.StatusCode} - {response.Player.Message}");
            
            if (response.Player.StatusCode == 7) // PERMISSION_DENIED
            {
                Debug.Log("✓ Correctly rejected invalid path");
            }
            else
            {
                Debug.LogError($"✗ Expected PERMISSION_DENIED (7), got {response.Player.StatusCode}");
            }
        }
        catch (System.Exception ex)
        {
            Debug.LogError($"Invalid path test exception: {ex.Message}");
        }
    }

    /// <summary>
    /// 無効なプラットフォームでのエラーテスト
    /// Unity Editor Console で実行: ManualBuildTests.TestInvalidPlatform();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Invalid Platform")]
    public static void TestInvalidPlatform()
    {
        Debug.Log("=== Manual Invalid Platform Test ===");
        
        var playerReq = new Pb.BuildPlayerRequest
        {
            Platform = (Pb.BuildPlatform)999, // 無効なプラットフォーム
            OutputPath = "Builds/ManualTest/InvalidPlatform.exe",
            Scenes = { },
            Variants = new Pb.BuildVariants { Development = true },
            DefineSymbols = { }
        };

        var buildReq = new Pb.BuildRequest { Player = playerReq };
        
        try 
        {
            var response = BuildHandler.Handle(buildReq);
            
            Debug.Log($"Invalid platform test result: {response.Player.StatusCode} - {response.Player.Message}");
            
            if (response.Player.StatusCode != 0)
            {
                Debug.Log("✓ Correctly rejected invalid platform");
            }
            else
            {
                Debug.LogError("✗ Expected error for invalid platform");
            }
        }
        catch (System.Exception ex)
        {
            Debug.Log($"✓ Invalid platform caused exception (expected): {ex.Message}");
        }
    }

    /// <summary>
    /// 複数のプラットフォームを順次テスト
    /// Unity Editor Console で実行: ManualBuildTests.TestMultiplePlatforms();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Multiple Platforms")]
    public static void TestMultiplePlatforms()
    {
        Debug.Log("=== Manual Multiple Platforms Test ===");
        
        var platforms = new[]
        {
            (Pb.BuildPlatform.BpStandaloneWindows64, "Builds/Multi/Windows/TestApp.exe"),
            (Pb.BuildPlatform.BpStandaloneOsx, "Builds/Multi/macOS/TestApp.app"),
            (Pb.BuildPlatform.BpStandaloneLinux64, "Builds/Multi/Linux/TestApp")
        };

        foreach (var (platform, outputPath) in platforms)
        {
            Debug.Log($"Testing platform: {platform}");
            
            var playerReq = new Pb.BuildPlayerRequest
            {
                Platform = platform,
                OutputPath = outputPath,
                Scenes = { },
                Variants = new Pb.BuildVariants { Development = true },
                DefineSymbols = { }
            };

            var buildReq = new Pb.BuildRequest { Player = playerReq };
            
            try 
            {
                var response = BuildHandler.Handle(buildReq);
                Debug.Log($"  {platform}: {response.Player.StatusCode} - {response.Player.Message}");
            }
            catch (System.Exception ex)
            {
                Debug.LogError($"  {platform} exception: {ex.Message}");
            }
        }
    }

    /// <summary>
    /// ビルドパフォーマンス測定テスト
    /// Unity Editor Console で実行: ManualBuildTests.TestBuildPerformance();
    /// </summary>
    [MenuItem("MCP Bridge/Manual Tests/Test Build Performance")]
    public static void TestBuildPerformance()
    {
        Debug.Log("=== Manual Build Performance Test ===");
        
        var startTime = System.DateTime.Now;
        
        var playerReq = new Pb.BuildPlayerRequest
        {
            Platform = Pb.BuildPlatform.BpStandaloneWindows64,
            OutputPath = "Builds/Performance/TestApp.exe",
            Scenes = { },
            Variants = new Pb.BuildVariants 
            { 
                Development = true, // 高速化のため
                Il2Cpp = false,
                StripSymbols = false
            },
            DefineSymbols = { }
        };

        var buildReq = new Pb.BuildRequest { Player = playerReq };
        
        try 
        {
            var response = BuildHandler.Handle(buildReq);
            var endTime = System.DateTime.Now;
            var totalTime = endTime - startTime;
            
            Debug.Log($"=== Performance Results ===");
            Debug.Log($"Status: {response.Player.StatusCode} - {response.Player.Message}");
            Debug.Log($"Total time (C# side): {totalTime.TotalMilliseconds:F0} ms");
            Debug.Log($"Unity reported time: {response.Player.BuildTimeMs} ms");
            Debug.Log($"Handler overhead: {(totalTime.TotalMilliseconds - response.Player.BuildTimeMs):F0} ms");
            
            if (response.Player.StatusCode == 0)
            {
                Debug.Log($"Build size: {response.Player.SizeBytes} bytes");
                Debug.Log($"Build speed: {(response.Player.SizeBytes / (1024.0 * 1024.0)) / (response.Player.BuildTimeMs / 1000.0):F2} MB/s");
            }
        }
        catch (System.Exception ex)
        {
            Debug.LogError($"Performance test exception: {ex.Message}");
        }
    }
}