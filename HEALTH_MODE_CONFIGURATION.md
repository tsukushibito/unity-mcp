# Health Handler Mode Configuration

M3-B Health Refactorで実装されたHealthHandlerは2つのモードをサポートします：

## モード設定

### 1. Fast Mode (デフォルト)
- **説明**: `EditorStateMirror`から直接状態を読み取り、最小レイテンシー
- **設定**: `HEALTH_STRICT`を**定義しない**（デフォルト状態）
- **特徴**: 
  - バックグラウンドスレッドから安全に実行可能
  - 最小レイテンシー
  - Eventually consistent（最終的整合性）

### 2. Strict Mode
- **説明**: `EditorDispatcher.RunOnMainAsync`を使用してメインスレッドで実行
- **設定**: Project Settings → Player → Scripting Define Symbolsに`HEALTH_STRICT`を追加
- **特徴**:
  - 最強の正確性保証
  - 負荷時には若干高いレイテンシー
  - メインスレッド実行によるUnity APIの直接アクセス

## 設定手順

### Strict Modeを有効にする場合：
1. Unity Editorで **Edit → Project Settings** を開く
2. **Player** セクションを選択
3. **Settings for PC, Mac & Linux Standalone** を展開
4. **Scripting Define Symbols** フィールドに `HEALTH_STRICT` を追加
   - 既存のシンボルがある場合はセミコロン（`;`）で区切る
   - 例: `EXISTING_SYMBOL;HEALTH_STRICT`
5. **Apply** をクリック
6. プロジェクトが自動的に再コンパイルされます

### Fast Modeに戻す場合：
1. 上記手順で **Scripting Define Symbols** から `HEALTH_STRICT` を削除
2. **Apply** をクリック

## 推奨運用

- **通常運用**: Fast Mode（デフォルト）を使用
  - Mirror による0.5-1.0秒のHealth polling推奨
  - クライアント側でdebouncing/throttling実装
- **検証・トラブルシューティング**: Strict Modeを使用
  - 最強の正確性が必要な場合
  - デバッグ時の状態確認

## テスト

両モードで動作確認：
- Fast Mode: Editorのコンパイル状態変更後に複数リクエストでmirror変更を観察
- Strict Mode: Handler実行がメインスレッド（guard/logs確認）、空でないバージョン応答