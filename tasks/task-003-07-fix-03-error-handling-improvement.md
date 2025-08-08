# Task 3.7 Fix 03: エラーハンドリング改善

## 概要
Task 3.7のストリーミングサービス実装で特定された不適切なエラーハンドリングを修正します。現在、すべてのエラーが`ImportAssetResponse`で返されている問題と、エラー詳細情報の不足を解決します。

## 優先度
**🟡 重要優先度** - ユーザビリティとデバッグ効率に重要な影響

## 実装時間見積もり
**2-3時間** - 集中作業時間

## 受け入れ基準

### エラーレスポンス要件
- [ ] 各メッセージタイプに応じた適切なエラーレスポンスの実装
- [ ] 汎用エラーレスポンスタイプの追加
- [ ] エラー詳細情報の充実
- [ ] 一貫したエラーコード体系の確立

### デバッグ支援要件
- [ ] エラートレーサビリティの改善
- [ ] 詳細なエラーコンテキストの提供
- [ ] ログレベルとメッセージの最適化

### クライアント対応要件
- [ ] 適切なエラーマッピングによるクライアント処理の改善
- [ ] エラー復旧のガイダンス提供
- [ ] エラーカテゴリーの明確化

## 技術的詳細

### 問題のあるコード

**ファイル**: `server/src/grpc/service.rs`  
**場所**: Lines 703-721, 733-746  

```rust
// 問題のコード - 全てImportAssetResponseでエラーを返している
StreamResponse {
    message: Some(stream_response::Message::ImportAsset(
        ImportAssetResponse {
            asset: None,
            error: Some(McpError {
                code: 3, // INVALID_ARGUMENT
                message: "Stream request message is empty".to_string(),
                details: "StreamRequest must contain a valid message".to_string(),
            }),
        },
    )),
}
```

### 修正アーキテクチャ

#### 1. エラータイプ分類システム
```rust
#[derive(Debug, Clone)]
pub enum StreamErrorType {
    InvalidRequest,
    ProcessingError,
    InternalError,
    ResourceExhausted,
    NotFound,
    ValidationError,
}

impl StreamErrorType {
    fn to_grpc_code(&self) -> i32 {
        match self {
            Self::InvalidRequest => 3,    // INVALID_ARGUMENT
            Self::ProcessingError => 13,  // INTERNAL
            Self::InternalError => 13,    // INTERNAL
            Self::ResourceExhausted => 8, // RESOURCE_EXHAUSTED
            Self::NotFound => 5,          // NOT_FOUND
            Self::ValidationError => 3,   // INVALID_ARGUMENT
        }
    }
}
```

#### 2. 統一エラーレスポンス生成
```rust
impl UnityMcpServiceImpl {
    fn create_error_response(
        error_type: StreamErrorType,
        message: &str,
        details: &str,
        request_type: Option<&str>,
    ) -> StreamResponse {
        let mcp_error = McpError {
            code: error_type.to_grpc_code(),
            message: message.to_string(),
            details: format!("{} | Context: {:?}", details, request_type),
        };

        match request_type {
            Some("import_asset") => StreamResponse {
                message: Some(stream_response::Message::ImportAsset(
                    ImportAssetResponse {
                        asset: None,
                        error: Some(mcp_error),
                    },
                )),
            },
            Some("move_asset") => StreamResponse {
                message: Some(stream_response::Message::MoveAsset(
                    MoveAssetResponse {
                        asset: None,
                        error: Some(mcp_error),
                    },
                )),
            },
            Some("delete_asset") => StreamResponse {
                message: Some(stream_response::Message::DeleteAsset(
                    DeleteAssetResponse {
                        success: false,
                        error: Some(mcp_error),
                    },
                )),
            },
            Some("refresh") => StreamResponse {
                message: Some(stream_response::Message::Refresh(
                    RefreshResponse {
                        success: false,
                        error: Some(mcp_error),
                    },
                )),
            },
            _ => {
                // 汎用エラーレスポンス - ImportAssetを使用するが適切にマークする
                StreamResponse {
                    message: Some(stream_response::Message::ImportAsset(
                        ImportAssetResponse {
                            asset: None,
                            error: Some(McpError {
                                code: error_type.to_grpc_code(),
                                message: format!("Generic stream error: {}", message),
                                details: format!("GENERIC_ERROR | {}", details),
                            }),
                        },
                    )),
                }
            }
        }
    }
}
```

