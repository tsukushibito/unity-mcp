# Step 1: Protobuf → C# Generation Plan

## 概要
Protocol Buffer ファイルから C# gRPC/Protobuf スタブを生成し、Unity プロジェクトにコミットする作業の詳細計画。

## 前提条件
- ~~ `.devcontainer/Dockerfile` で `grpc_csharp_plugin` が追加済み ~~
- ~~ コンテナの再ビルドが完了済み ~~

## 作業ステップ

### 1. 開発環境の再セットアップ

```bash
# VS Code で devcontainer の再ビルド
# Command Palette (Ctrl+Shift+P) → "Dev Containers: Rebuild Container"
```

### 2. gRPC C# プラグインの動作確認

```bash
# プラグインが利用可能であることを確認
which grpc_csharp_plugin
grpc_csharp_plugin --version

# protoc が正常に動作することを確認
protoc --version
```

### 3. ディレクトリ構造の作成

```bash
# リポジトリルートから実行
mkdir -p bridge/Assets/Editor/Generated/Proto
```

### 4. C# コード生成

```bash
# 環境変数設定
PROTO_ROOT=proto
OUT=bridge/Assets/Editor/Generated/Proto

# protoc実行でC#ファイルを生成
protoc \
  -I"$PROTO_ROOT" \
  --csharp_out="$OUT" \
  --grpc_out="$OUT" \
  --plugin=protoc-gen-grpc=grpc_csharp_plugin \
  $(find "$PROTO_ROOT/mcp/unity/v1" -name '*.proto')
```

### 5. 生成結果の確認

```bash
# 生成されたファイルを確認
ls -la bridge/Assets/Editor/Generated/Proto/

# 期待される生成ファイル:
# - Common.cs / CommonGrpc.cs
# - EditorControl.cs / EditorControlGrpc.cs  
# - Operations.cs / OperationsGrpc.cs
# - Assets.cs / AssetsGrpc.cs
# - Build.cs / BuildGrpc.cs
# - Events.cs / EventsGrpc.cs
```

### 6. Unity での確認（将来のステップ用）

```bash
# Unity Editor で開いてコンパイル確認
# - 生成された型に参照エラーが無いこと
# - using Grpc.Core などが解決されること
```

### 7. Git コミット

```bash
# 生成されたファイルをステージング
git add bridge/Assets/Editor/Generated/Proto/

# コミット
git commit -m "feat: add generated C# gRPC stubs for Unity bridge

- Generate C# classes from proto files in proto/mcp/unity/v1/
- Include both message classes and gRPC service stubs
- Files committed for Unity compilation without MSBuild dependency

🤖 Generated with Claude Code
Co-Authored-By: Claude <noreply@anthropic.com>"
```

## 完了条件 (Definition of Done)

1. **環境**: `grpc_csharp_plugin` がインストールされ、動作確認済み
2. **生成**: 6つの proto ファイルから対応する .cs ファイルが生成
3. **配置**: `bridge/Assets/Editor/Generated/Proto/` に配置済み
4. **Unity**: Unity Editor でコンパイルエラーが無い
5. **Git**: 生成されたファイルがバージョン管理に追加済み

## トラブルシューティング

### gRPC C# プラグインが見つからない場合
```bash
# Dockerfileの変更後、コンテナを完全に再ビルド
# Command Palette → "Dev Containers: Rebuild Container Without Cache"
```

### protoc 実行時のエラー
```bash
# proto ファイルの依存関係確認
grep -r "import" proto/mcp/unity/v1/

# 出力ディレクトリの権限確認
ls -la bridge/Assets/Editor/Generated/
```

### Unity コンパイルエラー
- Step 2 で Grpc.Core DLL の追加が必要
- Editor-only Assembly Definition (.asmdef) の作成が必要

## 次のステップ
このドキュメント完了後、Step 2「Editor-only Assembly & Dependencies」に進む。