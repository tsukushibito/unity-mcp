# M4 Smoke Checks - Unity MCP Bridge Manual Testing Guide

## 概要

このドキュメントは、Unity MCP Bridge の M4 (Tests & Smoke Checks) フェーズにおける手動スモークテスト手順を提供します。これらのテストは、dispatcher boundary が適切に実行され、handshake と health 機能がクロススレッド違反なしに動作することを検証します。

## 前提条件

- M1-M3 完了済み (EditorDispatcher, EditorStateMirror, MainThreadGuard, HealthHandler)
- Unity Editor 2022.3 LTS 以上
- Unity MCP Bridge プロジェクトが開かれている
- IPC サーバーが自動起動または手動起動可能な状態

## テスト環境設定

### 1. プロジェクト設定の確認

```bash
# Unity Editor でプロジェクトを開く
# File > Open Project > unity-mcp/bridge を選択
```

### 2. ログレベル設定

Unity Editor で Debug ログが表示されるよう設定：
- **Window > Console** を開く
- Console ウィンドウで **Log** チェックボックスが有効であることを確認
- **Editor Preferences > Console > Log Level** を **All** に設定（推奨）

### 3. Health モード設定の確認

現在のモード設定を確認：
- **Edit > Project Settings > Player > Scripting Define Symbols (Editor)** を確認
- `HEALTH_STRICT` が定義されている場合は **Strict Mode**
- 定義されていない場合は **Fast Mode** (デフォルト)

## 基本スモークテスト手順

### 重要: テスト手順の分類

**手動で実行可能**:
- ✅ Unity Editor起動確認
- ✅ ポート接続確認 (Test-NetConnection)
- ✅ ログ確認

**自動テスト推奨** (複雑なプロトコル処理のため):
- Hello→Welcome フロー → Unity Test Runner
- Health リクエスト → Unity Test Runner  
- エラー条件テスト → Unity Test Runner

### ステップ 1: Unity Editor 起動と Bridge 確認

1. **Unity Editor の起動**
   ```
   Unity Editor でプロジェクトを開く
   ```

2. **Bridge 自動起動の確認**
   - Console で `[BRIDGE]` タグのログを確認
   - `EditorIpcServer starting...` メッセージを探す
   - エラーがある場合は手動起動を試行

3. **手動 Bridge 起動**（必要に応じて）
   現在、手動起動用のメニューアイテムは実装されていません。
   サーバーは Unity Editor 起動時に InitializeOnLoadAttribute により自動起動されます。
   手動での再起動が必要な場合は、Unity Editor を再起動してください。
   
   **起動確認**: Console で以下のログを確認
   ```
   [TcpTransport] Started listening on 127.0.0.1:7777
   ```

4. **起動確認**
   - Console で `[BRIDGE.THREAD MAIN]` ログの出現を確認
   - `EditorStateMirror initialized` メッセージを確認
   - エラーメッセージがないことを確認

### ステップ 2: IPC クライアント接続テスト

1. **簡単なクライアント接続**
   - TCP ポート 7777 (デフォルト) への接続を試行
   - 手動テストツールまたは curl を使用：

   ```bash
   # 基本的な接続テスト
   # Linux/macOS
   telnet localhost 7777
   
   # Windows (PowerShell推奨)
   Test-NetConnection -ComputerName 127.0.0.1 -Port 7777
   ```

   **注意**: Test-NetConnection での接続テスト時、以下の警告が表示されますが正常動作です：
   ```
   [EditorIpcServer] Connection closed before handshake
   ```
   これは接続確認のみでハンドシェイクを行わないためです。

## 自動テスト実行（推奨）

### Unity Test Runner での実行

複雑なIPCプロトコルテストは Unity Test Runner での実行を推奨します。

1. **Unity Test Runner 起動**
   - **Window > General > Test Runner**

2. **EditMode テスト実行**
   - **CrossThreadDiagnosticsTests**: Health リクエストのクロススレッド安全性テスト
   - **HandshakeRefactorTests**: Hello→Welcome ハンドシェイクフローテスト  
   - **HealthTests**: Strict/Fast モード動作テスト

