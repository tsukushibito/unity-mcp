# フェーズB — CIとSSoTの固定化（詳細作業書）

## 概要
- 目的: Protoとスキーマハッシュの単一情報源（SSoT）を確立し、CIで自動検出・パリティ検証する。
- スコープ: Rust側proto再生成・差分検出、C# `SCHEMA_HASH` の事前生成とGit管理、パリティチェック、ログ/エラーメッセージ整備。
- 非スコープ: 大規模CI最適化（キャッシュ/分散などの高度化）。
- 対応する計画項目: `mvp_work_plan_direct_ipc_v1.md` フェーズB（B1/B2）

## 前提/依存
- フェーズAのハッシュ検証方針が固まっていること。
- `proto/unity_mcp.proto` がSSoTであること。

## 作業項目一覧

### B1: Rust側 proto 再生成＋差分検出をCIへ追加
- B1-1: `server/build.rs` の動作確認と再生成プロセスの明文化
- B1-2: CIで`prost`生成物を再生成→`git diff --exit-code`で差分検出
- B1-3: 失敗時メッセージに「再生成/修正手順」を明記

### B2: C# `SCHEMA_HASH` をRust `SCHEMA_HASH` から事前生成しGit管理（CIはパリティ検証）
- B2-1: Rust側からハッシュ値を取得→HEX化する生成スクリプトを用意（既存 `server/scripts/generate-rust-proto.sh` に統合可）
- B2-2: 生成C#ファイル（`SCHEMA_HASH_HEX` 定数）を `bridge/Assets/**` へ配置しコミット（ヘッダーに Generated 注記）
- B2-3: CIでRust↔C#のハッシュ一致を検証（不一致なら失敗し、再生成手順をメッセージに表示）

### パリティCIの詳細（GitHub Actions想定）
- ジョブ名: `proto-and-schema-parity`
- トリガ: `pull_request`, `push` (main)
- 配置: `.github/workflows/ci.yml` に新規ジョブとして追加（推奨）
- 代替: 既存 `build-test` ジョブの末尾にパリティ比較の3ステップ（Rust計算/ C#抽出/ 比較）を追加し、`if: matrix.os == 'ubuntu-latest'` で単一OSのみ実行
- 主要ステップ（擬似YAML）
  - `actions/checkout@v4`
  - Rustツールチェーンセットアップ（stable）
  - 依存セットアップ（`protoc` 必要なら）
  - Proto再生成: `server/scripts/generate-rust-proto.sh` 実行→`git diff --exit-code server/src/generated/`
  - スキーマHEX抽出: 小スクリプトでRust `SCHEMA_HASH`→HEX文字列を取得
  - C#のHEXを取得: `rg` or `sed` で `bridge/Packages/com.example.mcp-bridge/Editor/Generated/SchemaHash.cs` 内の定数を抽出
  - 比較: 不一致なら`exit 1`し、失敗メッセージに「ローカルで `cd bridge && ./Tools/generate-csharp.sh` を実行し、差分をコミット」と表示

例（メッセージ方針）
- 失敗時: `Schema hash parity check failed. Run: cd bridge && ./Tools/generate-csharp.sh && git add -A && git commit -m "chore: regen C# schema hash"`

### 開発者ノート（短）
- 生成・コミットの標準手順を `docs/` に簡単に記載（追加のビルド要件なし）

### 参考YAML（抜粋・差し込み用）
```
  parity-check:
    name: Proto & Schema Parity Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: arduino/setup-protoc@v3
        with: { version: "31.1", repo-token: ${{ secrets.GITHUB_TOKEN }} }
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with: { cache-on-failure: true }
      - name: Regenerate Rust proto and detect drift
        run: |
          cd server && ./scripts/generate-rust-proto.sh
          git diff --exit-code src/generated || {
            echo "::error ::Rust proto drift detected. Run: cd server && ./scripts/generate-rust-proto.sh && git add src/generated && git commit -m 'chore: regen rust proto'"; exit 1; }
      - name: Compute Rust schema hash (hex)
        id: rust_hash
        run: |
          HEX=$(sha256sum server/src/generated/schema.pb | cut -d' ' -f1 | tr 'A-Z' 'a-z'); echo "hex=$HEX" >> "$GITHUB_OUTPUT"
      - name: Extract C# schema hash (hex)
        id: csharp_hash
        run: |
          FILE="bridge/Packages/com.example.mcp-bridge/Editor/Generated/SchemaHash.cs"
          HEX=$(sed -n 's/.*SCHEMA_HASH_HEX\s*=\s*"\([0-9a-fA-F]\{64\}\)".*/\1/p' "$FILE" | head -n1 | tr 'A-Z' 'a-z')
          [ -n "$HEX" ] || { echo "::error ::Failed to extract SCHEMA_HASH_HEX from $FILE"; exit 1; }
          echo "hex=$HEX" >> "$GITHUB_OUTPUT"
      - name: Compare Rust vs C# schema hash
        run: |
          echo "Rust  : ${{ steps.rust_hash.outputs.hex }}"; echo "CSharp: ${{ steps.csharp_hash.outputs.hex }}"
          [ "${{ steps.rust_hash.outputs.hex }}" = "${{ steps.csharp_hash.outputs.hex }}" ] || {
            echo "::error ::Schema hash parity check failed. Run: cd bridge && ./Tools/generate-csharp.sh && commit changes"; exit 1; }
```

## 受け入れ条件（DoD）
- Protoが更新されていればCIが失敗し、修正方法がメッセージで提示される
- `SCHEMA_HASH_HEX`（C#生成物）が常にRustと一致し、手動編集は禁止（Generatedヘッダーあり）

## テスト
- 自動: ダミー差分を入れてCIが失敗することを確認（ローカル再現手順を記載）
- 手動: 生成スクリプトを実行→C#生成物更新→コミットで差分解消されることを確認

## リスク/ロールバック
- CI不安定化: 生成物を最小限にし、キャッシュを適切化
- 差分検出の誤検知: 除外パスや改行差の正規化を実装

## 監査ログ
- PRリンク、CIジョブURL、生成スクリプトのハッシュ

## 参照
- `tasks/mvp_work_plan_direct_ipc_v1.md` フェーズB
- `tasks/mvp_worklist_checklist.md`
