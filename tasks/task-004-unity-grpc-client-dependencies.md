# タスク 004: Unity gRPCクライアント依存関係と設定のセットアップ

## 概要

Unity プロジェクトに最新のgRPCクライアント依存関係とビルド設定を構成します。これには、HTTP/2サポート用のYetAnotherHttpHandlerを使用したGrpc.Net.Clientの追加、Unity用のプロトコルバッファコンパイルの設定、gRPCコード生成のためのC#ビルド環境の準備が含まれます。

## 受け入れ基準

- [ ] Unity プロジェクトにgRPC NuGetパッケージを追加
- [ ] YetAnotherHttpHandlerでGrpc.Net.Clientを設定
- [ ] C#コード生成用のプロトコルバッファコンパイルを設定
- [ ] 適切な依存関係を持つUnityパッケージマニフェストを作成
- [ ] gRPCクライアント用のアセンブリ定義を設定
- [ ] Unityプロジェクトがエラーなしでコンパイルされることを確認
- [ ] パッケージインポートと依存関係解決をテスト
- [ ] Unity Editorとランタイムとの互換性を確保

## 実装メモ

**必要なNuGetパッケージ:**
- `Grpc.Net.Client` (最新の .NET gRPCクライアント)
- `YetAnotherHttpHandler` (Unity用HTTP/2サポート)
- `Google.Protobuf` (プロトコルバッファランタイム)
- `Grpc.Tools` (C#コード生成、ビルド時のみ)

**Unity統合アプローチ:**
1. **UPMパッケージアプローチ** (推奨): `bridge/Packages/com.example.mcp-bridge/` に依存関係を追加
2. **直接アセンブリアプローチ**: コンパイル済みアセンブリを `bridge/Assets/` に配置

**技術的考慮事項:**
- Unityの.NET互換性要件
- 適切な依存関係分離のためのアセンブリ定義ファイル
- ビルド時プロトコルバッファコンパイル
- クロスプラットフォーム互換性（Editor vs ランタイム）
- Unity環境でのHTTP/2トランスポートサポート

**コード生成設定:**
- プロトコルバッファコンパイラ統合
- C#名前空間設定
- 出力パス管理
- ビルドスクリプト統合

## 作成/修正するファイル

- `bridge/Packages/com.example.mcp-bridge/package.json` - UPMパッケージ定義
- `bridge/Packages/com.example.mcp-bridge/Runtime/McpBridge.Runtime.asmdef` - アセンブリ定義
- `bridge/Packages/com.example.mcp-bridge/Editor/McpBridge.Editor.asmdef` - エディタアセンブリ定義
- プロトコルバッファビルド統合ファイル
- NuGet設定または手動アセンブリ配置

## テスト要件

- Unityプロジェクトがコンパイルエラーなしで開くこと
- gRPCアセンブリがUnity内で正常にロードされること
- 生成されたプロトコルバッファC#コードがコンパイルされること
- アセンブリ定義が依存関係を正しく解決すること
- 既存のUnityパッケージとの競合がないこと
- 基本的なgRPCクライアントのインスタンス化が動作すること
- HTTP/2ハンドラが正しく初期化されること

## 依存関係

- **必要:** タスク 001 (プロトコルバッファサービス定義)

## ブロック対象

- タスク 005: Unity gRPCクライアントラッパーの実装
- タスク 009: 統合テストの作成

## 実装優先度

**高優先度** - Unity側gRPC開発のための必須依存関係設定。

## 特別な考慮事項

**Unityバージョン互換性:**
- Unity 2022.3 LTS要件を考慮
- .NET Standard 2.1 vs .NET Framework互換性
- 必要に応じてIL2CPPコンパイル互換性

**代替アプローチ:**
- 最新のGrpc.Net.ClientでUnity互換性に問題がある場合、レガシーGrpc.Coreへのフォールバックを検討
- 手動アセンブリコンパイル vs NuGet統合オプション