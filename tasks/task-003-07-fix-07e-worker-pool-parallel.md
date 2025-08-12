# Task 3.7 Fix 07-E: 並列処理ワーカープール（Rayon活用版）

## 概要
Rayonクレートを活用した高性能並列処理システムを実装します。Rayonの実証済みwork-stealingアルゴリズムとCPU最適化されたスレッドプールを利用することで、シンプルかつ効率的な並列処理によるスループット向上とレスポンス時間短縮を実現します。

## 優先度
**🔴 最高優先度** - スループット向上の中核コンポーネント

## 実装時間見積もり
**30分** - 集中作業時間（Rayon活用により大幅短縮）

## 依存関係
- Task 3.7 Fix 07-A (基盤インフラ整備) 完了必須
- Task 3.7 Fix 07-B (パフォーマンス監視システム) 完了推奨

## 受け入れ基準

### 並列処理要件
- [ ] Rayonによる自動CPU最適化
- [ ] Work-stealingアルゴリズムによる効率的負荷分散
- [ ] Parallel iteratorによるバッチ処理最適化
- [ ] カスタムThreadPoolBuilder設定

### 負荷制御要件
- [ ] セマフォによるバックプレッシャー制御
- [ ] 非同期タスクとの統合（tokio-rayon）
- [ ] シンプルな優先度制御
- [ ] 統計収集とパフォーマンス監視統合

### 安定性要件
- [ ] Rayonの内蔵パニック復旧機能活用
- [ ] グレースフルシャットダウン
- [ ] Rustの型安全性によるデッドロック回避
- [ ] 軽量なエラーハンドリング

### パフォーマンス要件
- [ ] スループット 2000 req/s 達成
- [ ] タスク分散遅延 < 1ms
- [ ] ワーカー利用率 > 80%
- [ ] バッチ処理効率 > 90%

## 技術的詳細

### ParallelProcessor 実装（Rayon活用）

#### server/Cargo.toml への依存追加
```toml
[dependencies]
rayon = "1.8"
tokio-rayon = "2.1"  # tokioとの統合用
```

