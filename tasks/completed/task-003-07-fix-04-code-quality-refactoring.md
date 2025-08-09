# Task 3.7 Fix 04: コード品質リファクタリング

## 概要
Task 3.7のストリーミングサービス実装で特定された巨大メソッド（250行超）と重複コードを修正します。保守性とテスタビリティを向上させるため、メソッドを適切なサイズに分割し、重複コードを排除します。

## 優先度
**🟡 重要優先度** - 保守性と開発効率に重要な影響

## 実装時間見積もり
**3-4時間** - 集中作業時間

## 受け入れ基準

### コード構造要件
- [ ] `stream`メソッドを100行以下に削減
- [ ] 各関数を単一責任に分割
- [ ] 重複コードを80%以上削減
- [ ] 循環的複雑度の改善（10以下）

### 保守性要件
- [ ] 各関数の責任範囲の明確化
- [ ] 適切な抽象化レベルの実現
- [ ] テストしやすい構造への変更
- [ ] ドキュメンテーションの充実

### 品質要件
- [ ] 既存機能の完全な動作保証
- [ ] コードの可読性向上
- [ ] パフォーマンスの維持または向上

## 技術的詳細

### 現在の問題

**ファイル**: `server/src/grpc/service.rs`  
**場所**: Lines 513-761 (248行の巨大メソッド)  

```rust
// 問題のコード - 248行の巨大なstreamメソッド
async fn stream(
    &self,
    request: Request<Streaming<StreamRequest>>,
) -> Result<Response<Self::StreamStream>, Status> {
    // 248行の複雑な処理...
}
```

### リファクタリング設計

#### 1. メイン`stream`メソッドのスリム化
```rust
impl UnityMcpService for UnityMcpServiceImpl {
    #[instrument(skip(self))]
    async fn stream(
        &self,
        request: Request<Streaming<StreamRequest>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        info!("Stream connection established");

        let stream_handler = StreamConnectionHandler::new(request)?;
        let response_stream = stream_handler.create_response_stream().await?;
        
        Ok(Response::new(response_stream))
    }
}
```

#### 2. 専用のストリーム接続ハンドラー
```rust
/// Handles the lifecycle and processing of a single stream connection
pub struct StreamConnectionHandler {
    incoming_stream: Streaming<StreamRequest>,
    response_sender: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    response_receiver: tokio::sync::mpsc::Receiver<Result<StreamResponse, Status>>,
    connection_id: String,
}

impl StreamConnectionHandler {
    pub fn new(
        request: Request<Streaming<StreamRequest>>,
    ) -> Result<Self, Status> {
        let incoming_stream = request.into_inner();
        let (response_sender, response_receiver) = tokio::sync::mpsc::channel(
            UnityMcpServiceImpl::STREAM_CHANNEL_CAPACITY
        );
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        info!(connection_id = %connection_id, "Creating new stream connection handler");
        
        Ok(Self {
            incoming_stream,
            response_sender,
            response_receiver,
            connection_id,
        })
    }

    pub async fn create_response_stream(self) -> Result<ServiceStream, Status> {
        // メッセージ処理タスクを開始
        let message_processor = StreamMessageProcessor::new(
            self.connection_id.clone(),
            Arc::new(UnityMcpServiceImpl::new()),
        );
        
        tokio::spawn(async move {
            message_processor.process_messages(
                self.incoming_stream,
                self.response_sender,
            ).await;
        });

        // レスポンスストリームを作成
        let response_stream = tokio_stream::wrappers::ReceiverStream::new(self.response_receiver);
        Ok(Box::pin(response_stream))
    }
}
```

