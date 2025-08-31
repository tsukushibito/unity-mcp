# Unity MCP Server ツールロードマップ

## 概要

Unity MCP Server は Unity Editor との連携を可能にする MCP (Model Context Protocol) サーバーです。このロードマップは、Unity 開発で必要な各種ツールの実装優先度と計画を示します。

## 現在の実装状況 (Phase 0 - 基礎インフラ) ✅

### 接続管理ツール (2/2 完了)
- ✅ `unity_bridge_status` - Unity Bridge 接続ステータス確認
- ✅ `unity_health` - Unity Bridge ヘルスチェック

### アセット管理ツール (6/6 完了)
- ✅ `unity_assets_import` - アセットのインポート
- ✅ `unity_assets_move` - アセットの移動
- ✅ `unity_assets_delete` - アセットの削除
- ✅ `unity_assets_refresh` - AssetDatabase のリフレッシュ
- ✅ `unity_assets_guid_to_path` - GUID からパスへの変換
- ✅ `unity_assets_path_to_guid` - パスから GUID への変換

### 診断・テストツール (3/3 完了)
- ✅ `unity_get_compile_diagnostics` - C# コンパイル診断取得
- ✅ `unity_run_tests` - テストの実行 (EditMode/PlayMode)
- ✅ `unity_get_test_results` - テスト結果の取得

**Phase 0 進捗**: 11/11 ツール完了 (100%)

## Phase 1 - 開発効率向上ツール (高優先度) 🚀

### ビルド管理ツール (0/6 未実装)
- 🔲 `unity_build_player` - プレイヤービルドの実行
- 🔲 `unity_build_addressables` - アドレサブルコンテンツビルド
- 🔲 `unity_build_assetbundles` - AssetBundle ビルド
- 🔲 `unity_get_build_settings` - ビルド設定の取得
- 🔲 `unity_set_build_settings` - ビルド設定の変更
- 🔲 `unity_get_build_report` - ビルドレポートの取得

### シーン管理ツール (0/14 未実装)
- 🔲 `unity_open_scene` - シーンを開く
- 🔲 `unity_save_scene` - シーンの保存
- 🔲 `unity_create_scene` - 新しいシーンの作成
- 🔲 `unity_get_open_scenes` - 開いているシーンの一覧取得
- 🔲 `unity_set_active_scene` - アクティブシーンの設定
- 🔲 `unity_play_mode_control` - プレイモードの制御 (開始/停止/一時停止)
- 🔲 `unity_get_scene_objects` - シーン内オブジェクトの取得
- 🔲 `unity_find_objects` - オブジェクトの検索
- 🔲 `unity_create_gameobject` - GameObject の作成
- 🔲 `unity_delete_gameobject` - GameObject の削除
- 🔲 `unity_move_gameobject` - GameObject の移動・配置
- 🔲 `unity_duplicate_gameobject` - GameObject の複製
- 🔲 `unity_set_gameobject_properties` - GameObject プロパティの設定
- 🔲 `unity_get_gameobject_info` - GameObject 情報の取得

### プロジェクト設定ツール (0/6 未実装)
- 🔲 `unity_get_project_settings` - プロジェクト設定の取得
- 🔲 `unity_set_project_settings` - プロジェクト設定の変更
- 🔲 `unity_get_player_settings` - プレイヤー設定の取得
- 🔲 `unity_set_player_settings` - プレイヤー設定の変更
- 🔲 `unity_get_quality_settings` - クオリティ設定の取得
- 🔲 `unity_set_quality_settings` - クオリティ設定の変更

### コンポーネント管理ツール (0/8 未実装)
- 🔲 `unity_add_component` - GameObject へのコンポーネント追加
- 🔲 `unity_remove_component` - コンポーネントの削除
- 🔲 `unity_get_components` - GameObject の全コンポーネント取得
- 🔲 `unity_set_component_property` - コンポーネントプロパティの設定
- 🔲 `unity_get_component_property` - コンポーネントプロパティの取得
- 🔲 `unity_copy_component` - コンポーネントのコピー
- 🔲 `unity_enable_component` - コンポーネントの有効/無効切り替え
- 🔲 `unity_reset_component` - コンポーネントのリセット

