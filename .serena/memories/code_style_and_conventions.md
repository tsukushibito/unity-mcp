# Unity MCP Server - コード規約とスタイルガイド

## 言語とフレームワーク
- **サーバー用**: Rust（rmcp、tracing、非同期用tokio）
- **Unity Bridge 用**: C#（Unity Editor/Runtime コンポーネント）

## インポート整理

### Rust
```rust
// 1. std ライブラリ
use std::collections::HashMap;
use std::io;

// 2. 外部クレート
use anyhow::Result;
use tokio::time::sleep;
use tracing::info;

// 3. ローカルモジュール
use crate::handlers::unity;
use super::common;
```

### C#
```csharp
// 1. System 名前空間
using System;
using System.Collections.Generic;

// 2. Unity 名前空間
using UnityEngine;
using UnityEditor;

// 3. プロジェクト名前空間
using MCP.Bridge;
using MCP.Editor;
```

## 命名規則

### Rust
- **アイテム**: `snake_case` (関数、変数、モジュール)
- **型**: `CamelCase` (構造体、列挙型、トレイト)
- **定数**: `SCREAMING_SNAKE_CASE`

```rust
// 例
struct UnityHandler {
    connection_count: u32,
}

const MAX_CONNECTIONS: usize = 100;

fn handle_request() -> Result<()> {
    // ...
}
```

### C#
- **型/メソッド**: `PascalCase`
- **フィールド**: `camelCase`
- **定数**: `UPPER_CASE`

```csharp
// 例
public class McpBridge
{
    private string serverPath;
    public const int MAX_RETRY_COUNT = 3;
    
    public void StartServer()
    {
        // ...
    }
}
```

## エラーハンドリング

### Rust
- **アプリケーションレベル**: `anyhow` を使用
- **ドメインエラー**: `thiserror` を使用
- **本番環境**: `unwrap()` / `expect()` は避ける
- **リクエストハンドラー**: パニックは禁止

```rust
use anyhow::{Context, Result};

fn process_request() -> Result<String> {
    let data = fetch_data()
        .context("Failed to fetch data from Unity")?;
    Ok(data)
}
```

### C#
- **例外処理**: `try/catch` を使用
- **ログ出力**: `UnityEngine.Debug` を使用

```csharp
try
{
    var result = ProcessData();
    return result;
}
catch (Exception ex)
{
    Debug.LogError($"Processing failed: {ex.Message}");
    throw;
}
```

## 設定管理
- **サーバー設定**: `server/config/` の TOML ファイル
- **CLI フラグオーバーライド**: 後に予定

## テストコード規約
- **Rust**: `cfg(test)` でモジュール内にユニットテストを配置
- **統合テスト**: `server/tests/` に配置
- **決定論的テスト**: ネットワーク依存を避ける
- **Unity**: Unity Test Runner による EditMode テスト