#### 3. メッセージ処理の専用クラス
```rust
/// Processes individual stream messages and generates responses
pub struct StreamMessageProcessor {
    connection_id: String,
    service: Arc<UnityMcpServiceImpl>,
    message_counter: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl StreamMessageProcessor {
    pub fn new(connection_id: String, service: Arc<UnityMcpServiceImpl>) -> Self {
        Self {
            connection_id,
            service,
            message_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn process_messages(
        &self,
        mut incoming_stream: Streaming<StreamRequest>,
        response_sender: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        info!(connection_id = %self.connection_id, "Starting message processing");

        while let Some(result) = incoming_stream.next().await {
            let message_id = self.message_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            
            match result {
                Ok(stream_request) => {
                    let response = self.handle_stream_request(stream_request, message_id).await;
                    
                    if self.send_response(&response_sender, response, message_id).await.is_err() {
                        break;
                    }
                }
                Err(status) => {
                    let error_response = self.create_stream_error_response(status, message_id);
                    let _ = self.send_response(&response_sender, error_response, message_id).await;
                    break;
                }
            }
        }

        info!(connection_id = %self.connection_id, "Message processing completed");
    }

    async fn handle_stream_request(
        &self,
        stream_request: StreamRequest,
        message_id: u64,
    ) -> StreamResponse {
        debug!(
            connection_id = %self.connection_id,
            message_id = message_id,
            "Processing stream request"
        );

        match stream_request.message {
            Some(request_message) => {
                self.dispatch_message(request_message, message_id).await
            }
            None => {
                warn!(
                    connection_id = %self.connection_id,
                    message_id = message_id,
                    "Received stream request with no message content"
                );
                self.create_empty_message_error(message_id)
            }
        }
    }

    async fn dispatch_message(
        &self,
        request_message: stream_request::Message,
        message_id: u64,
    ) -> StreamResponse {
        match request_message {
            stream_request::Message::ImportAsset(req) => {
                ImportAssetStreamHandler::handle(&self.service, req, self.connection_id.clone(), message_id).await
            }
            stream_request::Message::MoveAsset(req) => {
                MoveAssetStreamHandler::handle(&self.service, req, self.connection_id.clone(), message_id).await
            }
            stream_request::Message::DeleteAsset(req) => {
                DeleteAssetStreamHandler::handle(&self.service, req, self.connection_id.clone(), message_id).await
            }
            stream_request::Message::Refresh(req) => {
                RefreshStreamHandler::handle(&self.service, req, self.connection_id.clone(), message_id).await
            }
        }
    }

    async fn send_response(
        &self,
        sender: &tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
        response: StreamResponse,
        message_id: u64,
    ) -> Result<(), ()> {
        match sender.send(Ok(response)).await {
            Ok(_) => {
                debug!(
                    connection_id = %self.connection_id,
                    message_id = message_id,
                    "Response sent successfully"
                );
                Ok(())
            }
            Err(_) => {
                warn!(
                    connection_id = %self.connection_id,
                    message_id = message_id,
                    "Failed to send response - receiver dropped"
                );
                Err(())
            }
        }
    }

    fn create_stream_error_response(&self, status: Status, message_id: u64) -> StreamResponse {
        warn!(
            connection_id = %self.connection_id,
            message_id = message_id,
            error_code = %status.code(),
            error_message = %status.message(),
            "Stream processing error"
        );

        UnityMcpServiceImpl::create_error_response(
            StreamErrorType::ProcessingError,
            &format!("Stream processing error: {}", status.message()),
            &format!("Connection: {} | Message: {} | Status: {:?}", 
                     self.connection_id, message_id, status),
            None,
        )
    }

    fn create_empty_message_error(&self, message_id: u64) -> StreamResponse {
        UnityMcpServiceImpl::create_error_response(
            StreamErrorType::InvalidRequest,
            "Empty stream request received",
            &format!("Connection: {} | Message: {} | StreamRequest must contain a valid message field", 
                     self.connection_id, message_id),
            None,
        )
    }
}
```

