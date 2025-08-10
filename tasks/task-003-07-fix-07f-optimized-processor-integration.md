# Task 3.7 Fix 07-F: 最適化プロセッサー統合

## 概要
全ての最適化コンポーネントを統合したOptimizedStreamProcessorを実装し、既存のstreamメソッドを段階的に置き換えます。パフォーマンス監視、リソースプール、キャッシュ、ワーカープールを組み合わせて、目標とするパフォーマンス指標を達成します。

## 優先度
**🔴 最高優先度** - 全最適化の統合とパフォーマンス目標達成

## 実装時間見積もり
**90分** - 集中作業時間

## 依存関係
- Task 3.7 Fix 07-A〜E 全て完了必須

## 受け入れ基準

### 統合要件
- [ ] 全最適化コンポーネントの統合完了
- [ ] 既存streamメソッドの段階的置き換え
- [ ] 後方互換性の維持
- [ ] 既存テストスイートの100%パス

### パフォーマンス要件
- [ ] スループット: 1000 → 2000 req/s 達成
- [ ] レイテンシー: P95 50ms → 25ms以下
- [ ] メモリ使用量: 30%削減
- [ ] 同時接続数: 100接続安定処理

### 安定性要件
- [ ] 高負荷状態でのシステム安定性
- [ ] グレースフルデグラデーション
- [ ] エラー率1%以下維持
- [ ] メモリリーク完全防止

### 監視・デバッグ要件
- [ ] 統合パフォーマンス監視
- [ ] 詳細メトリクス収集
- [ ] 最適化効果の定量的測定
- [ ] トラブルシューティング機能

## 技術的詳細

### OptimizedStreamProcessor 統合実装