3. **テスト実行手順**
   ```
   1. Test Runner ウィンドウで「EditMode」タブを選択
   2. 各テストクラスを展開して個別テストを確認
   3. 「Run All」または個別テストを実行
   4. Console でテスト結果とログを確認
   ```

4. **期待される結果**
   - 全テストが PASS
   - `[BRIDGE.THREAD MAIN]` タグの出現
   - クロススレッド違反エラーが発生しない

### Rust 統合テスト実行（開発者向け）

server/ ディレクトリでの統合テスト実行方法：

1. **前提条件**
   - Unity Editor が起動しており、IPC サーバーが動作中
   - Rust 開発環境がセットアップ済み

2. **統合テスト実行**
   ```bash
   cd server/
   
   # 環境変数設定
   export MCP_IPC_TOKEN=test-token
   
   # 統合テスト実行
   cargo test test_assets_operations_end_to_end -- --nocapture
   
   # 全テスト実行
   cargo test -- --nocapture
   ```

3. **期待される結果**
   - IPC ハンドシェイクが成功
   - Health チェックが正常に応答
   - Unity Editor との通信が確立

## 手動テスト制限事項

### 接続確認のみ可能
- ✅ ポート7777の接続性確認
- ✅ サーバー起動状態確認
- ✅ 基本ログの確認

### プロトコルテストは自動テスト推奨
以下の理由により、手動実行は困難です：
- 複雑なバイナリプロトコル（Protocol Buffers）
- フレーミングプロトコル（長さプレフィックス）
- 認証トークン処理
- コリレーション ID 管理

### ステップ 4: スクリプト変更時の動作確認

1. **スクリプト編集の開始**
   - 任意の C# スクリプトファイルを編集
   - ファイルを保存してコンパイルを開始

2. **コンパイル中の Health チェック**
   - コンパイル中に Health リクエストを送信
   - `IsCompiling: true` が返されることを確認
   - レスポンス時間が合理的であることを確認

3. **コンパイル完了後の確認**
   - コンパイル完了後に Health リクエストを送信
   - `IsCompiling: false` が返されることを確認

## 詳細検証項目

### MainThreadGuard 動作確認

Console ログで以下のパターンを確認：

```
[BRIDGE.THREAD MAIN] EditorDispatcher: Action queued
[BRIDGE.THREAD MAIN] ValidateEditorState called
[BRIDGE.THREAD MAIN] CreateWelcome called
```

### Health モード別の動作確認

#### Fast Mode (デフォルト) での確認
- Health リクエストの応答時間が 10ms 未満
- EditorStateMirror からの値読み取り
- 高頻度リクエスト (10-20 req/sec) でも安定

#### Strict Mode での確認
- `HEALTH_STRICT` を Project Settings に追加
- プロジェクト再コンパイル待ち
- Health リクエストでメインスレッド実行確認
- 応答時間が Fast Mode より若干高い可能性

### エラー条件のテスト

1. **無効なトークン**
   - 無効なトークンで Hello リクエスト送信
   - Reject レスポンスの確認
   - Unity API アクセス前の拒否確認

2. **無効なバージョン**
   - 互換性のないバージョン (999.0) でリクエスト
   - 早期拒否の確認

## ストレステスト（オプション）

### High Load Health Test

```bash
# 10-20 req/sec で 60秒間テスト
for i in {1..1200}; do
  # Health リクエスト送信
  sleep 0.05  # 50ms 間隔
done
```

**期待される結果:**
- Unity Editor が応答不能にならない
- 全リクエストが正常に処理される
- メモリリークが発生しない
- Console にエラーが表示されない

### 長時間動作テスト

1. **継続的接続テスト**
   - 8時間以上の連続動作
   - 定期的な Health ポーリング (1分間隔)
   - Editor の安定性確認

## 受け入れ基準チェックリスト

### ✅ 基本機能
- [ ] Unity Editor 起動時に Bridge が自動起動される
- [ ] IPC クライアントが正常に接続できる
- [ ] Hello → Welcome フローが成功する
- [ ] Health リクエストが正常にレスポンスを返す