### プレハブ管理ツール (0/10 未実装)
- 🔲 `unity_create_prefab` - プレハブの作成
- 🔲 `unity_instantiate_prefab` - プレハブのインスタンス化
- 🔲 `unity_update_prefab` - プレハブの更新
- 🔲 `unity_get_prefab_info` - プレハブ情報の取得
- 🔲 `unity_disconnect_prefab` - プレハブ接続の切断
- 🔲 `unity_reconnect_prefab` - プレハブ接続の再接続
- 🔲 `unity_get_prefab_overrides` - プレハブオーバーライドの取得
- 🔲 `unity_apply_prefab_overrides` - プレハブオーバーライドの適用
- 🔲 `unity_revert_prefab_overrides` - プレハブオーバーライドの復元
- 🔲 `unity_create_prefab_variant` - プレハブバリアントの作成

### エディタ制御ツール (0/4 未実装)
- 🔲 `unity_execute_menu_item` - メニューアイテムの実行
- 🔲 `unity_focus_window` - ウィンドウのフォーカス
- 🔲 `unity_get_editor_state` - エディタ状態の取得
- 🔲 `unity_refresh_editor` - エディタの更新

**Phase 1 進捗**: 0/42 ツール完了 (0%)

## Phase 2 - パフォーマンス・品質向上ツール (中優先度) 📊

### プロファイリングツール (0/8 未実装)
- 🔲 `unity_start_profiling` - プロファイリング開始
- 🔲 `unity_stop_profiling` - プロファイリング停止
- 🔲 `unity_get_profiler_data` - プロファイラーデータの取得
- 🔲 `unity_get_memory_snapshot` - メモリスナップショットの取得
- 🔲 `unity_get_frame_debugger_data` - Frame Debugger データの取得
- 🔲 `unity_analyze_build_size` - ビルドサイズ分析
- 🔲 `unity_get_performance_metrics` - パフォーマンス指標の取得
- 🔲 `unity_profile_gpu` - GPU プロファイリング

### パッケージ管理ツール (0/6 未実装)
- 🔲 `unity_list_packages` - インストール済みパッケージの一覧
- 🔲 `unity_install_package` - パッケージのインストール
- 🔲 `unity_remove_package` - パッケージの削除
- 🔲 `unity_update_package` - パッケージの更新
- 🔲 `unity_resolve_packages` - パッケージ依存関係の解決
- 🔲 `unity_get_package_info` - パッケージ情報の取得

### バージョン管理統合ツール (0/4 未実装)
- 🔲 `unity_get_vcs_status` - VCS ステータスの取得
- 🔲 `unity_refresh_vcs` - VCS 情報の更新
- 🔲 `unity_resolve_vcs_conflicts` - VCS 競合の解決
- 🔲 `unity_get_changed_assets` - 変更されたアセットの取得

### ログ・デバッグツール (0/6 未実装)
- 🔲 `unity_get_console_logs` - コンソールログの取得
- 🔲 `unity_clear_console` - コンソールのクリア
- 🔲 `unity_set_debug_mode` - デバッグモードの設定
- 🔲 `unity_capture_screenshot` - スクリーンショットの取得
- 🔲 `unity_get_system_info` - システム情報の取得
- 🔲 `unity_monitor_performance` - リアルタイムパフォーマンス監視

**Phase 2 進捗**: 0/24 ツール完了 (0%)

## Phase 3 - 高度機能ツール (低優先度) ⭐

### ライティング・レンダリングツール (0/8 未実装)
- 🔲 `unity_bake_lightmaps` - ライトマップベイク
- 🔲 `unity_get_lighting_settings` - ライティング設定の取得
- 🔲 `unity_set_lighting_settings` - ライティング設定の変更
- 🔲 `unity_generate_lighting` - ライティングの生成
- 🔲 `unity_get_render_pipeline` - レンダーパイプラインの取得
- 🔲 `unity_set_render_pipeline` - レンダーパイプラインの設定
- 🔲 `unity_optimize_rendering` - レンダリング最適化
- 🔲 `unity_get_graphics_settings` - グラフィック設定の取得

