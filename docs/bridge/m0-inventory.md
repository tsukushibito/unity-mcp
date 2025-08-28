# Unity MCP Bridge - M0 ベースライン調査結果

## 概要
Unity Editor APIのクロススレッドアクセスパターンを調査し、現在のIPCサーバー実装における問題箇所を特定。

**調査日**: 2025-08-25  
**スコープ**: `bridge/Packages/com.example.mcp-bridge/Editor/` 以下の全ファイル  
**目標**: 次のリファクタリング段階（EditorDispatcher実装）に向けた準備

## 重要な発見事項

### クロススレッド問題の根本原因
現在の実装では以下のパターンでクロススレッドアクセスが発生：

1. **接続処理**: `Task.Run(() => AcceptConnectionsAsync())` で BG スレッド開始
2. **リクエスト処理**: 各接続を `Task.Run(() => HandleConnectionAsync())` で並行処理  
3. **Unity API 呼び出し**: BG スレッドから直接 Unity Editor API にアクセス

### 現在の対処法の問題
- Assets/Build 処理で `EditorApplication.delayCall` を使用してメインスレッドに復帰
- ただし、Handshake や Health 処理では対処されていない

## Unity API 使用箇所一覧

### EditorIpcServer.cs
| 行番号 | メソッド | Unity API | 用途 | スレッド状況 |
|--------|----------|-----------|------|-------------|
| 42 | static constructor | `EditorApplication.quitting` | 終了時コールバック登録 | MAIN (安全) |
| 311-312 | HandleHealthRequest | `EditorApplication.isCompiling/isUpdating`, `Application.unityVersion` | エディタ状態取得 | **BG (危険)** |
| 345, 394 | HandleAssets/BuildRequest | `EditorApplication.delayCall` | メインスレッド復帰 | BG→MAIN (対処済み) |
| 468, 470 | CreateWelcome | `Application.unityVersion`, `Application.platform` | バージョン情報取得 | **BG (危険)** |
| 658, 663 | ValidateEditorState | `EditorApplication.isCompiling/isUpdating` | エディタ状態検証 | **BG (危険)** |
| 742-743 | UpdateEditorStateCache | `EditorApplication.isCompiling/isUpdating` | 状態キャッシュ更新 | MAIN (安全) |

### ServerFeatureConfig.cs
| 行番号 | メソッド | Unity API | 用途 | スレッド状況 |
|--------|----------|-----------|------|-------------|
| 47 | IsBuildSystemAvailable | `EditorApplication.isPlayingOrWillChangePlaymode` | ビルド可能性チェック | **不明 (危険)** |

### EditorLogBridge.cs
| 行番号 | メソッド | Unity API | 用途 | スレッド状況 |
|--------|----------|-----------|------|-------------|
| 15 | static constructor | `Application.logMessageReceivedThreaded` | ログイベント登録 | MAIN (安全) |

### AssetsHandler.cs
| 行番号 | メソッド | Unity API | 用途 | スレッド状況 |
|--------|----------|-----------|------|-------------|
| 57, 65 | Import | `AssetDatabase.AssetPathToGUID`, `ImportAsset`, `Refresh` | アセットインポート | MAIN (delayCall経由) |
| 93, 97 | Move | `AssetDatabase.MoveAsset`, `AssetPathToGUID` | アセット移動 | MAIN (delayCall経由) |
| 114 | Delete | `AssetDatabase.MoveAssetToTrash`, `DeleteAsset` | アセット削除 | MAIN (delayCall経由) |
| 125 | Refresh | `AssetDatabase.Refresh` | アセット更新 | MAIN (delayCall経由) |
| 136, 147 | G2P/P2G | `AssetDatabase.GUIDToAssetPath`, `AssetPathToGUID` | パス変換 | MAIN (delayCall経由) |

### BuildHandler.cs
| 行番号 | メソッド | Unity API | 用途 | スレッド状況 |
|--------|----------|-----------|------|-------------|
| 70 | BuildPlayer | `EditorBuildSettings.scenes` | シーン取得 | MAIN (delayCall経由) |
| 76, 78 | BuildPlayer | `EditorUserBuildSettings.activeBuildTarget`, `SwitchActiveBuildTarget` | プラットフォーム切替 | MAIN (delayCall経由) |
| 101 | BuildPlayer | `BuildPipeline.BuildPlayer` | プレイヤービルド | MAIN (delayCall経由) |
| 145-146 | BuildBundles | `EditorUserBuildSettings.activeBuildTarget`, `BuildPipeline.BuildAssetBundles` | バンドルビルド | MAIN (delayCall経由) |

## バックグラウンドスレッド起点一覧

