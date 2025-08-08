# Task 3.7 Fix 07: パフォーマンス最適化

## 概要
Task 3.7のストリーミングサービス実装のパフォーマンスを最適化します。前タスクで実施したテストの結果を基に、スループットの向上、レイテンシーの削減、メモリ使用量の最適化を実現します。

## 優先度
**🟡 中優先度** - システム性能向上とスケーラビリティに影響

## 実装時間見積もり
**3-4時間** - 集中作業時間

## 受け入れ基準

### パフォーマンス要件
- [ ] スループット向上: 1000 req/s → 2000 req/s
- [ ] レイテンシー削減: P95 50ms → 25ms以下
- [ ] メモリ使用量削減: 現状から30%削減
- [ ] CPUオーバーヘッド最小化

### スケーラビリティ要件
- [ ] 同時接続数: 100接続対応
- [ ] バックプレッシャー機能の実装
- [ ] 動的リソース調整機能
- [ ] 効率的なリソースプール管理

### 安定性要件
- [ ] 高負荷状態でのシステム安定性
- [ ] グレースフルデグラデーション
- [ ] メモリリーク完全防止

## 技術的詳細

### パフォーマンス最適化アーキテクチャ

#### 1. メッセージ処理パイプラインの最適化
```rust
/// High-performance message processing pipeline
pub struct OptimizedStreamProcessor {
    // 専用ワーカープール
    worker_pool: Arc<WorkerPool>,
    
    // 効率的なメッセージキュー
    message_queue: Arc<BoundedMpscQueue<ProcessingTask>>,
    
    // リソースプール管理
    resource_pool: Arc<ResourcePool>,
    
    // パフォーマンス監視
    performance_monitor: Arc<StreamPerformanceMonitor>,
    
    // 設定可能な最適化パラメータ
    config: OptimizationConfig,
}

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    // ワーカー数（CPUコア数ベース）
    pub worker_count: usize,
    
    // バッチ処理サイズ
    pub batch_size: usize,
    
    // キューサイズ
    pub queue_capacity: usize,
    
    // バックプレッシャー閾値
    pub backpressure_threshold: f64,
    
    // プリフェッチサイズ
    pub prefetch_size: usize,
    
    // キャッシュサイズ
    pub cache_size: usize,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            worker_count: cpu_count.max(4),
            batch_size: 10,
            queue_capacity: 10000,
            backpressure_threshold: 0.8,
            prefetch_size: 100,
            cache_size: 1000,
        }
    }
}

impl OptimizedStreamProcessor {
    pub fn new(config: OptimizationConfig) -> Self {
        let worker_pool = Arc::new(WorkerPool::new(config.worker_count));
        let message_queue = Arc::new(BoundedMpscQueue::new(config.queue_capacity));
        let resource_pool = Arc::new(ResourcePool::new());
        let performance_monitor = Arc::new(StreamPerformanceMonitor::new());
        
        Self {
            worker_pool,
            message_queue,
            resource_pool,
            performance_monitor,
            config,
        }
    }

    pub async fn process_stream_optimized(
        &self,
        mut incoming_stream: Streaming<StreamRequest>,
        response_sender: tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        let start_time = std::time::Instant::now();
        let connection_id = uuid::Uuid::new_v4().to_string();
        
        info!(connection_id = %connection_id, "Starting optimized stream processing");

        // バッチ処理用バッファ
        let mut batch_buffer = Vec::with_capacity(self.config.batch_size);
        let mut message_counter = 0u64;

        while let Some(result) = incoming_stream.next().await {
            match result {
                Ok(stream_request) => {
                    message_counter += 1;
                    
                    let processing_task = ProcessingTask {
                        request: stream_request,
                        message_id: message_counter,
                        connection_id: connection_id.clone(),
                        timestamp: std::time::Instant::now(),
                    };

                    batch_buffer.push(processing_task);

                    // バッチサイズに達したら処理
                    if batch_buffer.len() >= self.config.batch_size {
                        self.process_batch(
                            std::mem::take(&mut batch_buffer),
                            &response_sender,
                        ).await;
                    }

                    // バックプレッシャーチェック
                    if self.should_apply_backpressure().await {
                        self.handle_backpressure(&response_sender).await;
                    }
                }
                Err(status) => {
                    warn!(connection_id = %connection_id, error = %status, "Stream error encountered");
                    
                    // 残りのバッチを処理してエラーレスポンスを送信
                    if !batch_buffer.is_empty() {
                        self.process_batch(batch_buffer, &response_sender).await;
                    }
                    
                    let error_response = self.create_stream_error_response(status);
                    let _ = response_sender.send(Ok(error_response)).await;
                    break;
                }
            }
        }

        // 残りのバッファを処理
        if !batch_buffer.is_empty() {
            self.process_batch(batch_buffer, &response_sender).await;
        }

        let total_time = start_time.elapsed();
        self.performance_monitor.record_stream_session(
            connection_id.clone(),
            message_counter,
            total_time,
        );

        info!(
            connection_id = %connection_id,
            message_count = message_counter,
            duration = ?total_time,
            "Optimized stream processing completed"
        );
    }

    async fn process_batch(
        &self,
        batch: Vec<ProcessingTask>,
        response_sender: &tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        let batch_start = std::time::Instant::now();
        let batch_size = batch.len();

        // 並列処理用のワーカータスクを作成
        let mut worker_tasks = Vec::new();
        
        for task in batch {
            let worker = Arc::clone(&self.worker_pool);
            let resource_pool = Arc::clone(&self.resource_pool);
            
            let worker_task = tokio::spawn(async move {
                worker.execute(task, resource_pool).await
            });
            
            worker_tasks.push(worker_task);
        }

        // 全てのワーカータスクの完了を待機
        let results = futures::future::join_all(worker_tasks).await;

        // 結果をレスポンスチャネルに送信
        for result in results {
            match result {
                Ok(Ok(response)) => {
                    if response_sender.send(Ok(response)).await.is_err() {
                        warn!("Failed to send response - receiver dropped");
                        break;
                    }
                }
                Ok(Err(error_response)) => {
                    if response_sender.send(Ok(error_response)).await.is_err() {
                        warn!("Failed to send error response - receiver dropped");
                        break;
                    }
                }
                Err(join_error) => {
                    warn!(error = %join_error, "Worker task failed");
                    // 適切なエラーレスポンスを生成して送信
                    let error_response = self.create_worker_error_response(join_error);
                    let _ = response_sender.send(Ok(error_response)).await;
                }
            }
        }

        let batch_duration = batch_start.elapsed();
        self.performance_monitor.record_batch_processing(batch_size, batch_duration);
        
        debug!(
            batch_size = batch_size,
            duration = ?batch_duration,
            throughput = batch_size as f64 / batch_duration.as_secs_f64(),
            "Batch processing completed"
        );
    }

    async fn should_apply_backpressure(&self) -> bool {
        let queue_usage = self.message_queue.usage_ratio().await;
        let worker_usage = self.worker_pool.usage_ratio().await;
        
        queue_usage > self.config.backpressure_threshold ||
        worker_usage > self.config.backpressure_threshold
    }

    async fn handle_backpressure(
        &self,
        response_sender: &tokio::sync::mpsc::Sender<Result<StreamResponse, Status>>,
    ) {
        warn!("Applying backpressure due to high load");
        
        // バックプレッシャー警告をクライアントに送信
        let backpressure_response = self.create_backpressure_warning();
        let _ = response_sender.send(Ok(backpressure_response)).await;
        
        // 短時間待機してシステム負荷を軽減
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        self.performance_monitor.record_backpressure_event();
    }
}
```

