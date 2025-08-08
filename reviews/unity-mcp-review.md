# Unity MCP Server 改善レビュー（修正後実装）

- 対象リポジトリ: `server/` Rust MCP サーバー（`src/grpc/`、`src/unity.rs`、`Cargo.toml`）
- 対象ブランチ/状態: 指摘事項修正後（重複削減・定数化・パス検証強化・エラー統一）
- 本文言語: 日本語

---

## 概要

重複排除・定数の集約・パス検証強化・エラーハンドリング統一が実装され、全体として安全性と保守性が向上しました。特に `server/src/grpc/service.rs` に共通バリデーション関数と定数を集約し、`server/src/grpc/error.rs` でエラー生成と `Status` 変換を統一した効果が大きいです。一方で、設定ファイル活用（外部化の徹底）やパス検証の正規化、gRPC サーバー配線など、次段の改善余地が残ります。

---

## 対象と修正点（サマリ）

- 重複コードの削減
  - `validate_asset_path()` による共通パス検証
  - `validate_move_paths()` による移動操作向けの拡張検証（同一パス拒否など）
  - エラーレスポンス構築のパターン化（`create_import_error_response()` ほか）
- 定数の外部化（実装内集約）
  - `UNITY_ASSETS_PREFIX`, `STUB_PROJECT_NAME`, `STUB_UNITY_VERSION`, `DEFAULT_ASSET_TYPE`, `MAX_PATH_LENGTH` を `UnityMcpServiceImpl` の associated const で集約
- パス検証の強化
  - `Assets/` 接頭辞の必須化、`../`・`..\` のトラバーサル拒否
  - NUL・`<`・`>` の不正文字拒否、260 文字長制限
- エラーハンドリング改善
  - `grpc/error.rs` に `validation_error`/`not_found_error`/`internal_server_error`/`no_error` を集中配置
  - `McpError` → `tonic::Status` 変換 `mcp_error_to_status()` の整備

---

## A. 改善効果の評価

- 品質改善度
  - 重複削減: `import_asset` と `move_asset` でのパス検証/エラー生成が共通化され、同種ロジックの重複が解消（現時点で約20行前後の削減、横展開で 50 行規模まで見込み）。
  - 可読性: エラー生成がヘルパ関数に統一され、RPC 実装の意図が明瞭化。
- セキュリティ強化
  - トラバーサル対策・不正文字・長さ制限の導入により、典型的な攻撃面が「高」→「中〜低」へ低減。
  - まだパス正規化は未導入のため、混在セパレータやエンコーディング等での回避余地は残存（後述）。
- 保守性向上
  - 変更点の局所化: パス検証ポリシー変更が `validate_asset_path()` に集約。
  - エラー生成の一貫性: 呼び出し側の分岐ロジックが簡潔になり、バグ混入余地が低下。

---

## B. 実装品質の再評価

- Rust ベストプラクティス準拠
  - 良い: `tracing` + `#[instrument]` で観測性確保、`Result<Response<_>, Status>` の戻り、テストは `#[cfg(test)]` 下、`anyhow` と `?` の活用、モジュール分割（`grpc::{error,server,service}`）。
  - 改善余地: 定数は「実装内の集約」に留まっており、`server/config/default.toml`（現状空）での設定化が未対応。パス検証は文字列包含ベースで、`Path::components()` を用いた正規化へ移行可能。
- アーキテクチャの適切性
  - gRPC 実装は整備されているが、`main.rs` は `rmcp` の stdio ルータを起動しており、gRPC の配線は未実装。ランタイムとしてどちらを採用するかの方針決定と不要コードの整理が必要。
- Unity 統合準備度
  - 型定義（`UnityAsset`, `ProjectInfo`）や基本検証は整備済み。エンドツーエンドでの呼び出し（Editor 側クライアント）や接続方式（stdio/gRPC）の決定が次段の論点。

---

## C. 残存する課題

- パス検証の正規化
  - 文字列包含検査は回避余地があるため、`std::path::Path` + `components()` で `..` を検知し、`Assets` 配下のみ許可するロジックへ移行推奨。
- 文字チェックの網羅性
  - Windows 予約文字（`:*?"<>|`）、先頭/末尾スペース・ドット、予約名（`CON`, `PRN` 等）への配慮。クロスプラットフォームかつ Unity の制約に沿ったホワイトリスト型検証が望ましい。
- 設定の外部化
  - `UNITY_ASSETS_PREFIX`, `MAX_PATH_LENGTH`, `STUB_*` を `default.toml`（や環境変数）から上書き可能にして、環境差異や要件変更に対応。
- gRPC サーバー配線
  - `GrpcServerBuilder` から `UnityMcpServiceServer::new(UnityMcpServiceImpl)` を `.add_service(...)` で配線し、起動/停止を統合テストで検証。
- 監査・権限
  - 書き込み操作（Import/Move/Delete）のポリシー設定、監査ログ、相関IDの付与。

---

## D. 推奨される次のステップ

- パス正規化の導入（例）

```rust
use std::path::{Component, Path};

fn is_under_assets(path: &str) -> bool {
    let p = Path::new(path);
    let mut comps = p.components();
    match comps.next() {
        Some(Component::Normal(first)) if first == "Assets" => {}
        _ => return false,
    }
    !comps.any(|c| matches!(c, Component::ParentDir))
}
```

