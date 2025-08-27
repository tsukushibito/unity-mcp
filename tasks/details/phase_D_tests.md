# フェーズD — テスト強化と最終仕上げ（詳細作業書）

## 概要
- 目的: Rust/Unity双方のテストを拡充し、最終ゲート前の品質を担保する。
- スコープ: Rust統合テスト、Unity EditModeテスト、再接続の仕上げ（任意）。
- 非スコープ: 長期耐久・負荷試験。
- 対応する計画項目: `mvp_work_plan_direct_ipc_v1.md` フェーズD（D1/D2/D3）

## 前提/依存
- フェーズA/B/Cの仕様が確定していること。

## 作業項目一覧

### D1: Rust統合テスト追加
- D1-1: schema mismatch
- D1-2: project_root mismatch
- D1-3: 機能交渉（unknown featureのドロップ）

### D2: Unity EditModeテスト更新
- D2-1: トークン必須
- D2-2: 基本ハッピーパス（Health/Assets/Build）
- D2-3: トークン取得経路の検証（EditorUserSettingsのみ有効、Env/EditorPrefsは無視）

### D3: （任意）再接続の仕上げ
- D3-1: `spawn_supervisor` の writer 差し替えを実装
- D3-2: 手動検証（Unity再起動→自動再接続）

## 受け入れ条件（DoD）
- 上記テストが追加・更新され、`cargo test` とUnity EditModeテストがグリーン

## テスト
- 自動: `server/tests/**`、Unity EditMode
- 手動: 再接続挙動の簡易検証

## リスク/ロールバック
- テストフレーク: リトライや適切なタイムアウト設定

## 監査ログ
- CI実行URL、テストレポート、PRリンク

## 参照
- `tasks/mvp_work_plan_direct_ipc_v1.md` フェーズD
- `tasks/mvp_worklist_checklist.md`