#### 2. 効率的なワーカープール
```rust
/// High-performance worker pool for message processing
pub struct WorkerPool {
    workers: Vec<Worker>,
    task_scheduler: Arc<TaskScheduler>,
    load_balancer: Arc<LoadBalancer>,
}

pub struct Worker {
    id: usize,
    task_queue: tokio::sync::mpsc::Receiver<ProcessingTask>,
    service_instance: Arc<UnityMcpServiceImpl>,
    performance_counter: Arc<WorkerPerformanceCounter>,
}

impl WorkerPool {
    pub fn new(worker_count: usize) -> Self {
        let task_scheduler = Arc::new(TaskScheduler::new());
        let load_balancer = Arc::new(LoadBalancer::new(worker_count));
        
        let workers = (0..worker_count)
            .map(|id| Worker::new(id, Arc::clone(&task_scheduler)))
            .collect();

        Self {
            workers,
            task_scheduler,
            load_balancer,
        }
    }

    pub async fn execute(
        &self,
        task: ProcessingTask,
        resource_pool: Arc<ResourcePool>,
    ) -> Result<StreamResponse, StreamResponse> {
        let worker_id = self.load_balancer.select_worker(&task).await;
        
        // タスクを選択されたワーカーに送信
        self.task_scheduler.schedule_task(worker_id, task).await
            .map_err(|e| self.create_scheduling_error(e))
    }

    pub async fn usage_ratio(&self) -> f64 {
        let total_capacity = self.workers.len() as f64;
        let busy_workers = self.count_busy_workers().await as f64;
        busy_workers / total_capacity
    }

    async fn count_busy_workers(&self) -> usize {
        // 実装: ビジーなワーカーの数をカウント
        0 // スタブ
    }
}

impl Worker {
    pub fn new(id: usize, task_scheduler: Arc<TaskScheduler>) -> Self {
        let (task_sender, task_receiver) = tokio::sync::mpsc::channel(1000);
        let service_instance = Arc::new(UnityMcpServiceImpl::new());
        let performance_counter = Arc::new(WorkerPerformanceCounter::new());

        // ワーカータスクを開始
        let worker_task = Self::start_worker_loop(
            id,
            task_receiver,
            Arc::clone(&service_instance),
            Arc::clone(&performance_counter),
        );
        
        tokio::spawn(worker_task);

        // タスクスケジューラーにワーカーを登録
        task_scheduler.register_worker(id, task_sender);

        Self {
            id,
            task_queue: task_receiver,
            service_instance,
            performance_counter,
        }
    }

    async fn start_worker_loop(
        worker_id: usize,
        mut task_queue: tokio::sync::mpsc::Receiver<ProcessingTask>,
        service: Arc<UnityMcpServiceImpl>,
        performance_counter: Arc<WorkerPerformanceCounter>,
    ) {
        info!(worker_id = worker_id, "Worker started");

        while let Some(task) = task_queue.recv().await {
            let task_start = std::time::Instant::now();
            
            let response = Self::process_single_task(&service, task).await;
            
            let task_duration = task_start.elapsed();
            performance_counter.record_task(task_duration, response.is_ok());
        }

        info!(worker_id = worker_id, "Worker terminated");
    }

    async fn process_single_task(
        service: &Arc<UnityMcpServiceImpl>,
        task: ProcessingTask,
    ) -> Result<StreamResponse, StreamResponse> {
        match task.request.message {
            Some(stream_request::Message::ImportAsset(req)) => {
                ImportAssetStreamHandler::handle(
                    service,
                    req,
                    task.connection_id,
                    task.message_id,
                ).await
            }
            Some(stream_request::Message::MoveAsset(req)) => {
                MoveAssetStreamHandler::handle(
                    service,
                    req,
                    task.connection_id,
                    task.message_id,
                ).await
            }
            Some(stream_request::Message::DeleteAsset(req)) => {
                DeleteAssetStreamHandler::handle(
                    service,
                    req,
                    task.connection_id,
                    task.message_id,
                ).await
            }
            Some(stream_request::Message::Refresh(req)) => {
                RefreshStreamHandler::handle(
                    service,
                    req,
                    task.connection_id,
                    task.message_id,
                ).await
            }
            None => Err(Self::create_empty_message_error(task.message_id)),
        }
        .map_err(|error_response| error_response) // エラーレスポンスもOk扱い（ストリームは継続）
    }
}
```