### ✅ スレッドセーフティ
- [ ] Console に `[BRIDGE.THREAD MAIN]` タグが表示される
- [ ] MainThreadGuard エラーが発生しない
- [ ] クロススレッド例外が発生しない

### ✅ モード別動作
- [ ] Fast Mode: 低レイテンシー応答 (< 10ms)
- [ ] Strict Mode: メインスレッド実行確認
- [ ] モード切り替えが正常に動作

### ✅ エラーハンドリング
- [ ] 無効トークン → 適切な拒否
- [ ] 無効バージョン → 適切な拒否
- [ ] ネットワークエラー → 適切な処理

### ✅ 性能・安定性
- [ ] 高負荷 (10-20 req/sec) でも安定動作
- [ ] Unity Editor の応答性が維持される
- [ ] メモリリークが発生しない
- [ ] 長時間動作でクラッシュしない

## トラブルシューティング

### よくある問題と解決方法

1. **Bridge が起動しない**
   - Console でエラーメッセージを確認
   - Port 7777 が他のプロセスに使用されていないか確認
   - Unity Editor を再起動

2. **MainThreadGuard エラーが発生**
   - `BRIDGE_THREAD_GUARD_STRICT` が設定されている可能性
   - Project Settings の Scripting Define Symbols を確認

3. **Health 応答が遅い**
   - Strict Mode の場合は Fast Mode に切り替え検討
   - システムリソース使用率を確認

4. **接続が不安定**
   - ファイアウォール設定を確認
   - localhost (127.0.0.1) 接続を試行
   - Unity Editor のバージョン互換性を確認

### Windows環境固有の注意事項

1. **ポート確認方法**
   ```cmd
   netstat -an | findstr 7777
   ```

2. **ファイアウォール設定**
   - Windows Defender ファイアウォールでローカルホスト通信が許可されていることを確認
   - 必要に応じて Unity Editor を例外に追加

3. **接続テスト**
   
   **Option A: telnetクライアント機能を有効化**
   ```cmd
   # Windows機能の有効化（管理者権限が必要）
   dism /online /Enable-Feature /FeatureName:TelnetClient
   
   # その後telnetコマンドが使用可能
   telnet 127.0.0.1 7777
   ```
   
   **Option B: PowerShellでの接続テスト**
   ```powershell
   # PowerShellでのTCP接続テスト
   Test-NetConnection -ComputerName 127.0.0.1 -Port 7777
   ```
   
   **Option C: curlでの接続テスト**
   ```cmd
   # curlでのTCP接続確認（Windows 10/11に標準搭載）
   curl -v telnet://127.0.0.1:7777
   ```

## ログ分析

### 正常動作時のログパターン

```log
[TcpTransport] Started listening on 127.0.0.1:7777
[BRIDGE.THREAD MAIN] EditorStateMirror initialized
[BRIDGE.THREAD MAIN] MainThreadGuard: Thread validation passed
[BRIDGE] Client connected from 127.0.0.1
[BRIDGE.THREAD MAIN] Hello request received, token validated
[BRIDGE.THREAD MAIN] Welcome response sent
[BRIDGE] Health request processed in 2ms
```

### 異常時のログパターン

```log
[BRIDGE.ERROR] MainThreadGuard: Cross-thread access detected!
[BRIDGE.ERROR] Invalid token in Hello request
[BRIDGE.ERROR] IPC transport error: Connection reset
```

## 報告書テンプレート

テスト完了後、以下の情報を含む報告書を作成：

```
## M4 Smoke Test Report

**実行日時:** [YYYY-MM-DD HH:MM]
**Unity バージョン:** [Unity Version]
**テスト環境:** [OS, Hardware specs]
**Health モード:** [Fast/Strict]

### テスト結果
- [ ] 基本機能: PASS/FAIL
- [ ] スレッドセーフティ: PASS/FAIL  
- [ ] 性能テスト: PASS/FAIL
- [ ] ストレステスト: PASS/FAIL

### 発見した問題
[問題の詳細, ログ, 再現手順]

### 推奨事項
[改善提案, 設定変更など]
```

この手順に従ってスモークテストを実行し、Unity MCP Bridge の M4 フェーズが適切に動作することを確認してください。