#### src/grpc/performance/processor.rs
```rust
//! 最適化ストリーミングプロセッサー統合
//! 
//! 全ての最適化コンポーネントを統合し、Unity MCP Server の
//! ストリーミング処理において最大限のパフォーマンス向上を実現する。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::sync::{mpsc, Semaphore};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;
use tracing::{debug, info, warn, error, instrument};

use crate::unity::{StreamRequest, StreamResponse, stream_response};
use crate::grpc::service::UnityMcpServiceImpl;
use crate::grpc::performance::{
    OptimizationConfig, OptimizationContext, OptimizationResult, OptimizationError,
    monitor::StreamPerformanceMonitor,
    resource_pool::ResourcePool,
    cache::StreamCache,
    worker_pool::{WorkerPool, TaskPriority, TaskContext},
};

/// 最適化ストリーミングプロセッサー
pub struct OptimizedStreamProcessor {
    // コア最適化コンポーネント
    performance_monitor: Arc<StreamPerformanceMonitor>,
    resource_pool: Arc<ResourcePool>,
    cache_system: Arc<StreamCache>,
    worker_pool: Arc<WorkerPool>,
    
    // 設定とコンテキスト
    config: OptimizationConfig,
    
    // バックプレッシャー制御
    connection_limiter: Arc<Semaphore>,
    backpressure_controller: Arc<BackpressureController>,
    
    // 統計と監視
    processor_stats: Arc<Mutex<ProcessorStatistics>>,
    optimization_metrics: Arc<Mutex<OptimizationMetrics>>,
    
    // フィーチャーフラグ
    feature_flags: FeatureFlags,
}

/// プロセッサー統計
#[derive(Debug, Default, Clone)]
pub struct ProcessorStatistics {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_messages_processed: u64,
    pub cache_hit_rate: f64,
    pub worker_pool_utilization: f64,
    pub avg_end_to_end_latency: Duration,
    pub backpressure_events: u64,
    pub optimization_effectiveness: f64,
}

/// 最適化メトリクス
#[derive(Debug, Default, Clone)]
pub struct OptimizationMetrics {
    pub throughput_improvement: f64,      // 改善倍率
    pub latency_reduction: f64,           // 削減率
    pub memory_efficiency_gain: f64,      // 効率化率
    pub cpu_utilization_optimization: f64,
    pub overall_performance_score: f64,   // 総合スコア
}

/// バックプレッシャー制御
#[derive(Debug)]
pub struct BackpressureController {
    current_load: Arc<Mutex<f64>>,
    thresholds: BackpressureThresholds,
    adaptive_settings: Arc<Mutex<AdaptiveBackpressureSettings>>,
}

/// バックプレッシャー閾値
#[derive(Debug, Clone)]
pub struct BackpressureThresholds {
    pub warning_threshold: f64,    // 0.7
    pub critical_threshold: f64,   // 0.8
    pub emergency_threshold: f64,  // 0.9
}

/// 適応的バックプレッシャー設定
#[derive(Debug, Clone)]
pub struct AdaptiveBackpressureSettings {
    pub current_limit: f64,
    pub adjustment_factor: f64,
    pub last_adjustment: Instant,
}

/// フィーチャーフラグ
#[derive(Debug, Clone)]
pub struct FeatureFlags {
    pub enable_caching: bool,
    pub enable_batching: bool,
    pub enable_compression: bool,
    pub enable_adaptive_optimization: bool,
    pub enable_detailed_metrics: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_caching: true,
            enable_batching: true,
            enable_compression: true,
            enable_adaptive_optimization: true,
            enable_detailed_metrics: false, // パフォーマンス重視
        }
    }
}

/// 処理タスク（統合版）
#[derive(Debug)]
pub struct OptimizedProcessingTask {
    pub task_id: Uuid,
    pub connection_id: String,
    pub message_id: u64,
    pub request: StreamRequest,
    pub submitted_at: Instant,
    pub optimization_context: OptimizationContext,
    pub priority: TaskPriority,
}

impl OptimizedStreamProcessor {
    /// 新しい最適化プロセッサーを作成
    pub fn new(config: OptimizationConfig) -> Self {
        info!("Initializing optimized stream processor");

        // コンポーネント初期化
        let performance_monitor = Arc::new(StreamPerformanceMonitor::new());
        let resource_pool = Arc::new(ResourcePool::with_config(
            config.to_resource_pool_config()
        ));
        let cache_system = Arc::new(StreamCache::with_config(
            config.to_cache_config()
        ));
        let worker_pool = Arc::new(
            WorkerPool::with_config(config.to_worker_pool_config())
                .with_performance_monitor(Arc::clone(&performance_monitor))
        );

        // バックプレッシャー制御
        let connection_limiter = Arc::new(Semaphore::new(config.max_concurrent_connections));
        let backpressure_controller = Arc::new(BackpressureController::new(
            BackpressureThresholds {
                warning_threshold: config.backpressure_threshold * 0.8,
                critical_threshold: config.backpressure_threshold,
                emergency_threshold: config.backpressure_threshold * 1.2,
            }
        ));

        let processor = Self {
            performance_monitor,
            resource_pool,
            cache_system,
            worker_pool,
            config: config.clone(),
            connection_limiter,
            backpressure_controller,
            processor_stats: Arc::new(Mutex::new(ProcessorStatistics::default())),
            optimization_metrics: Arc::new(Mutex::new(OptimizationMetrics::default())),
            feature_flags: FeatureFlags::default(),
        };

        // 統計更新タスクを開始
        processor.start_statistics_update_tasks();

        info!("Optimized stream processor initialized successfully");
        processor
    }

    /// 最適化ストリーミング処理のメインエントリーポイント
    #[instrument(skip(self, incoming_stream))]
    pub async fn process_stream_optimized(
        &self,
        mut incoming_stream: Streaming<StreamRequest>,
        response_sender: mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        let connection_id = Uuid::new_v4().to_string();
        let session_start = Instant::now();
        
        // 接続制限チェック
        let _connection_permit = match self.connection_limiter.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                error!(connection_id = %connection_id, "Connection limit exceeded");
                let _ = response_sender.send(Err(Status::resource_exhausted("Too many connections"))).await;
                return;
            }
        };

        info!(connection_id = %connection_id, "Starting optimized stream processing");

        // 最適化コンテキスト初期化
        let mut optimization_context = OptimizationContext::new(
            connection_id.clone(),
            self.config.clone()
        );

        // バッチング用バッファ
        let mut batch_buffer = Vec::with_capacity(self.config.batch_size);
        let mut message_counter = 0u64;
        let mut last_batch_flush = Instant::now();

        // 統計更新
        self.increment_active_connections().await;

        // メインストリーミングループ
        while let Some(result) = incoming_stream.next().await {
            match result {
                Ok(stream_request) => {
                    message_counter += 1;
                    optimization_context.increment_message_count();

                    // バックプレッシャーチェック
                    if self.should_apply_backpressure().await {
                        self.handle_backpressure(&response_sender, &connection_id).await;
                        continue;
                    }

                    // キャッシュチェック（有効な場合）
                    if self.feature_flags.enable_caching {
                        if let Some(cached_response) = self.cache_system.get(&stream_request).await {
                            debug!(
                                connection_id = %connection_id,
                                message_id = message_counter,
                                "Cache hit - returning cached response"
                            );
                            
                            if let Err(_) = response_sender.send(Ok(cached_response)).await {
                                warn!("Response channel closed - client disconnected");
                                break;
                            }
                            continue;
                        }
                    }

                    // バッチ処理（有効な場合）
                    if self.feature_flags.enable_batching && self.should_batch_request(&stream_request) {
                        let task = OptimizedProcessingTask {
                            task_id: Uuid::new_v4(),
                            connection_id: connection_id.clone(),
                            message_id: message_counter,
                            request: stream_request,
                            submitted_at: Instant::now(),
                            optimization_context: optimization_context.clone(),
                            priority: self.determine_task_priority(&stream_request),
                        };

                        batch_buffer.push(task);

                        // バッチサイズまたはタイムアウトでフラッシュ
                        let should_flush = batch_buffer.len() >= self.config.batch_size ||
                                          last_batch_flush.elapsed() >= self.config.max_batch_wait;

                        if should_flush {
                            self.process_batch_optimized(
                                std::mem::take(&mut batch_buffer),
                                &response_sender
                            ).await;
                            last_batch_flush = Instant::now();
                        }
                    } else {
                        // 直接処理
                        self.process_single_request_optimized(
                            stream_request,
                            message_counter,
                            &connection_id,
                            &optimization_context,
                            &response_sender,
                        ).await;
                    }
                }
                Err(status) => {
                    warn!(
                        connection_id = %connection_id,
                        error = %status,
                        "Stream error encountered"
                    );

                    // 残りバッチを処理してからエラーレスポンス
                    if !batch_buffer.is_empty() {
                        self.process_batch_optimized(batch_buffer, &response_sender).await;
                    }

                    let error_response = self.create_stream_error_response(status);
                    let _ = response_sender.send(Ok(error_response)).await;
                    break;
                }
            }
        }

        // 残りバッファを処理
        if !batch_buffer.is_empty() {
            self.process_batch_optimized(batch_buffer, &response_sender).await;
        }

        // セッション完了処理
        let session_duration = session_start.elapsed();
        self.performance_monitor.record_stream_session(
            connection_id.clone(),
            message_counter,
            session_duration,
            optimization_context.calculate_total_bytes_processed(),
        );

        self.decrement_active_connections().await;

        info!(
            connection_id = %connection_id,
            message_count = message_counter,
            duration = ?session_duration,
            "Optimized stream processing completed"
        );
    }

    /// 単一リクエストの最適化処理
    #[instrument(skip(self, request, response_sender))]
    async fn process_single_request_optimized(
        &self,
        request: StreamRequest,
        message_id: u64,
        connection_id: &str,
        optimization_context: &OptimizationContext,
        response_sender: &mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        let processing_start = Instant::now();
        
        // タスクコンテキスト作成
        let task_context = TaskContext {
            connection_id: connection_id.to_string(),
            message_id,
            batch_id: None,
            deadline: Some(processing_start + self.config.stream_timeout),
        };

        // ワーカープールで処理
        let priority = self.determine_task_priority(&request);
        match self.worker_pool.execute(request.clone(), priority, task_context).await {
            Ok(response) => {
                // キャッシュに保存（可能であれば）
                if self.feature_flags.enable_caching {
                    self.cache_system.put(&request, response.clone()).await;
                }

                if let Err(_) = response_sender.send(Ok(response)).await {
                    warn!("Response channel closed during single request processing");
                }
            }
            Err(error) => {
                error!(
                    connection_id = connection_id,
                    message_id = message_id,
                    error = %error,
                    "Single request processing failed"
                );

                let error_response = self.create_processing_error_response(error);
                let _ = response_sender.send(Ok(error_response)).await;
            }
        }

        // 処理時間記録
        let processing_time = processing_start.elapsed();
        self.performance_monitor.record_single_request_processing(processing_time);

        debug!(
            connection_id = connection_id,
            message_id = message_id,
            processing_time = ?processing_time,
            "Single request processing completed"
        );
    }

    /// バッチの最適化処理
    #[instrument(skip(self, batch, response_sender))]
    async fn process_batch_optimized(
        &self,
        batch: Vec<OptimizedProcessingTask>,
        response_sender: &mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        if batch.is_empty() {
            return;
        }

        let batch_id = Uuid::new_v4();
        let batch_size = batch.len();
        let batch_start = Instant::now();

        debug!(
            batch_id = %batch_id,
            batch_size = batch_size,
            "Processing optimized batch"
        );

        // バッチを並列処理用のタスクに変換
        let processing_tasks: Vec<_> = batch.into_iter()
            .map(|task| {
                let task_context = TaskContext {
                    connection_id: task.connection_id,
                    message_id: task.message_id,
                    batch_id: Some(batch_id),
                    deadline: Some(task.submitted_at + self.config.stream_timeout),
                };
                (task.request, task.priority, task_context)
            })
            .collect();

        // 並列実行
        let mut futures = Vec::new();
        for (request, priority, context) in processing_tasks {
            let worker_pool = Arc::clone(&self.worker_pool);
            let future = async move {
                worker_pool.execute(request, priority, context).await
            };
            futures.push(future);
        }

        // 全てのタスク完了を待機
        let results = futures::future::join_all(futures).await;

        // 結果をレスポンスチャネルに送信
        for result in results {
            match result {
                Ok(response) => {
                    if let Err(_) = response_sender.send(Ok(response)).await {
                        warn!("Response channel closed during batch processing");
                        break;
                    }
                }
                Err(error) => {
                    error!(error = %error, "Batch task processing failed");
                    let error_response = self.create_processing_error_response(error);
                    let _ = response_sender.send(Ok(error_response)).await;
                }
            }
        }

        let batch_duration = batch_start.elapsed();
        self.performance_monitor.record_batch_processing(batch_size, batch_duration);

        debug!(
            batch_id = %batch_id,
            batch_size = batch_size,
            duration = ?batch_duration,
            throughput = batch_size as f64 / batch_duration.as_secs_f64(),
            "Optimized batch processing completed"
        );
    }

    /// 最適化結果の取得
    pub fn get_optimization_result(&self) -> OptimizationResult {
        let performance_summary = self.performance_monitor.get_performance_summary();
        let processor_stats = self.get_processor_statistics();
        
        OptimizationResult {
            throughput: performance_summary.throughput_rps,
            avg_latency: Duration::from_secs_f64(performance_summary.avg_latency_ms / 1000.0),
            p95_latency: Duration::from_secs_f64(performance_summary.p95_latency_ms / 1000.0),
            p99_latency: Duration::from_secs_f64(performance_summary.p99_latency_ms / 1000.0),
            memory_usage: performance_summary.memory_usage_mb as usize * 1024 * 1024,
            cache_hit_ratio: performance_summary.cache_efficiency,
            worker_utilization: performance_summary.worker_utilization,
        }
    }

    /// プロセッサー統計を取得
    pub fn get_processor_statistics(&self) -> ProcessorStatistics {
        let mut stats = self.processor_stats.lock()
            .map(|s| s.clone())
            .unwrap_or_default();

        // リアルタイム統計を更新
        stats.worker_pool_utilization = self.worker_pool.usage_ratio().await;
        stats.cache_hit_rate = self.cache_system.get_statistics().hit_ratio;
        
        stats
    }

    /// 最適化効果を測定
    pub async fn measure_optimization_effectiveness(&self) -> OptimizationMetrics {
        // ベースライン測定（実装省略）
        let baseline_metrics = self.get_baseline_metrics().await;
        let current_metrics = self.get_optimization_result();

        let throughput_improvement = current_metrics.throughput / baseline_metrics.throughput;
        let latency_reduction = 1.0 - (current_metrics.avg_latency.as_secs_f64() / baseline_metrics.avg_latency.as_secs_f64());
        let memory_efficiency = baseline_metrics.memory_usage as f64 / current_metrics.memory_usage as f64;

        OptimizationMetrics {
            throughput_improvement,
            latency_reduction,
            memory_efficiency_gain: memory_efficiency - 1.0,
            cpu_utilization_optimization: 0.2, // 仮の値
            overall_performance_score: (throughput_improvement + latency_reduction + memory_efficiency) / 3.0,
        }
    }

    // 内部ヘルパーメソッド

    async fn should_apply_backpressure(&self) -> bool {
        let current_load = self.backpressure_controller.get_current_load().await;
        current_load > self.backpressure_controller.thresholds.critical_threshold
    }

    async fn handle_backpressure(
        &self,
        response_sender: &mpsc::Sender<Result<StreamResponse, Status>>,
        connection_id: &str,
    ) {
        warn!(connection_id = connection_id, "Applying backpressure due to high load");

        // バックプレッシャー警告をクライアントに送信
        let backpressure_response = self.create_backpressure_warning();
        let _ = response_sender.send(Ok(backpressure_response)).await;

        // 短時間待機
        tokio::time::sleep(Duration::from_millis(10)).await;

        self.performance_monitor.record_backpressure_event();
    }

    fn should_batch_request(&self, _request: &StreamRequest) -> bool {
        // バッチ処理判定ロジック
        true // 簡略化
    }

    fn determine_task_priority(&self, _request: &StreamRequest) -> TaskPriority {
        // 優先度決定ロジック
        TaskPriority::Normal // 簡略化
    }

    fn create_stream_error_response(&self, _status: Status) -> StreamResponse {
        // エラーレスポンス作成
        StreamResponse::default() // 簡略化
    }

    fn create_processing_error_response(&self, _error: crate::grpc::performance::worker_pool::ProcessingError) -> StreamResponse {
        // 処理エラーレスポンス作成
        StreamResponse::default() // 簡略化
    }

    fn create_backpressure_warning(&self) -> StreamResponse {
        // バックプレッシャー警告レスポンス
        StreamResponse::default() // 簡略化
    }

    async fn increment_active_connections(&self) {
        if let Ok(mut stats) = self.processor_stats.lock() {
            stats.active_connections += 1;
            stats.total_connections += 1;
        }
    }

    async fn decrement_active_connections(&self) {
        if let Ok(mut stats) = self.processor_stats.lock() {
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }
    }

    async fn get_baseline_metrics(&self) -> OptimizationResult {
        // ベースライン測定（実装省略）
        OptimizationResult {
            throughput: 1000.0,
            avg_latency: Duration::from_millis(50),
            p95_latency: Duration::from_millis(100),
            p99_latency: Duration::from_millis(200),
            memory_usage: 100 * 1024 * 1024, // 100MB
            cache_hit_ratio: 0.0,
            worker_utilization: 0.3,
        }
    }

    fn start_statistics_update_tasks(&self) {
        // 統計更新タスク開始
    }
}

impl BackpressureController {
    fn new(thresholds: BackpressureThresholds) -> Self {
        Self {
            current_load: Arc::new(Mutex::new(0.0)),
            thresholds,
            adaptive_settings: Arc::new(Mutex::new(AdaptiveBackpressureSettings {
                current_limit: 1.0,
                adjustment_factor: 0.1,
                last_adjustment: Instant::now(),
            })),
        }
    }

    async fn get_current_load(&self) -> f64 {
        self.current_load.lock()
            .map(|load| *load)
            .unwrap_or(0.0)
    }
}

// 設定変換トレイト実装
impl OptimizationConfig {
    pub fn to_resource_pool_config(&self) -> crate::grpc::performance::resource_pool::ResourcePoolConfig {
        // 実装省略
        crate::grpc::performance::resource_pool::ResourcePoolConfig::default()
    }

    pub fn to_cache_config(&self) -> crate::grpc::performance::cache::CacheConfig {
        // 実装省略  
        crate::grpc::performance::cache::CacheConfig::default()
    }

    pub fn to_worker_pool_config(&self) -> crate::grpc::performance::worker_pool::WorkerPoolConfig {
        // 実装省略
        crate::grpc::performance::worker_pool::WorkerPoolConfig::default()
    }
}
```