#### 3. インテリジェントキャッシュシステム
```rust
/// Intelligent caching system for stream processing
pub struct StreamCache {
    // LRU キャッシュ
    response_cache: Arc<Mutex<lru::LruCache<CacheKey, CacheEntry>>>,
    
    // 統計情報
    cache_stats: Arc<CacheStatistics>,
    
    // 設定
    config: CacheConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    operation_type: String,
    request_hash: u64,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    response: StreamResponse,
    created_at: std::time::Instant,
    access_count: u64,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub ttl: std::time::Duration,
    pub enable_prefetch: bool,
}

impl StreamCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            response_cache: Arc::new(Mutex::new(
                lru::LruCache::new(std::num::NonZeroUsize::new(config.max_entries).unwrap())
            )),
            cache_stats: Arc::new(CacheStatistics::new()),
            config,
        }
    }

    pub async fn get(&self, key: &CacheKey) -> Option<StreamResponse> {
        if let Ok(mut cache) = self.response_cache.lock() {
            if let Some(entry) = cache.get_mut(key) {
                // TTL チェック
                if entry.created_at.elapsed() < self.config.ttl {
                    entry.access_count += 1;
                    self.cache_stats.record_hit();
                    return Some(entry.response.clone());
                } else {
                    // 期限切れエントリを削除
                    cache.pop(key);
                }
            }
        }
        
        self.cache_stats.record_miss();
        None
    }

    pub async fn put(&self, key: CacheKey, response: StreamResponse) {
        if let Ok(mut cache) = self.response_cache.lock() {
            let entry = CacheEntry {
                response,
                created_at: std::time::Instant::now(),
                access_count: 1,
            };
            
            cache.put(key, entry);
            self.cache_stats.record_store();
        }
    }

    pub fn generate_cache_key(&self, request: &StreamRequest) -> Option<CacheKey> {
        match &request.message {
            Some(stream_request::Message::ImportAsset(req)) => {
                Some(CacheKey {
                    operation_type: "import_asset".to_string(),
                    request_hash: self.hash_import_request(req),
                })
            }
            Some(stream_request::Message::MoveAsset(req)) => {
                Some(CacheKey {
                    operation_type: "move_asset".to_string(),
                    request_hash: self.hash_move_request(req),
                })
            }
            // DeleteとRefreshはキャッシュしない（副作用があるため）
            _ => None,
        }
    }

    fn hash_import_request(&self, req: &ImportAssetRequest) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        req.asset_path.hash(&mut hasher);
        hasher.finish()
    }

    fn hash_move_request(&self, req: &MoveAssetRequest) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        req.src_path.hash(&mut hasher);
        req.dst_path.hash(&mut hasher);
        hasher.finish()
    }

    pub fn get_cache_statistics(&self) -> CacheStatistics {
        self.cache_stats.clone()
    }
}
```

