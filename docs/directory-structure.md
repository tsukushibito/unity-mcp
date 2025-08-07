# Unity MCP Server ― 最小ディレクトリ構成

rmcp Rust SDK を使って **最小限の開発コスト** で Unity 向け MCP サーバーを構築する際のディレクトリ構成をまとめています。まず MVP を素早く届け、その後の拡張に備えることを目的としています。

---

## ルートレイアウト

```text
unity-mcp-server/
├── server/                    # Rust 製 MCP サーバー（単一クレート）
│   ├── Cargo.toml             # 依存 & バイナリ定義
│   ├── src/
│   │   ├── main.rs            # エントリーポイント
│   │   └── handlers/          # rmcp ハンドラを機能別に分割
│   ├── config/                # デフォルト設定（TOML / YAML）
│   └── README.md
│
├── bridge/                    # Unity プロジェクト
│   ├── Assets/
│   │   └── MCP/
│   │       ├── Editor/        # メニュー、EditorWindow、CLI 呼び出し
│   │       └── Runtime/       # （任意）PlayMode 用クライアント
│   ├── Packages/
│   │   └── com.example.mcp-bridge/   # UPM パッケージ化（再利用用）
│   ├── ProjectSettings/
│   └── UserSettings/
│
├── docs/                      # アーキテクチャ図・How‑to
├── scripts/                   # ビルド・テスト・リリース補助
├── .github/workflows/         # rust-ci.yml / unity-ci.yml / lint.yml
├── .devcontainer/             # VS Code Dev Container
└── README.md
```

---

## この構成を選ぶ理由

| 階層                                         | 目的                                          | 根拠                                                                                     |
| ------------------------------------------ | ------------------------------------------- | -------------------------------------------------------------------------------------- |
| **server/**                                | MCP サーバーを 1 クレートで実装                         | `rmcp` が複数トランスポートを抽象化しているため、クレート追加なしで stdio / WebSocket 対応が可能。規模拡大時に workspace 化すれば良い。 |
| **bridge/Assets/MCP/Editor**               | Unity Editor から Rust サーバーを起動し、メニューやウィンドウを提供 | MVP では Editor 機能だけで十分。ランタイムビルドをスリムに保てる。                                                |
| **bridge/Packages/com.example.mcp-bridge** | UPM パッケージ化                                  | `git url?path=` で他プロジェクトへ簡単に導入可能。                                                      |
| **.github/workflows**                      | Rust と Unity を並列 CI                         | 1 リポジトリなので checkout が 1 度で済む。                                                          |
| **.devcontainer**                          | 統一開発環境                                      | Rust toolchain + Unity Hub CLI を含め、オンボーディングを簡略化。                                       |

---

## 運用メモ

1. **設定ファイル**

   * `server/config/` に `default.toml` を配置。
   * 別設定が必要なら `--config /path/to/xxx.toml` で上書き。

2. **ロギング**

   * `tracing` & `tracing-subscriber` を `main.rs` に直接記述。
   * ログの粒度が増えたらモジュール化を検討。

3. **拡張パス**

   * コード量・ビルド時間が増えたら `server/` を workspace ルートとし、`core` や `transport-*` へ切り出す。
   * Unity 側は既に UPM 化されているため、機能追加は単純にフォルダを増やすだけで済む。

---

### まとめ

**Rust 1 クレート × Unity 1 プロジェクト × 1 リポジトリ** で始めることで、フィードバックループを最短化しつつ、将来的な拡張にもスムーズに対応できます。