#### 4. 個別操作ハンドラーの抽象化
```rust
/// Generic trait for handling different types of stream requests
#[async_trait]
pub trait StreamRequestHandler<TRequest, TResponse> {
    async fn handle(
        service: &Arc<UnityMcpServiceImpl>,
        request: TRequest,
        connection_id: String,
        message_id: u64,
    ) -> StreamResponse;

    fn get_operation_name() -> &'static str;
    fn create_success_response(response: TResponse) -> StreamResponse;
    fn create_error_response(error: McpError) -> StreamResponse;
}

/// Handler for ImportAsset operations
pub struct ImportAssetStreamHandler;

#[async_trait]
impl StreamRequestHandler<ImportAssetRequest, ImportAssetResponse> for ImportAssetStreamHandler {
    async fn handle(
        service: &Arc<UnityMcpServiceImpl>,
        request: ImportAssetRequest,
        connection_id: String,
        message_id: u64,
    ) -> StreamResponse {
        debug!(
            connection_id = %connection_id,
            message_id = message_id,
            asset_path = %request.asset_path,
            operation = %Self::get_operation_name(),
            "Processing stream request"
        );

        let grpc_request = Request::new(request);
        match service.import_asset(grpc_request).await {
            Ok(response) => {
                let import_response = response.into_inner();
                debug!(
                    connection_id = %connection_id,
                    message_id = message_id,
                    operation = %Self::get_operation_name(),
                    "Operation completed successfully"
                );
                Self::create_success_response(import_response)
            }
            Err(status) => {
                warn!(
                    connection_id = %connection_id,
                    message_id = message_id,
                    operation = %Self::get_operation_name(),
                    error_code = %status.code(),
                    error_message = %status.message(),
                    "Operation failed"
                );

                let mcp_error = McpError {
                    code: status.code() as i32,
                    message: status.message().to_string(),
                    details: format!(
                        "Connection: {} | Message: {} | Operation: {}",
                        connection_id, message_id, Self::get_operation_name()
                    ),
                };
                Self::create_error_response(mcp_error)
            }
        }
    }

    fn get_operation_name() -> &'static str {
        "import_asset"
    }

    fn create_success_response(response: ImportAssetResponse) -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::ImportAsset(response)),
        }
    }

    fn create_error_response(error: McpError) -> StreamResponse {
        StreamResponse {
            message: Some(stream_response::Message::ImportAsset(
                ImportAssetResponse {
                    asset: None,
                    error: Some(error),
                },
            )),
        }
    }
}

// 他の操作タイプについても同様のハンドラーを実装
// MoveAssetStreamHandler, DeleteAssetStreamHandler, RefreshStreamHandler
```

#### 5. 重複コード排除のためのマクロ
```rust
/// Macro for generating boilerplate stream handler implementations
macro_rules! impl_stream_handler {
    (
        $handler_name:ident,
        $request_type:ty,
        $response_type:ty,
        $service_method:ident,
        $response_variant:ident,
        $operation_name:expr
    ) => {
        pub struct $handler_name;

        #[async_trait]
        impl StreamRequestHandler<$request_type, $response_type> for $handler_name {
            async fn handle(
                service: &Arc<UnityMcpServiceImpl>,
                request: $request_type,
                connection_id: String,
                message_id: u64,
            ) -> StreamResponse {
                debug!(
                    connection_id = %connection_id,
                    message_id = message_id,
                    operation = %Self::get_operation_name(),
                    "Processing stream request"
                );

                let grpc_request = Request::new(request);
                match service.$service_method(grpc_request).await {
                    Ok(response) => {
                        let inner_response = response.into_inner();
                        debug!(
                            connection_id = %connection_id,
                            message_id = message_id,
                            operation = %Self::get_operation_name(),
                            "Operation completed successfully"
                        );
                        Self::create_success_response(inner_response)
                    }
                    Err(status) => {
                        warn!(
                            connection_id = %connection_id,
                            message_id = message_id,
                            operation = %Self::get_operation_name(),
                            error_code = %status.code(),
                            error_message = %status.message(),
                            "Operation failed"
                        );

                        let mcp_error = McpError {
                            code: status.code() as i32,
                            message: status.message().to_string(),
                            details: format!(
                                "Connection: {} | Message: {} | Operation: {}",
                                connection_id, message_id, Self::get_operation_name()
                            ),
                        };
                        Self::create_error_response(mcp_error)
                    }
                }
            }

            fn get_operation_name() -> &'static str {
                $operation_name
            }

            fn create_success_response(response: $response_type) -> StreamResponse {
                StreamResponse {
                    message: Some(stream_response::Message::$response_variant(response)),
                }
            }

            fn create_error_response(error: McpError) -> StreamResponse {
                // Each operation type needs custom error response logic
                StreamResponse {
                    message: Some(stream_response::Message::$response_variant(
                        Self::create_error_response_inner(error)
                    )),
                }
            }
        }
    };
}

// マクロを使用してハンドラーを生成
impl_stream_handler!(
    MoveAssetStreamHandler,
    MoveAssetRequest,
    MoveAssetResponse,
    move_asset,
    MoveAsset,
    "move_asset"
);

impl_stream_handler!(
    DeleteAssetStreamHandler,
    DeleteAssetRequest,
    DeleteAssetResponse,
    delete_asset,
    DeleteAsset,
    "delete_asset"
);

impl_stream_handler!(
    RefreshStreamHandler,
    RefreshRequest,
    RefreshResponse,
    refresh,
    Refresh,
    "refresh"
);
```