#### 4. リソースプール管理
```rust
/// Efficient resource pool management
pub struct ResourcePool {
    // サービスインスタンスプール
    service_pool: Arc<ObjectPool<UnityMcpServiceImpl>>,
    
    // バリデーションエンジンプール
    validator_pool: Arc<ObjectPool<StreamValidationEngine>>,
    
    // 一時的なバッファプール
    buffer_pool: Arc<ObjectPool<Vec<u8>>>,
    
    // プール統計
    pool_stats: Arc<PoolStatistics>,
}

pub struct ObjectPool<T> {
    objects: Arc<Mutex<Vec<T>>>,
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    max_size: usize,
}

impl<T> ObjectPool<T> {
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            objects: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            factory: Arc::new(factory),
            max_size,
        }
    }

    pub async fn get(&self) -> PooledObject<T> {
        if let Ok(mut objects) = self.objects.lock() {
            if let Some(object) = objects.pop() {
                return PooledObject::new(object, Arc::clone(&self.objects));
            }
        }
        
        // プールが空の場合は新しいオブジェクトを作成
        let object = (self.factory)();
        PooledObject::new(object, Arc::clone(&self.objects))
    }
}

pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<Mutex<Vec<T>>>,
}

impl<T> PooledObject<T> {
    fn new(object: T, pool: Arc<Mutex<Vec<T>>>) -> Self {
        Self {
            object: Some(object),
            pool,
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object.as_ref().unwrap()
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object.as_mut().unwrap()
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            if let Ok(mut pool) = self.pool.lock() {
                pool.push(object);
            }
        }
    }
}

impl ResourcePool {
    pub fn new() -> Self {
        Self {
            service_pool: Arc::new(ObjectPool::new(
                || UnityMcpServiceImpl::new(),
                10,
            )),
            validator_pool: Arc::new(ObjectPool::new(
                || StreamValidationEngine::new(),
                5,
            )),
            buffer_pool: Arc::new(ObjectPool::new(
                || Vec::with_capacity(8192),
                50,
            )),
            pool_stats: Arc::new(PoolStatistics::new()),
        }
    }

    pub async fn get_service(&self) -> PooledObject<UnityMcpServiceImpl> {
        let service = self.service_pool.get().await;
        self.pool_stats.record_service_acquisition();
        service
    }

    pub async fn get_validator(&self) -> PooledObject<StreamValidationEngine> {
        let validator = self.validator_pool.get().await;
        self.pool_stats.record_validator_acquisition();
        validator
    }

    pub async fn get_buffer(&self) -> PooledObject<Vec<u8>> {
        let mut buffer = self.buffer_pool.get().await;
        buffer.clear(); // バッファをクリア
        self.pool_stats.record_buffer_acquisition();
        buffer
    }
}
```