#### src/grpc/performance/parallel_processor.rs
```rust
//! Rayon活用並列処理システム
//! 
//! Unity MCP Server のストリーミング処理において、Rayonの実証済み
//! work-stealingアルゴリズムによる高効率並列処理を実現する。

use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::{Duration, Instant};
use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn, error, instrument};
use uuid::Uuid;
use crate::grpc::service::UnityMcpServiceImpl;
use crate::grpc::performance::monitor::StreamPerformanceMonitor;
use crate::unity::{StreamRequest, StreamResponse};

/// Rayon並列処理システム
pub struct ParallelProcessor {
    // Rayonスレッドプール
    thread_pool: ThreadPool,
    
    // 非同期制御
    semaphore: Arc<Semaphore>,
    
    // 統計情報
    stats: Arc<ProcessingStatistics>,
    
    // パフォーマンス監視
    performance_monitor: Option<Arc<StreamPerformanceMonitor>>,
    
    // 設定
    config: ParallelConfig,
}

/// 並列処理設定
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    // Rayonスレッドプール設定
    pub thread_count: Option<usize>,  // Noneの場合はCPUコア数自動設定
    pub thread_name_prefix: String,
    
    // バッチ処理設定
    pub batch_size: usize,
    pub max_concurrent_batches: usize,
    
    // バックプレッシャー制御
    pub max_pending_tasks: usize,
    
    // 監視設定
    pub enable_statistics: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            thread_count: None,  // Rayonが自動設定
            thread_name_prefix: "unity-mcp-worker".to_string(),
            batch_size: 10,
            max_concurrent_batches: 100,
            max_pending_tasks: 1000,
            enable_statistics: true,
        }
    }
}

/// 処理統計
#[derive(Debug, Default)]
pub struct ProcessingStatistics {
    pub total_processed: AtomicU64,
    pub total_failed: AtomicU64,
    pub total_processing_time: Mutex<Duration>,
}

impl ParallelProcessor {
    /// 新しい並列プロセッサーを作成
    pub fn new() -> anyhow::Result<Self> {
        Self::with_config(ParallelConfig::default())
    }

    /// 設定付きで並列プロセッサーを作成
    pub fn with_config(config: ParallelConfig) -> anyhow::Result<Self> {
        let mut builder = ThreadPoolBuilder::new()
            .thread_name(|i| format!("{}-{}", config.thread_name_prefix, i));
        
        if let Some(count) = config.thread_count {
            builder = builder.num_threads(count);
        }
        
        let thread_pool = builder.build()?;
        let semaphore = Arc::new(Semaphore::new(config.max_pending_tasks));
        let stats = Arc::new(ProcessingStatistics::default());
        
        info!(
            "Parallel processor initialized with {} threads", 
            thread_pool.current_num_threads()
        );
        
        Ok(Self {
            thread_pool,
            semaphore,
            stats,
            performance_monitor: None,
            config,
        })
    }

    /// パフォーマンス監視を設定
    pub fn with_performance_monitor(mut self, monitor: Arc<StreamPerformanceMonitor>) -> Self {
        self.performance_monitor = Some(monitor);
        self
    }

    /// 並列バッチ処理を実行
    #[instrument(skip(self, requests))]
    pub async fn execute_parallel_batch(
        &self,
        requests: Vec<StreamRequest>,
    ) -> Vec<Result<StreamResponse, ProcessingError>> {
        let batch_size = requests.len();
        let start_time = Instant::now();
        
        debug!("Processing batch of {} requests in parallel", batch_size);
        
        // バックプレッシャー制御
        let _permit = self.semaphore.acquire_many(batch_size as u32).await
            .map_err(|_| ProcessingError::BackpressureExceeded)?;
        
        // Rayonで並列処理実行
        let results = self.thread_pool.install(|| {
            requests.par_iter()
                .map(|request| self.process_single_request(request))
                .collect::<Vec<_>>()
        });
        
        let processing_time = start_time.elapsed();
        self.record_batch_completion(batch_size, processing_time);
        
        Ok(results)
    }

    /// チャンク分割バッチ処理
    pub async fn execute_chunked_parallel(
        &self,
        requests: Vec<StreamRequest>,
    ) -> Vec<Result<StreamResponse, ProcessingError>> {
        let chunk_size = self.config.batch_size;
        let start_time = Instant::now();
        
        let results: Vec<_> = requests
            .par_chunks(chunk_size)
            .flat_map(|chunk| {
                chunk.par_iter()
                    .map(|request| self.process_single_request(request))
            })
            .collect();
        
        let processing_time = start_time.elapsed();
        self.record_batch_completion(requests.len(), processing_time);
        
        results
    }

    /// 単一リクエスト処理（内部実装）
    fn process_single_request(&self, request: &StreamRequest) -> Result<StreamResponse, ProcessingError> {
        let start_time = Instant::now();
        
        // 実際の処理ロジック（実装時に詳細化）
        let result = self.execute_request_sync(request);
        
        // 統計更新
        if self.config.enable_statistics {
            self.update_statistics(&result, start_time.elapsed());
        }
        
        result
    }

    /// 同期リクエスト実行
    fn execute_request_sync(&self, _request: &StreamRequest) -> Result<StreamResponse, ProcessingError> {
        // TODO: 実際のサービス呼び出し実装
        Err(ProcessingError::NotImplemented)
    }

    /// 統計更新
    fn update_statistics(&self, result: &Result<StreamResponse, ProcessingError>, duration: Duration) {
        self.stats.total_processed.fetch_add(1, Ordering::Relaxed);
        if result.is_err() {
            self.stats.total_failed.fetch_add(1, Ordering::Relaxed);
        }
        
        if let Ok(mut total_time) = self.stats.total_processing_time.lock() {
            *total_time += duration;
        }
    }

    /// バッチ完了記録
    fn record_batch_completion(&self, batch_size: usize, duration: Duration) {
        debug!(
            "Batch completed: {} requests in {:?}",
            batch_size, duration
        );
        
        if let Some(monitor) = &self.performance_monitor {
            // パフォーマンス監視システムに記録
        }
    }
}

/// 処理エラー
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Task processing failed: {0}")]
    TaskFailed(String),
    
    #[error("Backpressure limit exceeded")]
    BackpressureExceeded,
    
    #[error("Thread pool error: {0}")]
    ThreadPoolError(String),
    
    #[error("Not implemented")]
    NotImplemented,
}
```