### 既存service.rsとの統合

#### 段階的置き換え実装
```rust
// src/grpc/service.rs への統合

use crate::grpc::performance::{
    OptimizedStreamProcessor,
    OptimizationConfig,
};

impl UnityMcpService for UnityMcpServiceImpl {
    #[instrument(skip(self))]
    async fn stream(
        &self,
        request: Request<Streaming<StreamRequest>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        info!("Stream connection established with optimized processor");

        // 最適化プロセッサーを作成
        let optimization_config = OptimizationConfig::default();
        let processor = OptimizedStreamProcessor::new(optimization_config);
        
        let incoming_stream = request.into_inner();
        let (response_sender, response_receiver) = tokio::sync::mpsc::channel(10000);
        
        // 最適化ストリーミング処理を開始
        tokio::spawn(async move {
            processor.process_stream_optimized(incoming_stream, response_sender).await;
        });

        // レスポンスストリームを作成
        let response_stream = tokio_stream::wrappers::ReceiverStream::new(response_receiver);
        let boxed_stream: Self::StreamStream = Box::pin(response_stream);

        Ok(Response::new(boxed_stream))
    }
}
```

## 実装計画

### Step 1: OptimizedStreamProcessor 基盤 (30分)
1. 基本構造とコンポーネント統合
2. 最適化コンテキスト管理
3. フィーチャーフラグ実装

