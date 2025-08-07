# タスク 011: モック Unity Editor 環境の設計と実装

## 目的
Unity Editor 依存を排し、テストコードのみで検証可能なモック環境を提供する。RuntimeテストでgRPCクライアントラッパーと双方向ストリーミング、再接続、エラーシナリオを再現できることを目標とする。

## 成果物
- `IUnityEditorApi` 相当の抽象インターフェース定義（Bridgeの公開APIから抽出）
- `MockUnityEditor` 実装（イベント発火、時間経過、エラー注入、状態保持）
- ストリーミング用のバックプレッシャー/バッチ/サンプリング設定可能なシミュレーター
- テストユーティリティ（タイムアウト/リトライ/仮想時間、Fixture）
- フックポイント: DI/ファクトリで実装差し替え可能

## 受入基準
- [ ] Editor 非起動で Runtime テストが全て通る
- [ ] モックで双方向ストリーミング、切断/再接続、エラー注入が再現可能
- [ ] gRPC-Web/HTTP2 いずれでも API 交換無しで動作
- [ ] CI（Linux headless）で実行可能
- [ ] ドキュメントに使用方法と拡張ポイントを記載

## 実装ノート
- インターフェースは最小集合から開始（ProjectInfo 取得、アセット列挙、イベント通知など）
- MainThreadDispatcher 依存を無効化し、代替の`IScheduler`インターフェースを導入
- イベントはチャネル/Queueで蓄積し、サンプリング/バッチ送出を切替可能
- 乱数シード固定で決定論的に

## 作成/変更するファイル
- `bridge/Packages/com.example.mcp-bridge/Runtime/Abstractions/IUnityEditorApi.cs`
- `bridge/Packages/com.example.mcp-bridge/Runtime/Abstractions/IScheduler.cs`
- `bridge/Packages/com.example.mcp-bridge/Tests/TestUtilities/MockUnityEditor.cs`
- `bridge/Packages/com.example.mcp-bridge/Tests/TestUtilities/VirtualTimeScheduler.cs`
- `bridge/Packages/com.example.mcp-bridge/Tests/Runtime/*`（既存テストをモック利用に移行）
- `docs/testing-mock-editor.md`（使用ガイド）

## 依存関係
- 必須: タスク005（Unity gRPC クライアントラッパーの公開API確定）
- 推奨: タスク007（ストリーミング仕様）、タスク010（再接続仕様）

## ブロック
- IUnityEditorApi 抽出が未完了の間は着手範囲を最小に限定

## 実装優先度
高 - テストのEditor非依存化とCI安定化に必須

## テスト要件
- モックを用いた Runtime 単体/結合テストで全シナリオを網羅
- 仮想時間でタイムアウト/バックオフ挙動を検証
- 負荷下（高頻度イベント）でバックプレッシャーの動作が確認できる