### アニメーション・Timeline ツール (0/6 未実装)
- 🔲 `unity_create_animation_clip` - アニメーションクリップの作成
- 🔲 `unity_get_timeline_assets` - Timeline アセットの取得
- 🔲 `unity_create_timeline` - Timeline の作成
- 🔲 `unity_animate_property` - プロパティのアニメーション
- 🔲 `unity_get_animator_state` - Animator 状態の取得
- 🔲 `unity_control_animation` - アニメーションの制御

### アドレサブル管理ツール (0/6 未実装)
- 🔲 `unity_build_addressables_content` - アドレサブルコンテンツビルド
- 🔲 `unity_get_addressable_groups` - アドレサブルグループの取得
- 🔲 `unity_create_addressable_group` - アドレサブルグループの作成
- 🔲 `unity_assign_addressable` - アセットのアドレサブル割り当て
- 🔲 `unity_analyze_addressables` - アドレサブル分析
- 🔲 `unity_update_addressable_catalog` - アドレサブルカタログの更新

### ネットワーク・接続ツール (0/4 未実装)
- 🔲 `unity_get_network_settings` - ネットワーク設定の取得
- 🔲 `unity_test_connectivity` - 接続テスト
- 🔲 `unity_upload_to_cloud` - クラウドへのアップロード
- 🔲 `unity_download_from_cloud` - クラウドからのダウンロード

**Phase 3 進捗**: 0/24 ツール完了 (0%)

## 実装スケジュール

### Q1 2025 - Phase 1 開発効率向上
- **目標**: Phase 1 ツールの 50% (21/42) を実装
- **重点領域**: シーン管理、GameObject/コンポーネント操作、基本的なプレハブ管理
- **期待成果**: 基本的な Unity 開発ワークフローの自動化

### Q2 2025 - Phase 1 完了 + Phase 2 開始
- **目標**: Phase 1 完了 (42/42) + Phase 2 の 25% (6/24) を実装
- **重点領域**: ビルド管理、プロジェクト設定、プロファイリング開始
- **期待成果**: 包括的な開発環境制御とオブジェクト指向開発支援

### Q3 2025 - Phase 2 中心開発
- **目標**: Phase 2 の 75% (18/24) を実装
- **重点領域**: プロファイリングとパッケージ管理、VCS統合
- **期待成果**: パフォーマンス分析と品質管理の自動化

### Q4 2025 - Phase 2 完了 + Phase 3 開始
- **目標**: Phase 2 完了 (24/24) + Phase 3 の 25% (6/24) を実装
- **重点領域**: 全体最適化と高度機能の基礎
- **期待成果**: 企業レベルの Unity 開発支援

## 技術的考慮事項

### Protocol Buffer 拡張
各フェーズで新しいツールを追加する際は、対応する Protocol Buffer 定義の拡張が必要です：
- `proto/mcp/unity/v1/build.proto` - ビルド関連
- `proto/mcp/unity/v1/editor_control.proto` - エディタ制御
- 新規追加: `gameobject.proto`, `component.proto`, `prefab.proto`, `profiling.proto`, `packages.proto`, `rendering.proto` など

### IPC 通信の最適化
ツール数の増加に伴い、IPC 通信の効率化が重要になります：
- バッチリクエスト対応
- ストリーミングレスポンス対応
- 非同期操作の並列化

### エラーハンドリング強化
各ツールカテゴリに特化したエラーハンドリングの実装：
- ビルドエラーの詳細レポート
- アセット操作の競合検出
- パフォーマンス測定の信頼性向上

## 関連ドキュメント

- [クイックスタートガイド](./quickstart.md) - 15分で始める Unity MCP Server
- [アーキテクチャ詳細](./unity_mcp_server_architecture_direct_ipc_variant.md) - Direct IPC アーキテクチャ
- [Rust MCP サーバー開発ガイド](./guide_building_a_streamable_http_mcp_server_in_rust_with_rmcp_macro_based.md) - rmcp マクロベースの実装

---

*このロードマップは Unity 開発コミュニティのニーズと技術的実現可能性を考慮して策定されており、定期的に見直しと更新が行われます。*

**全体進捗**: 11/101 ツール完了 (10.9%)