#### 3. 個別メッセージタイプ用エラーハンドラー
```rust
impl UnityMcpServiceImpl {
    async fn handle_import_asset_stream_with_error_handling(
        service: &Arc<Self>,
        import_req: ImportAssetRequest,
    ) -> StreamResponse {
        debug!(asset_path = %import_req.asset_path, "Processing import_asset stream request");
        
        let request = Request::new(import_req);
        match service.import_asset(request).await {
            Ok(response) => {
                let import_response = response.into_inner();
                debug!("ImportAsset stream request completed successfully");
                StreamResponse {
                    message: Some(stream_response::Message::ImportAsset(import_response)),
                }
            }
            Err(status) => {
                warn!(
                    error_code = %status.code(),
                    error_message = %status.message(),
                    "ImportAsset stream request failed"
                );
                
                Self::create_error_response(
                    Self::map_grpc_status_to_error_type(&status),
                    &format!("ImportAsset operation failed: {}", status.message()),
                    &format!("gRPC status: {:?} | Details: {}", status.code(), status.message()),
                    Some("import_asset"),
                )
            }
        }
    }

    async fn handle_move_asset_stream_with_error_handling(
        service: &Arc<Self>,
        move_req: MoveAssetRequest,
    ) -> StreamResponse {
        debug!(
            src_path = %move_req.src_path,
            dst_path = %move_req.dst_path,
            "Processing move_asset stream request"
        );
        
        let request = Request::new(move_req);
        match service.move_asset(request).await {
            Ok(response) => {
                let move_response = response.into_inner();
                debug!("MoveAsset stream request completed successfully");
                StreamResponse {
                    message: Some(stream_response::Message::MoveAsset(move_response)),
                }
            }
            Err(status) => {
                warn!(
                    error_code = %status.code(),
                    error_message = %status.message(),
                    "MoveAsset stream request failed"
                );
                
                Self::create_error_response(
                    Self::map_grpc_status_to_error_type(&status),
                    &format!("MoveAsset operation failed: {}", status.message()),
                    &format!("gRPC status: {:?} | Details: {}", status.code(), status.message()),
                    Some("move_asset"),
                )
            }
        }
    }

    // 他のメッセージタイプについても同様の実装
}
```

#### 4. gRPCステータスマッピング
```rust
impl UnityMcpServiceImpl {
    fn map_grpc_status_to_error_type(status: &Status) -> StreamErrorType {
        match status.code() {
            tonic::Code::InvalidArgument => StreamErrorType::ValidationError,
            tonic::Code::NotFound => StreamErrorType::NotFound,
            tonic::Code::ResourceExhausted => StreamErrorType::ResourceExhausted,
            tonic::Code::FailedPrecondition => StreamErrorType::ValidationError,
            tonic::Code::Internal => StreamErrorType::InternalError,
            tonic::Code::Unavailable => StreamErrorType::ProcessingError,
            _ => StreamErrorType::InternalError,
        }
    }

    fn create_empty_message_error() -> StreamResponse {
        Self::create_error_response(
            StreamErrorType::InvalidRequest,
            "Empty stream request received",
            "StreamRequest must contain a valid message field",
            None,
        )
    }

    fn create_stream_processing_error(status: Status) -> StreamResponse {
        Self::create_error_response(
            Self::map_grpc_status_to_error_type(&status),
            &format!("Stream processing error: {}", status.message()),
            &format!("Stream handler encountered an error: {:?}", status),
            None,
        )
    }
}
```

#### 5. エラーコンテキスト追跡
```rust
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub request_id: String,
    pub timestamp: std::time::SystemTime,
    pub request_type: Option<String>,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(request_type: Option<String>) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now(),
            request_type,
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn add_info(&mut self, key: String, value: String) {
        self.additional_info.insert(key, value);
    }

    pub fn to_details_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "Error context serialization failed".to_string())
    }
}
```

