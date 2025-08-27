#if UNITY_EDITOR
using System;
using System.IO;
using NUnit.Framework;
using UnityEngine;
using Pb = Mcp.Unity.V1;
using Mcp.Unity.V1.Ipc; // for EditorIpcServer type to locate assembly

namespace Bridge.Editor.Ipc.Tests
{
    /// <summary>
    /// 軽量ハッピーパス: Assets の P2G/G2P/Refresh と Bundles 最小ビルド
    /// 可能な限りファイル書き込みを抑えつつ、基本経路の成功のみ検証
    /// </summary>
    [TestFixture]
    public class AssetsAndBuildHappyPathTests
    {
        [Test]
        public void TestAssetsP2G_G2P_Refresh()
        {
            var asm = typeof(EditorIpcServer).Assembly;
            var handlerType = asm.GetType("Mcp.Unity.V1.Ipc.AssetsHandler");
            Assert.IsNotNull(handlerType, "AssetsHandler type not found");

            var flags = System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static;
            var p2g = handlerType.GetMethod("P2G", flags);
            var g2p = handlerType.GetMethod("G2P", flags);
            var refresh = handlerType.GetMethod("Refresh", flags);
            Assert.IsNotNull(p2g);
            Assert.IsNotNull(g2p);
            Assert.IsNotNull(refresh);

            // P2G("Assets")
            var p2gReq = new Pb.PathToGuidRequest();
            p2gReq.Paths.Add("Assets");
            var p2gResp = (Pb.AssetsResponse)p2g.Invoke(null, new object[] { p2gReq });
            Assert.AreEqual(0, p2gResp.StatusCode);
            Assert.IsNotNull(p2gResp.P2G);
            Assert.IsTrue(p2gResp.P2G.Map.ContainsKey("Assets"));
            var guid = p2gResp.P2G.Map["Assets"];
            Assert.IsNotEmpty(guid);

            // G2P(guid) -> "Assets"
            var g2pReq = new Pb.GuidToPathRequest();
            g2pReq.Guids.Add(guid);
            var g2pResp = (Pb.AssetsResponse)g2p.Invoke(null, new object[] { g2pReq });
            Assert.AreEqual(0, g2pResp.StatusCode);
            Assert.IsNotNull(g2pResp.G2P);
            Assert.IsTrue(g2pResp.G2P.Map.ContainsKey(guid));
            Assert.AreEqual("Assets", g2pResp.G2P.Map[guid]);

            // Refresh(false)
            var refreshReq = new Pb.RefreshRequest { Force = false };
            var refreshResp = (Pb.AssetsResponse)refresh.Invoke(null, new object[] { refreshReq });
            Assert.AreEqual(0, refreshResp.StatusCode);
            Assert.IsTrue(refreshResp.Refresh.Ok);
        }

        [Test]
        public void TestBuildBundlesMinimal()
        {
            var asm = typeof(EditorIpcServer).Assembly;
            var handlerType = asm.GetType("Mcp.Unity.V1.Ipc.BuildHandler");
            Assert.IsNotNull(handlerType, "BuildHandler type not found");

            var flags = System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Static;
            var buildBundles = handlerType.GetMethod("BuildBundles", flags);
            Assert.IsNotNull(buildBundles);

            // 出力先は PathPolicy に従い AssetBundles/ 配下にする
            var outDir = Path.Combine("AssetBundles", "EditModeSmoke");
            if (Directory.Exists(outDir))
            {
                // 事前に削除しなくてもよいが、状態を整える
                // Directory.Delete(outDir, true); // 任意
            }

            var req = new Pb.BuildAssetBundlesRequest
            {
                OutputDirectory = outDir,
                Deterministic = true,
                ChunkBased = false,
                ForceRebuild = false,
            };

            var resp = (Pb.BuildAssetBundlesResponse)buildBundles.Invoke(null, new object[] { req });
            Assert.IsNotNull(resp);
            Assert.AreEqual(0, resp.StatusCode, $"BuildAssetBundles should succeed: {resp.Message}");
            Assert.IsTrue(Directory.Exists(resp.OutputDirectory), "Output directory should exist");
        }
    }
}
#endif