#### 5. パフォーマンス監視システム
```rust
/// Comprehensive performance monitoring system
pub struct StreamPerformanceMonitor {
    // メトリクス収集
    metrics: Arc<Mutex<PerformanceMetrics>>,
    
    // リアルタイム統計
    real_time_stats: Arc<RealTimeStats>,
    
    // 履歴データ
    historical_data: Arc<Mutex<HistoricalData>>,
}

#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    // スループット
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    
    // レイテンシー
    pub latency_sum: std::time::Duration,
    pub latency_min: std::time::Duration,
    pub latency_max: std::time::Duration,
    
    // メモリ使用量
    pub current_memory_usage: usize,
    pub peak_memory_usage: usize,
    
    // ワーカー統計
    pub worker_utilization: f64,
    pub queue_depth: usize,
    
    // キャッシュ統計
    pub cache_hit_ratio: f64,
}

impl StreamPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
            real_time_stats: Arc::new(RealTimeStats::new()),
            historical_data: Arc::new(Mutex::new(HistoricalData::new())),
        }
    }

    pub fn record_stream_session(
        &self,
        connection_id: String,
        message_count: u64,
        duration: std::time::Duration,
    ) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_requests += message_count;
            metrics.successful_requests += message_count; // 簡略化
            
            let avg_latency = duration / message_count as u32;
            metrics.latency_sum += duration;
            metrics.latency_min = metrics.latency_min.min(avg_latency);
            metrics.latency_max = metrics.latency_max.max(avg_latency);
        }

        self.real_time_stats.update_throughput(message_count, duration);
        
        if let Ok(mut history) = self.historical_data.lock() {
            history.record_session(connection_id, message_count, duration);
        }
    }

    pub fn record_batch_processing(&self, batch_size: usize, duration: std::time::Duration) {
        let throughput = batch_size as f64 / duration.as_secs_f64();
        self.real_time_stats.update_batch_throughput(throughput);
    }

    pub fn record_backpressure_event(&self) {
        self.real_time_stats.increment_backpressure_events();
    }

    pub fn get_current_metrics(&self) -> PerformanceMetrics {
        if let Ok(metrics) = self.metrics.lock() {
            metrics.clone()
        } else {
            PerformanceMetrics::default()
        }
    }

    pub fn get_performance_summary(&self) -> PerformanceSummary {
        let metrics = self.get_current_metrics();
        let real_time = self.real_time_stats.get_current_stats();
        
        PerformanceSummary {
            throughput: real_time.current_throughput,
            avg_latency: if metrics.total_requests > 0 {
                metrics.latency_sum / metrics.total_requests as u32
            } else {
                std::time::Duration::default()
            },
            success_rate: if metrics.total_requests > 0 {
                metrics.successful_requests as f64 / metrics.total_requests as f64
            } else {
                0.0
            },
            memory_usage: metrics.current_memory_usage,
            worker_utilization: metrics.worker_utilization,
            cache_efficiency: metrics.cache_hit_ratio,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub throughput: f64,              // req/s
    pub avg_latency: std::time::Duration,
    pub success_rate: f64,            // 0.0-1.0
    pub memory_usage: usize,          // bytes
    pub worker_utilization: f64,      // 0.0-1.0
    pub cache_efficiency: f64,        // 0.0-1.0
}
```

