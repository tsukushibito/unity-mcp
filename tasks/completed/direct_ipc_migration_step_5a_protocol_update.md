# Step 5A — Protocol Buffer 定義更新: Build メッセージの完全実装

**目的:** `build.proto` を作業書仕様に合わせて完全更新し、Player/AssetBundle ビルド用の型安全なメッセージ定義を確立する。

**前提条件:** Step 0-4 完了済み

**所要時間:** 2-3時間（プロトコル更新 + コード再生成 + 動作確認）

---

## 現在の状況

現在の `proto/mcp/unity/v1/build.proto` は最小限の定義のみ：
```proto
service Build {
  rpc BuildPlayer(BuildPlayerRequest) returns (BuildPlayerResponse);
}
message BuildPlayerRequest { string target = 1; }
message BuildPlayerResponse { bool started = 1; string op_id = 2; }
```

**問題点:**
- プラットフォーム指定が文字列ベース（型安全性なし）
- AssetBundles サポートなし  
- エラーハンドリング機能なし
- ビルド設定の詳細化ができない

---

## 1) 完全な build.proto 定義

### 1.1 プラットフォーム enum
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
```

### 1.2 ビルド設定
```proto
// Architecture or variant knobs (optional where relevant)
message BuildVariants {
  string architecture = 1;   // e.g., "x86_64", "arm64" (macOS), "universal"
  repeated string abis = 2;  // e.g., ["arm64-v8a","armeabi-v7a"] (Android)
  bool development = 3;      // Development build flag
  bool il2cpp = 4;           // Force IL2CPP if applicable  
  bool strip_symbols = 5;    // Strip build
}
```

### 1.3 Player ビルド
```proto
message BuildPlayerRequest {
  BuildPlatform platform = 1;
  string output_path = 2;            // absolute or project-relative path
  repeated string scenes = 3;        // project-relative e.g., "Assets/Scenes/Main.unity"
  BuildVariants variants = 4;        // arch/abi/dev flags
  map<string,string> define_symbols = 5; // scripting define symbols per group
}

message BuildPlayerResponse {
  int32 status_code = 1;     // 0 OK; nonzero = failure
  string message = 2;
  string output_path = 3;     // final file/dir
  uint64 build_time_ms = 4;
  uint64 size_bytes = 5;      // if available from report
  repeated string warnings = 6;
}
```

### 1.4 AssetBundles ビルド  
```proto
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
```

### 1.5 統合メッセージ
```proto
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

---

## 2) ipc.proto への統合

`proto/mcp/unity/v1/ipc.proto` の `IpcRequest` と `IpcResponse` に Build フィールドを追加：

```proto
message IpcRequest {
  oneof payload {
    IpcHello hello = 1;
    HealthRequest health = 2;
    AssetsRequest assets = 3;
    BuildRequest build = 4;  // <- 追加
  }
}

message IpcResponse {
  string correlation_id = 1;
  oneof payload {
    IpcWelcome welcome = 2;
    HealthResponse health = 3;
    AssetsResponse assets = 4;
    BuildResponse build = 5;  // <- 追加
  }
}
```

---

## 3) 実装手順

### 3.1 Protocol 定義更新
1. `proto/mcp/unity/v1/build.proto` を上記仕様で完全書き換え
2. `proto/mcp/unity/v1/ipc.proto` に Build フィールド追加

### 3.2 コード再生成
```bash
# Rust側
cd server/
cargo clean  # 重要: プロトコル変更時は必須
cargo build

# C#側  
cd bridge/
./Tools/generate-csharp.sh
```

### 3.3 生成確認
- Rust: `server/src/generated/mcp/unity/v1/build.rs` の存在と内容確認
- C#: `bridge/Packages/com.example.mcp-bridge/Editor/Generated/Build.cs` の更新確認

---

## 4) エラーコード体系

gRPC 準拠のステータスコード使用：
- `0`: OK (成功)
- `2`: INVALID_ARGUMENT (無効入力)  
- `5`: NOT_FOUND (ファイル/シーン未発見)
- `7`: PERMISSION_DENIED (パス権限違反)
- `9`: FAILED_PRECONDITION (SDK未導入、ターゲット切替失敗)
- `13`: INTERNAL (Unity例外、BuildResult.Failed)

---

## 5) 検証項目

### 5.1 コンパイル確認
- [ ] Rust プロジェクトがエラーなくビルドできる
- [ ] Unity プロジェクトがエラーなくコンパイルできる

### 5.2 型生成確認
- [ ] `BuildPlatform` enum が正しく生成されている
- [ ] `BuildRequest`/`BuildResponse` の oneof 構造が正しい
- [ ] `IpcRequest`/`IpcResponse` に Build フィールドが追加されている

---

## 6) Definition of Done (Step 5A)

- [ ] `build.proto` が作業書仕様で完全実装されている
- [ ] `ipc.proto` に Build 統合が完了している  
- [ ] Rust/C# 両方でコード生成が成功している
- [ ] 既存の Health/Assets 機能に影響がない
- [ ] Step 5B (Unity実装) に進める状態である

---

## 7) 次のステップ

Step 5A 完了後は **Step 5B (Unity Build Handler 実装)** に進む。