- エラーレスポンスの定型化

```rust
trait WithMcpError { fn with_error(err: crate::grpc::McpError) -> Self; }

impl WithMcpError for crate::grpc::ImportAssetResponse {
    fn with_error(err: crate::grpc::McpError) -> Self { Self { asset: None, error: Some(err) } }
}

impl WithMcpError for crate::grpc::MoveAssetResponse {
    fn with_error(err: crate::grpc::McpError) -> Self { Self { asset: None, error: Some(err) } }
}
```

- 設定ファイルの活用（例: `server/config/default.toml`）

```toml
unity_assets_prefix = "Assets/"
max_path_length = 260
project_name = "Unity MCP Test Project"
unity_version = "2023.3.0f1"
```

- テストカバレッジ拡張
  - パス検証（OK/NG ケース網羅）
  - 移動検証（同一パス拒否、異常文字、長さ）
  - エラーコード整合性（400/404/500 の戻り）
  - gRPC サービスの起動〜呼び出しまでの統合テスト

```rust
#[tokio::test]
async fn test_validate_asset_path_traversal() {
    let svc = crate::grpc::service::UnityMcpServiceImpl::new();
    assert!(svc.validate_asset_path("Assets/../a", "asset_path").is_err());
    assert!(svc.validate_asset_path("Assets\\..\\a", "asset_path").is_err());
}

#[tokio::test]
async fn test_validate_move_paths_same() {
    let svc = crate::grpc::service::UnityMcpServiceImpl::new();
    assert!(svc.validate_move_paths("Assets/a", "Assets/a").is_err());
}
```

- Unity Editor 統合準備
  - 接続方式（stdio vs gRPC）の方針決定と最小クライアントの雛形作成（C# `Grpc.Net.Client`、あるいは MCP stdio）。
  - `default.toml` に gRPC ホスト/ポートを追記し、Editor から参照。
  - 検証用メニューコマンド（Import ダミー実行 → Console 出力）。

---

## 修正前後の比較（要点）

- 重複コード
  - 前: 各 RPC ごとに検証/エラー生成が散在
  - 後: `validate_asset_path`/`validate_move_paths` と `error.rs` のヘルパで集約（2 箇所で活用、今後拡張可）
- 定数の外部化
  - 前: ハードコード散在を想定
  - 後: associated const に集約（さらに設定化への余地あり）
- パス検証
  - 前: 最低限/不在
  - 後: 接頭辞・トラバーサル・不正文字・長さの包括チェック
- エラーハンドリング
  - 前: 実装散在
  - 後: `validation_error`/`not_found_error`/`internal_server_error`/`no_error` に統一

---

## 定量的評価（可能な範囲）

- 重複削減: 現状で約20行前後（2 エンドポイント分）。横展開で 50 行規模まで見込み。
- テスト: 既存の `grpc/error.rs` テストは良好。パス検証テストは未整備（推定0件）→ 10〜15 ケースの追加を推奨。
- セキュリティ: 主要チェック導入によりリスク「高」→「中〜低」。パス正規化導入で「低」まで低減見込み。

---

## 具体的コード例（現状参照）

- 定数・検証（`server/src/grpc/service.rs` 抜粋）

```rust
impl UnityMcpServiceImpl {
    const UNITY_ASSETS_PREFIX: &str = "Assets/";
    const STUB_PROJECT_NAME: &str = "Unity MCP Test Project";
    const STUB_UNITY_VERSION: &str = "2023.3.0f1";
    const DEFAULT_ASSET_TYPE: &str = "Unknown";
    const MAX_PATH_LENGTH: usize = 260;

    fn validate_asset_path(&self, path: &str, field_name: &str) -> Result<(), crate::grpc::McpError> {
        if path.trim().is_empty() { /* ... */ }
        if !path.starts_with(Self::UNITY_ASSETS_PREFIX) { /* ... */ }
        if path.contains("../") || path.contains("..\\") { /* ... */ }
        if path.contains('\u{0000}') || path.contains('<') || path.contains('>') { /* ... */ }
        if path.len() > Self::MAX_PATH_LENGTH { /* ... */ }
        Ok(())
    }

    fn validate_move_paths(&self, src: &str, dst: &str) -> Result<(), crate::grpc::McpError> {
        self.validate_asset_path(src, "src_path")?;
        self.validate_asset_path(dst, "dst_path")?;
        if src == dst { /* ... */ }
        Ok(())
    }
}
```

- エラー統一（`server/src/grpc/error.rs` 抜粋）

```rust
pub fn validation_error(message: &str, details: &str) -> McpError { /* 400 */ }
pub fn not_found_error(resource: &str, id: &str) -> McpError { /* 404 */ }
pub fn internal_server_error(message: &str) -> McpError { /* 500 */ }
pub fn no_error() -> Option<McpError> { None }
```

---

## 結論

現行の修正は正しい方向に進んでおり、特に安全性・一貫性・保守性の面で顕著な改善が見られます。次段として「設定の外部化」「パス検証の正規化」「gRPC サーバー配線」「テスト拡充」を進めることで、Unity Editor とのエンドツーエンド統合に耐える実装へ段階的に引き上げられます。

