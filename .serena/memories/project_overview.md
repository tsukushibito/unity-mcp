# Unity MCP Server プロジェクト概要

## プロジェクト目的
Unity MCP Server は、Rust MCP サーバーと Unity Editor ブリッジを組み合わせた双方向コンポーネントプロジェクトです。現在はスケルトン段階で最小限のコードですが、明確なアーキテクチャ方向性を持っています。

## アーキテクチャ
- **server/** - rmcp SDK を使用した複数トランスポート対応（stdio/WebSocket）のRust MCP サーバー
- **bridge/** - Rust サーバーを起動し連携するための Unity Editor ツール
- 高速フィードバックループを目的とした単一リポジトリアプローチ、将来のワークスペース拡張を想定

## テック スタック
### Rust Server
- **rmcp SDK** - MCP プロトコル実装
- **tokio** - 非同期ランタイム
- **tracing** - ログ出力
- **anyhow** - アプリケーションレベルエラーハンドリング
- **serde** - シリアライゼーション

### Unity Bridge
- **Unity Editor** - Unity 2022.3 LTS 以降想定
- **C#** - Unity Editor/Runtime コンポーネント用

## プロジェクト構造
```
unity-mcp/
├── server/                    # Rust MCP サーバー
│   ├── src/
│   │   ├── main.rs           # エントリーポイント  
│   │   └── unity.rs          # Unity 用ハンドラー
│   └── Cargo.toml
├── bridge/                   # Unity プロジェクト
│   ├── Assets/MCP/          
│   │   ├── Editor/          # Unity Editor 統合（MVP重視）
│   │   └── Runtime/         # PlayMode 用クライアント
│   └── Packages/com.example.mcp-bridge/  # UPM パッケージ化
├── docs/                     # アーキテクチャドキュメント
├── .github/workflows/        # CI設定
└── .devcontainer/           # 開発環境設定
```

## 開発環境
- **Dev Container** を使用してRust + .NET + Node.js + GitHub CLI環境を提供
- Ubuntu ベースイメージ
- SSH マウントによる認証情報共有