#### 6. 構造化されたエラーログ
```rust
impl UnityMcpServiceImpl {
    fn log_stream_error(
        error_type: &StreamErrorType,
        message: &str,
        context: &ErrorContext,
        status: Option<&Status>,
    ) {
        let error_details = json!({
            "error_type": format!("{:?}", error_type),
            "message": message,
            "context": context,
            "grpc_status": status.map(|s| {
                json!({
                    "code": s.code() as i32,
                    "message": s.message(),
                })
            }),
        });

        match error_type {
            StreamErrorType::InternalError | StreamErrorType::ProcessingError => {
                error!(error_details = %error_details, "Stream processing error");
            }
            StreamErrorType::ValidationError | StreamErrorType::InvalidRequest => {
                warn!(error_details = %error_details, "Stream validation error");
            }
            StreamErrorType::ResourceExhausted => {
                warn!(error_details = %error_details, "Stream resource exhausted");
            }
            StreamErrorType::NotFound => {
                info!(error_details = %error_details, "Stream resource not found");
            }
        }
    }
}
```

## 実装計画

### Step 1: エラータイプシステムの構築
1. `StreamErrorType` enumの定義
2. gRPCステータスコードマッピング
3. エラーコンテキスト構造の実装

### Step 2: 統一エラーレスポンス生成
1. `create_error_response`メソッドの実装
2. メッセージタイプ別エラーレスポンス生成
3. 汎用エラーレスポンスの対応

### Step 3: 個別ハンドラーのエラー処理改善
1. 各メッセージハンドラーの修正
2. 適切なエラーマッピングの実装
3. エラーログの構造化

### Step 4: エラートレーサビリティの改善
1. エラーコンテキスト追跡
2. 詳細なログ出力
3. デバッグ情報の充実

## テスト方針

### ユニットテスト
```rust
#[tokio::test]
async fn test_error_response_mapping() {
    // エラータイプマッピングの確認
}

#[tokio::test]
async fn test_message_type_specific_errors() {
    // メッセージタイプ別エラーレスポンスの確認
}

#[tokio::test]
async fn test_error_context_tracking() {
    // エラーコンテキストの追跡確認
}
```

### エラーシナリオテスト
```rust
#[tokio::test]
async fn test_empty_message_error_handling() {
    // 空メッセージのエラー処理確認
}

#[tokio::test]
async fn test_invalid_request_error_handling() {
    // 無効なリクエストのエラー処理確認
}
```

## 検証方法

### エラーレスポンス検証
1. **適切なマッピング**
   - 各メッセージタイプでの正しいエラーレスポンス
   - エラーコードの一貫性
   - エラーメッセージの適切性

2. **デバッグ支援**
   - エラーログの詳細度
   - トレーサビリティの確認
   - コンテキスト情報の有用性

### クライアント側検証
1. **エラー処理**
   - クライアントでの適切なエラーハンドリング
   - エラー情報の有用性
   - 復旧処理の可能性

## 依存関係

### 前提条件
- Task 3.7 Fix 01 (メモリリーク脆弱性修正) の完了
- Task 3.7 Fix 02 (リソース管理改善) の完了

### ブロック対象
- Task 3.7 Fix 04 (コード品質改善)
- Task 3.7 Fix 05 (テストカバレッジ改善)

## リスク評価

### 中リスク
- **API契約**: エラーレスポンス形式の変更
- **ログ量**: 詳細なログによるパフォーマンス影響

### 低リスク
- **後方互換性**: 基本的なレスポンス構造は維持
- **実装複雑度**: 比較的単純な修正

## 緩和策

### API契約対策
- 段階的なロールアウト
- クライアント側の対応確認
- ドキュメント更新

### パフォーマンス対策
- ログレベルの適切な設定
- パフォーマンス監視
- 必要に応じた最適化

## 成功基準

### 定量的基準
- エラーレスポンス正確性: 100%
- 適切なエラーコードマッピング: 100%
- ログ情報の有用性向上

### 定性的基準
- デバッグ効率の向上
- クライアント側エラー処理の改善
- 運用監視の改善

## 次のステップ

修正完了後:
1. エラーシナリオテストの実行
2. クライアント側対応の確認
3. Task 3.7 Fix 04への移行

## 関連ドキュメント
- `reviews/task-3-7-streaming-service-review.md`
- `server/src/grpc/service.rs` (Lines 703-721, 733-746)
- gRPC error handling best practices
- Structured logging guidelines