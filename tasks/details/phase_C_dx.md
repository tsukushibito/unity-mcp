# フェーズC — 開発者体験（DX）（詳細作業書）

## 概要
- 目的: 新規クローンから15分以内でE2Eを再現可能にする開発者体験の整備。
- スコープ: Quickstart、例の拡充（ログtail、成功/失敗の見える化）。
- 非スコープ: 包括的チュートリアルや動画教材の作成。
- 対応する計画項目: `mvp_work_plan_direct_ipc_v1.md` フェーズC（C1/C2）

## 前提/依存
- フェーズA/Bの仕様とUI文言が確定していること。

## 作業項目一覧

### C1: Quickstart作成
- C1-1: Unity起動→`cargo run --example test_unity_ipc` の通し手順の記述
- C1-2: EditorUserSettingsで`MCP.IpcToken`を設定する手順の明記（Env/EditorPrefsを不使用と明示）
- C1-3: トラブルシュート項目（Unityバージョン差・DLL等）

### C2: 例の拡充
- C2-1: Rust側でUnityログイベントを~10秒tailする例を追加
- C2-2: 成功/失敗の判定出力を明確化（色/タグなどは任意）

## 受け入れ条件（DoD）
- Quickstartに従い、15分以内にE2Eが再現できる
- 例の出力で成功/失敗が一目で分かる

## テスト
- 手動: 新規クローン環境でQuickstartを実施、タイムボックス内で再現
- 自動: 最低限のスクリプト体験（ヘルスチェック例）の実行確認

## リスク/ロールバック
- Unity環境差による躓き: トラブルシュートを厚めに用意

## 監査ログ
- スクリーンショット、ログ抜粋、PRリンク

## 参照
- `tasks/mvp_work_plan_direct_ipc_v1.md` フェーズC
- `tasks/mvp_worklist_checklist.md`