## 実装計画

### Step 1: 構造の分析と設計
1. 現在の`stream`メソッドの責任分析
2. 適切な分割ポイントの特定
3. 新しいクラス構造の設計

### Step 2: 基盤クラスの実装
1. `StreamConnectionHandler`の実装
2. `StreamMessageProcessor`の実装
3. 基本的な抽象化の確立

### Step 3: 個別ハンドラーの実装
1. `StreamRequestHandler`トレイトの実装
2. 各操作タイプ用ハンドラーの実装
3. 重複コード排除マクロの実装

### Step 4: メインメソッドのリファクタリング
1. 元の`stream`メソッドの簡略化
2. 新しい構造への移行
3. テストの実行と修正

### Step 5: 最適化とドキュメント化
1. パフォーマンスの確認と最適化
2. ドキュメントの充実
3. コードレビューと最終調整

## テスト方針

### 単体テスト
```rust
#[tokio::test]
async fn test_stream_connection_handler_creation() {
    // StreamConnectionHandlerの作成テスト
}

#[tokio::test]
async fn test_message_processor_dispatch() {
    // メッセージディスパッチのテスト
}

#[tokio::test]
async fn test_individual_stream_handlers() {
    // 個別ハンドラーのテスト
}
```

### 統合テスト
```rust
#[tokio::test]
async fn test_refactored_stream_functionality() {
    // リファクタリング後の全体機能テスト
}

#[tokio::test]
async fn test_stream_error_handling_after_refactor() {
    // エラーハンドリングの統合テスト
}
```

### 回帰テスト
```rust
#[tokio::test]
async fn test_stream_behavior_unchanged() {
    // 既存動作の保証テスト
}
```

## 検証方法

### コード品質メトリクス
1. **循環的複雑度**
   - 各関数の複雑度測定
   - 10以下の目標達成確認
   - 全体的な複雑度の改善

2. **メソッドサイズ**
   - 各メソッドの行数確認
   - 100行以下の目標達成
   - 責任範囲の適切性

3. **重複コード**
   - 重複率の測定
   - 80%以上削減の確認
   - DRY原則の遵守

### 保守性評価
1. **テスタビリティ**
   - 単体テストの作成容易性
   - モッキングの可能性
   - テストカバレッジの向上

2. **可読性**
   - コードレビューでの理解度
   - ドキュメント化の充実
   - 新規開発者の理解速度

## 依存関係

### 前提条件
- Task 3.7 Fix 01 (メモリリーク脆弱性修正) の完了
- Task 3.7 Fix 02 (リソース管理改善) の完了
- Task 3.7 Fix 03 (エラーハンドリング改善) の完了

### ブロック対象
- Task 3.7 Fix 05 (テストカバレッジ改善)
- Task 3.7 Fix 06 (パフォーマンス最適化)

## リスク評価

### 高リスク
- **大規模リファクタリング**: 既存機能への影響
- **複雑な依存関係**: 変更の波及効果

### 中リスク
- **抽象化レベル**: 適切でない抽象化による複雑化
- **パフォーマンス**: 新しい構造による性能劣化

### 低リスク
- **テスト容易性**: 改善されたテスト環境
- **保守性**: 長期的な保守の改善

## 緩和策

### 大規模変更対策
- 段階的なリファクタリング
- 既存テストの継続実行
- ロールバック計画の準備

### パフォーマンス対策
- ベンチマークテストの実行
- プロファイリングによる確認
- 最適化ポイントの特定

## 成功基準

### 定量的基準
- メソッドサイズ: 100行以下
- 循環的複雑度: 10以下
- 重複コード削減: 80%以上

### 定性的基準
- コードの可読性向上
- テスタビリティの改善
- 保守性の向上

## 次のステップ

リファクタリング完了後:
1. 全体テストスイートの実行
2. パフォーマンス回帰テスト
3. Task 3.7 Fix 05への移行

## 関連ドキュメント
- `reviews/task-3-7-streaming-service-review.md`
- `server/src/grpc/service.rs` (Lines 513-761)
- Rust refactoring best practices
- Clean Code principles