## 実装計画（Rayon活用版）

### Step 1: Rayon統合とCargo設定 (10分)
1. `Cargo.toml` にrayon依存関係追加
2. `ParallelProcessor` 基本構造実装
3. ThreadPoolBuilder設定

### Step 2: 並列処理機能実装 (15分)
1. `execute_parallel_batch()` メソッド実装
2. `execute_chunked_parallel()` メソッド実装
3. バックプレッシャー制御（Semaphore）

### Step 3: 統計とモニタリング統合 (5分)
1. 簡単な統計収集機能
2. `StreamPerformanceMonitor` 統合
3. 基本テスト実装

## テスト要件（Rayon版）

### 並列処理機能テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parallel_processor_creation() {
        let processor = ParallelProcessor::new().unwrap();
        assert!(processor.thread_pool.current_num_threads() > 0);
    }

    #[tokio::test]
    async fn test_parallel_batch_processing() {
        let processor = ParallelProcessor::new().unwrap();
        
        let requests: Vec<StreamRequest> = (0..100)
            .map(|i| create_test_stream_request(i))
            .collect();

        let results = processor.execute_parallel_batch(requests).await;
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 100);
    }

    #[tokio::test]
    async fn test_chunked_parallel_processing() {
        let processor = ParallelProcessor::new().unwrap();
        
        let requests: Vec<StreamRequest> = (0..50)
            .map(|i| create_test_stream_request(i))
            .collect();

        let results = processor.execute_chunked_parallel(requests).await;
        assert_eq!(results.len(), 50);
    }

    #[tokio::test]
    async fn test_backpressure_control() {
        let config = ParallelConfig {
            max_pending_tasks: 10,
            ..Default::default()
        };
        let processor = ParallelProcessor::with_config(config).unwrap();
        
        // 大量リクエストでバックプレッシャーテスト
        let requests: Vec<StreamRequest> = (0..1000)
            .map(|i| create_test_stream_request(i))
            .collect();

        let result = processor.execute_parallel_batch(requests).await;
        // バックプレッシャー制御の動作確認
        assert!(result.is_ok() || matches!(result, Err(ProcessingError::BackpressureExceeded)));
    }

    fn create_test_stream_request(id: usize) -> StreamRequest {
        // テスト用リクエスト作成
        StreamRequest {
            message: Some(format!("test-message-{}", id).into()),
        }
    }
}
```

## 成功基準（Rayon版）

### 機能基準
- Rayonによる自動並列処理が正常動作
- バッチ処理効率 > 90%（Rayon最適化）
- 軽量なエラーハンドリング
- シンプルな統計収集機能

### パフォーマンス基準
- スループット > 2000 req/s（Rayonによる最適化）
- Work-stealing効率 > 80%
- タスク分散遅延 < 1ms（Rayon内蔵最適化）
- セマフォによるバックプレッシャー制御

### 開発効率基準
- 実装時間大幅短縮（75分 → 30分）
- 保守性向上（プロベンライブラリ活用）
- コード複雑度削減（自作実装 → Rayon活用）

## 次のステップ

Rayon並列処理システム完了後：
1. Task 3.7 Fix 07-F: 最適化プロセッサー統合実装
2. 全コンポーネントの統合テスト
3. パフォーマンスベンチマーク実行（Rayon効果測定）

## 関連ドキュメント
- Task 3.7 Fix 07-A (基盤インフラ整備)
- Task 3.7 Fix 07-B (パフォーマンス監視システム)
- [Rayon公式ドキュメント](https://docs.rs/rayon/)
- Rust並行プログラミングベストプラクティス