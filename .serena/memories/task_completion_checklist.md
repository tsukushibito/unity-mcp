# Unity MCP Server - タスク完了時のチェックリスト

## Rust Server 開発完了時

### 必須チェック
1. **フォーマット確認**
   ```bash
   cd server
   cargo fmt --all -- --check
   ```

2. **Lint チェック**
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

3. **ビルド確認**
   ```bash
   cargo build
   ```

4. **テスト実行**
   ```bash
   cargo test
   ```

### CI/CD 確認
- GitHub Actions CI が成功することを確認
- 全ステップ（fmt → clippy → build → test）が通ることを確認

## Unity Bridge 開発完了時

### テスト実行
```bash
# Unity CLI でのテスト実行（将来実装予定）
Unity -quit -batchmode -projectPath bridge -runTests -testResults results.xml -testPlatform EditMode
```

### Unity Editor での確認
- エディター内でスクリプトエラーが出ていないことを確認
- コンパイルエラーがないことを確認

## 全般的なチェック

### コミット前
1. **変更内容の確認**
   ```bash
   git status
   git diff
   ```

2. **Conventional Commits 形式でコミット**
   ```bash
   git commit -m "feat: implement Unity MCP handler"
   # または
   git commit -m "fix: resolve connection timeout issue"
   ```

### プルリクエスト作成前
1. **ローカルでの最終テスト**
   - 全てのビルドコマンドが成功すること
   - テストが全て通ること

2. **ドキュメント更新**
   - 必要に応じて README や docs を更新
   - CLAUDE.md の更新（設定や手順の変更がある場合）

## 品質保証

### コードレビュー観点
- **Rust**: パニックの可能性がないか確認
- **エラーハンドリング**: 適切な `Result` 型の使用
- **非同期処理**: deadlock や競合状態がないか確認
- **Unity**: エディター/ランタイムでの動作確認

### パフォーマンス
- 不要なメモリ確保がないか確認
- I/O ブロックが適切に非同期化されているか確認

### セキュリティ
- 外部入力の適切なバリデーション
- ログに機密情報が含まれていないか確認