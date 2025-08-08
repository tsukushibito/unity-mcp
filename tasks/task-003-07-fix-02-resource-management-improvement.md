# Task 3.7 Fix 02: リソース管理改善

## 概要
Task 3.7のストリーミングサービス実装で特定されたリソース管理不備を修正します。各リクエスト処理で新しいサービスインスタンスを生成している問題と、タスクライフサイクル管理の不備を解決します。

## 優先度
**🔴 高優先度** - メモリ効率とシステム安定性に直接影響

## 実装時間見積もり
**2-3時間** - 集中作業時間

## 受け入れ基準

### リソース効率要件
- [ ] サービスインスタンスの重複生成排除
- [ ] `Arc<Self>`による適切なインスタンス共有
- [ ] タスクハンドルの適切な管理
- [ ] リソースリークの防止

### ライフサイクル管理要件
- [ ] ストリーム終了時の適切なクリーンアップ
- [ ] タスクキャンセレーション機能の実装
- [ ] グレースフルシャットダウンの対応

### パフォーマンス要件
- [ ] メモリ使用量の削減（30%以上）
- [ ] 処理速度の向上または維持
- [ ] CPUオーバーヘッドの削減

## 技術的詳細

### 問題のあるコード
**ファイル**: `server/src/grpc/service.rs`  
**場所**: Lines 536, 580, 622, 664  

```rust
// 問題のコード - 各リクエストで新規作成
let service = UnityMcpServiceImpl::new();
let request = Request::new(import_req);
```

### 修正アーキテクチャ

#### 1. サービスインスタンス共有
```rust
impl UnityMcpService for UnityMcpServiceImpl {
    async fn stream(
        &self,
        request: Request<Streaming<StreamRequest>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        info!("Stream connection established");

        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(Self::STREAM_CHANNEL_CAPACITY);
        
        // selfの共有Arcを作成
        let service = Arc::new(UnityMcpServiceImpl::new());
        let service_clone = Arc::clone(&service);
        
        // タスクハンドルを保持
        let handle = tokio::spawn(async move {
            // service_cloneを使用してリクエスト処理
            Self::handle_stream_messages(service_clone, stream, tx).await;
        });

        // ストリーム終了時のクリーンアップ設定
        let cleanup_handle = tokio::spawn(async move {
            let _ = handle.await;
            info!("Stream handler task completed");
        });

        Ok(Response::new(Box::pin(rx)))
    }
}
```

#### 2. 共有インスタンス処理ハンドラー
```rust
impl UnityMcpServiceImpl {
    async fn handle_stream_messages(
        service: Arc<Self>,
        mut stream: Streaming<StreamRequest>,
        tx: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        while let Some(result) = stream.next().await {
            match result {
                Ok(stream_request) => {
                    let response = Self::process_stream_request(&service, stream_request).await;
                    
                    if tx.send(Ok(response)).await.is_err() {
                        warn!("Stream receiver dropped - terminating handler");
                        break;
                    }
                }
                Err(status) => {
                    warn!("Stream request error: {}", status.message());
                    let error_response = Self::create_stream_error_response(status);
                    let _ = tx.send(Ok(error_response)).await;
                    break;
                }
            }
        }
        
        info!("Stream message handler terminated");
    }
}
```

#### 3. 統一されたリクエスト処理
```rust
impl UnityMcpServiceImpl {
    async fn process_stream_request(
        service: &Arc<Self>,
        stream_request: StreamRequest,
    ) -> StreamResponse {
        debug!("Processing stream request");
        
        match stream_request.message {
            Some(request_message) => {
                Self::handle_request_message(service, request_message).await
            }
            None => {
                warn!("Received stream request with no message content");
                Self::create_empty_message_error()
            }
        }
    }

    async fn handle_request_message(
        service: &Arc<Self>,
        request_message: stream_request::Message,
    ) -> StreamResponse {
        match request_message {
            stream_request::Message::ImportAsset(import_req) => {
                Self::handle_import_asset_stream(service, import_req).await
            }
            stream_request::Message::MoveAsset(move_req) => {
                Self::handle_move_asset_stream(service, move_req).await
            }
            stream_request::Message::DeleteAsset(delete_req) => {
                Self::handle_delete_asset_stream(service, delete_req).await
            }
            stream_request::Message::Refresh(refresh_req) => {
                Self::handle_refresh_stream(service, refresh_req).await
            }
        }
    }
}
```

#### 4. 個別メッセージハンドラー
```rust
impl UnityMcpServiceImpl {
    async fn handle_import_asset_stream(
        service: &Arc<Self>,
        import_req: ImportAssetRequest,
    ) -> StreamResponse {
        debug!(asset_path = %import_req.asset_path, "Processing import_asset stream request");
        
        let request = Request::new(import_req);
        match service.import_asset(request).await {
            Ok(response) => {
                let import_response = response.into_inner();
                StreamResponse {
                    message: Some(stream_response::Message::ImportAsset(import_response)),
                }
            }
            Err(status) => {
                warn!("ImportAsset stream request failed: {}", status.message());
                Self::create_import_error_response(status)
            }
        }
    }

    // 他のメッセージタイプについても同様のハンドラーを実装
}
```