#### 6. 統合された最適化ストリーム処理
```rust
/// Optimized stream service implementation
impl UnityMcpService for UnityMcpServiceImpl {
    #[instrument(skip(self))]
    async fn stream(
        &self,
        request: Request<Streaming<StreamRequest>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        info!("Optimized stream connection established");

        // 最適化されたプロセッサーを作成
        let optimization_config = OptimizationConfig::default();
        let processor = OptimizedStreamProcessor::new(optimization_config);
        
        // ストリーミング処理を開始
        let incoming_stream = request.into_inner();
        let (response_sender, response_receiver) = tokio::sync::mpsc::channel(10000);
        
        // メッセージ処理タスクを開始
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

### Step 1: パフォーマンス基盤の構築
1. `OptimizedStreamProcessor`の実装
2. 設定可能な最適化パラメータの定義
3. パフォーマンス監視システムの構築

### Step 2: 並列処理システムの実装
1. 効率的なワーカープールの実装
2. タスクスケジューリングとロードバランシング
3. バッチ処理機能の実装

### Step 3: キャッシュシステムの実装
1. LRUキャッシュの実装
2. インテリジェントなキーヒング戦略
3. TTL管理とキャッシュ統計

### Step 4: リソース管理最適化
1. オブジェクトプールの実装
2. メモリ効率的なバッファ管理
3. リソースライフサイクル管理

### Step 5: 統合テストと調整
1. パフォーマンス測定とベンチマーク
2. 最適化パラメータのチューニング
3. 負荷テストと安定性確認

## ベンチマークと検証

### パフォーマンステスト
```rust
#[cfg(test)]
mod performance_benchmarks {
    use super::*;
    
    #[tokio::test]
    async fn benchmark_optimized_throughput() {
        let processor = OptimizedStreamProcessor::new(OptimizationConfig::default());
        
        let request_count = 20000;
        let start_time = std::time::Instant::now();
        
        // 高負荷テスト実行
        let results = run_throughput_test(&processor, request_count).await;
        
        let elapsed = start_time.elapsed();
        let throughput = request_count as f64 / elapsed.as_secs_f64();
        
        println!("Optimized throughput: {:.2} req/s", throughput);
        assert!(throughput > 2000.0, "Target throughput not achieved");
    }

    #[tokio::test]
    async fn benchmark_memory_efficiency() {
        let processor = OptimizedStreamProcessor::new(OptimizationConfig::default());
        
        let initial_memory = get_current_memory_usage();
        
        // メモリ使用量テスト
        run_memory_test(&processor).await;
        
        let peak_memory = get_peak_memory_usage();
        let memory_increase = peak_memory - initial_memory;
        
        println!("Memory increase: {} MB", memory_increase / 1024 / 1024);
        assert!(memory_increase < 50 * 1024 * 1024, "Memory usage too high");
    }

    #[tokio::test]
    async fn benchmark_latency_optimization() {
        let processor = OptimizedStreamProcessor::new(OptimizationConfig::default());
        
        let latency_results = run_latency_test(&processor).await;
        
        println!("P95 latency: {:?}", latency_results.p95);
        println!("P99 latency: {:?}", latency_results.p99);
        
        assert!(latency_results.p95 < std::time::Duration::from_millis(25));
        assert!(latency_results.p99 < std::time::Duration::from_millis(50));
    }
}
```

## 成功基準

### パフォーマンス指標
- **スループット**: 2000 req/s以上
- **レイテンシー**: P95 25ms以下、P99 50ms以下
- **メモリ使用量**: 現状から30%削減
- **CPU使用率**: 効率的な利用（80%以下）

### スケーラビリティ指標
- **同時接続**: 100接続の安定処理
- **負荷耐性**: 高負荷状態での安定動作
- **リソース効率**: プール利用率の最適化

### 品質指標
- **エラー率**: 1%以下維持
- **メモリリーク**: 完全防止
- **レスポンス一貫性**: 100%

## 次のステップ

最適化完了後:
1. 本番環境での段階的ロールアウト
2. 継続的なパフォーマンス監視
3. 最適化効果の長期評価

## 関連ドキュメント
- Task 3.7 Fix 06 (包括的テストスイート)
- パフォーマンスベンチマーク結果
- 最適化設定ガイドライン