### 主要な BG スレッド作成箇所
| ファイル | 行番号 | パターン | 説明 | 影響範囲 |
|---------|--------|----------|-------|-----------|
| EditorIpcServer.cs | 69 | `Task.Run(() => AcceptConnectionsAsync())` | 接続受付ループ | **高リスク** - すべてのHandshake処理 |
| EditorIpcServer.cs | 124 | `Task.Run(() => HandleConnectionAsync())` | 個別接続処理 | **高リスク** - Health, 検証処理 |
| EditorIpcServer.cs | 341 | `Task.Run(() => { delayCall... })` | Assets処理のBG化 | **低リスク** - delayCallで対処済み |
| EditorIpcServer.cs | 390 | `Task.Run(() => { delayCall... })` | Build処理のBG化 | **低リスク** - delayCallで対処済み |

### 呼び出しチェーン分析

#### 危険なパス (BG → Unity API 直接アクセス)
```
Task.Run(AcceptConnectionsAsync)
  └─ AcceptConnectionsAsync (BG スレッド)
      └─ Task.Run(HandleConnectionAsync) 
          └─ HandleConnectionAsync (BG スレッド)
              ├─ ValidateEditorState() → EditorApplication.isCompiling ❌
              ├─ SendWelcomeAsync()
              │   └─ CreateWelcome() → Application.unityVersion ❌
              └─ ProcessRequestsAsync()
                  └─ HandleHealthRequest() → EditorApplication.isCompiling ❌
```

#### 対処済みパス (BG → delayCall → MAIN)
```
Task.Run(HandleAssetsRequest)
  └─ EditorApplication.delayCall (BG → MAIN 復帰) ✅
      └─ AssetsHandler.Handle() (MAIN スレッド)
          └─ AssetDatabase.* (安全)
```

## テスト結果

### 実装したテストケース
1. **CrossThreadDiagnosticsTests.cs** - クロススレッドアクセスの検証
2. **MockIpcClient.cs** - 実際のRustサーバーを使わないテスト環境

### テスト項目
- `TestHealthRequestFromBackgroundThread()` - Health リクエスト処理
- `TestAssetsRequestCrossThreadAccess()` - Assets API アクセス
- `TestBuildRequestCrossThreadAccess()` - Build API アクセス  
- `TestEditorStateValidationFromBG()` - エディタ状態検証
- `TestMainThreadDetection()` - メインスレッド検出
- `TestConcurrentConnections()` - 並行接続処理

### 診断機能の実装
- **Diag.cs** - スレッド情報付きログ出力
- `[BRIDGE.THREAD MAIN/BG]` タグでスレッド識別
- `LogUnityApiAccess()` でUnity API呼び出し追跡

## 影響分析

### 現在ブロックされる機能
1. **Handshake (T01)** - エディタ状態検証とバージョン取得でクラッシュ可能性
2. **Health Request** - エディタ状態確認でクラッシュ可能性  
3. **Feature Negotiation** - プラットフォーム判定でクラッシュ可能性

### 正常動作する機能
1. **Assets Operations** - `delayCall` による対処済み
2. **Build Operations** - `delayCall` による対処済み
3. **Event Logging** - メインスレッドから登録済み

## 変更の緊急度

### 🔴 緊急 (即座に修正が必要)
- **HandleHealthRequest** (line 311-312) - 頻繁に呼ばれるため高リスク
- **ValidateEditorState** (line 658, 663) - Handshake失敗の原因
- **CreateWelcome** (line 468, 470) - Handshake失敗の原因

### 🟡 重要 (次の段階で修正)
- **ServerFeatureConfig** (line 47) - 機能判定への影響
- **接続処理全体のアーキテクチャ見直し**

### 🟢 低優先度 (リファクタリング時に修正)
- **Assets/Build処理** - 既に対処済み、より良い解決策への移行

## 次の段階への推奨事項

### M1: EditorDispatcher実装
1. **UnityMainThreadDispatcher** パターンの採用
2. **すべてのUnity API呼び出しをキューに蓄積してメインスレッドで実行**
3. **現在のdelayCall方式を統一されたDispatcher機構に置換**

### 実装優先順位
1. Handshake/Health リクエスト処理の修正
2. Feature negotiation の修正  
3. 既存のdelayCall方式の統合
4. エラーハンドリングとタイムアウト対応

### アーキテクチャの改善提案
- BG スレッドは通信処理のみに専念
- Unity API 呼び出しは全てDispatcher経由
- レスポンスも非同期でBG側に返却

## 結論

現在の実装では重要な部分（Handshake, Health）でクロススレッド問題が発生する一方、Assets/Build処理では`delayCall`で対処されている。統一されたEditorDispatcher機構の導入により、すべてのUnity API呼び出しを安全に処理できるようになる。

テスト環境とログ機能により、問題の再現と修正の効果測定が可能な状態が整った。