### タスクライフサイクル管理

#### 1. 構造化されたハンドル管理
```rust
pub struct StreamHandler {
    message_handler: JoinHandle<()>,
    cleanup_handler: JoinHandle<()>,
    cancellation_token: CancellationToken,
}

impl StreamHandler {
    fn new(
        service: Arc<UnityMcpServiceImpl>,
        stream: Streaming<StreamRequest>,
        tx: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) -> Self {
        let cancellation_token = CancellationToken::new();
        
        let message_handler = tokio::spawn({
            let cancellation_token = cancellation_token.clone();
            async move {
                tokio::select! {
                    _ = UnityMcpServiceImpl::handle_stream_messages(service, stream, tx) => {
                        info!("Stream message processing completed normally");
                    }
                    _ = cancellation_token.cancelled() => {
                        info!("Stream message processing cancelled");
                    }
                }
            }
        });

        let cleanup_handler = tokio::spawn({
            let message_handler = message_handler.clone();
            async move {
                let _ = message_handler.await;
                info!("Stream cleanup completed");
            }
        });

        Self {
            message_handler,
            cleanup_handler,
            cancellation_token,
        }
    }

    async fn shutdown(self) -> Result<(), tokio::task::JoinError> {
        self.cancellation_token.cancel();
        self.message_handler.await?;
        self.cleanup_handler.await?;
        Ok(())
    }
}
```

## 実装計画

### Step 1: サービス共有アーキテクチャの実装
1. `Arc<Self>`による共有インスタンス作成
2. 既存のインスタンス生成箇所の特定と削除
3. 新しい共有メカニズムの実装

### Step 2: 統合されたメッセージハンドリング
1. 共通のリクエスト処理関数の実装
2. 個別メッセージハンドラーの分離
3. エラーハンドリングの統一

### Step 3: ライフサイクル管理の改善
1. タスクハンドル管理の実装
2. キャンセレーション機能の追加
3. グレースフルシャットダウンの対応

### Step 4: リソースクリーンアップの実装
1. ストリーム終了時の処理
2. メモリリーク防止機能
3. 監視とロギングの改善

## テスト方針

### ユニットテスト
```rust
#[tokio::test]
async fn test_service_instance_sharing() {
    // インスタンス共有の確認
}

#[tokio::test]
async fn test_task_lifecycle_management() {
    // タスクライフサイクルの確認
}

#[tokio::test]
async fn test_resource_cleanup() {
    // リソースクリーンアップの確認
}
```

### メモリリーク検証
```rust
#[tokio::test]
async fn test_memory_usage_optimization() {
    // メモリ使用量の改善確認
}

#[tokio::test]
async fn test_concurrent_stream_handling() {
    // 並行ストリーム処理の確認
}
```

## 検証方法

### パフォーマンス測定
1. **メモリ使用量**
   - 修正前後のメモリプロファイリング
   - 長時間運用でのメモリリーク確認
   - 並行ストリーム処理時の使用量

2. **処理速度**
   - リクエスト処理時間の測定
   - スループットの比較
   - レイテンシーへの影響評価

### リソース管理検証
1. **インスタンス管理**
   - 重複生成の排除確認
   - 共有メカニズムの動作確認
   - ライフサイクル管理の適切性

2. **クリーンアップ**
   - 正常終了時のクリーンアップ
   - 異常終了時の処理
   - リソースリークの確認

## 依存関係

### 前提条件
- Task 3.7 Fix 01 (メモリリーク脆弱性修正) の完了

### ブロック対象
- Task 3.7 Fix 03 (エラーハンドリング改善)
- Task 3.7 Fix 04 (コード品質改善)

## リスク評価

### 高リスク
- **アーキテクチャ変更**: 既存の動作への影響
- **並行性**: 共有インスタンスでのスレッドセーフティ

### 中リスク
- **複雑度**: 実装の複雑さ増加
- **デバッグ**: 問題の特定が困難になる可能性

### 低リスク
- **後方互換性**: API変更なし
- **テスト**: 明確な検証方法

## 緩和策

### アーキテクチャ変更対策
- 段階的な実装とテスト
- 既存機能の回帰テスト充実
- ロールバック計画の準備

### 並行性対策
- スレッドセーフティの確認
- 競合状態のテスト
- 適切な同期プリミティブの使用

## 成功基準

### 定量的基準
- メモリ使用量削減: 30%以上
- 処理速度: 現状維持または向上
- リソースリーク: ゼロ

### 定性的基準
- コードの可読性向上
- 保守性の改善
- システムの安定性向上

## 次のステップ

修正完了後:
1. パフォーマンス測定の実行
2. メモリプロファイリングの確認
3. Task 3.7 Fix 03への移行

## 関連ドキュメント
- `reviews/task-3-7-streaming-service-review.md`
- `server/src/grpc/service.rs` (Lines 536, 580, 622, 664)
- Tokio task management documentation
- Arc sharing patterns in Rust