### Step 2: ストリーミング処理統合 (35分)
1. process_stream_optimized メインロジック
2. バッチ処理とキャッシュ統合
3. バックプレッシャー制御

### Step 3: 既存システム統合 (25分)
1. service.rs の段階的置き換え
2. 後方互換性確保
3. 統合テスト実行

## テスト要件

### 統合テスト
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_optimized_processor_integration() {
        let config = OptimizationConfig::high_performance();
        let processor = OptimizedStreamProcessor::new(config);
        
        // 模擬ストリーミングテスト
        let (stream_sender, stream_receiver) = create_test_stream();
        let (response_sender, mut response_receiver) = mpsc::channel(100);
        
        // 最適化処理開始
        let processor_task = tokio::spawn(async move {
            processor.process_stream_optimized(stream_receiver, response_sender).await;
        });
        
        // テストデータ送信
        send_test_requests(stream_sender, 1000).await;
        
        // レスポンス検証
        let mut response_count = 0;
        while let Some(response) = response_receiver.recv().await {
            response_count += 1;
            assert!(response.is_ok());
        }
        
        assert_eq!(response_count, 1000);
        
        processor_task.await.unwrap();
    }
    
    #[tokio::test]
    async fn test_performance_targets() {
        let processor = OptimizedStreamProcessor::new(
            OptimizationConfig::high_performance()
        );
        
        // パフォーマンステスト実行
        let result = run_performance_benchmark(&processor).await;
        
        // 目標値検証
        assert!(result.throughput >= 2000.0);
        assert!(result.p95_latency <= Duration::from_millis(25));
        assert!(result.cache_hit_ratio >= 0.7);
        assert!(result.worker_utilization >= 0.8);
    }
}
```

## 成功基準

### パフォーマンス目標
- **スループット**: 2000 req/s 以上
- **レイテンシー**: P95 25ms以下、P99 50ms以下  
- **メモリ使用量**: 現状から30%削減
- **同時接続**: 100接続安定処理

### 統合基準
- 既存テスト100%パス
- 後方互換性維持
- エラー率1%以下
- グレースフルデグラデーション

## 次のステップ

最適化プロセッサー統合完了後：
1. 本番環境での段階的ロールアウト
2. 継続的パフォーマンス監視の設定
3. 最適化効果の長期評価開始

## 関連ドキュメント
- Task 3.7 Fix 07-A〜E (全サブタスク)
- 既存service.rsレビュー結果
- パフォーマンス目標仕様書