//! パフォーマンス最適化モジュール
//! 
//! このモジュールは Unity MCP Server の gRPC ストリーミングサービスの
//! パフォーマンスを大幅に向上させるための最適化コンポーネントを提供します。

use std::time::Duration;

// パフォーマンス最適化のサブモジュール
pub mod config;
pub mod monitor;
pub mod resource_pool;
pub mod cache;
pub mod worker_pool;
pub mod processor;

// 公開 API
pub use config::OptimizationConfig;
pub use monitor::StreamPerformanceMonitor;
pub use resource_pool::ResourcePool;
pub use cache::StreamCache;
pub use worker_pool::WorkerPool;
pub use processor::OptimizedStreamProcessor;

/// パフォーマンス最適化の結果
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub throughput: f64,           // req/s
    pub avg_latency: Duration,     // 平均レイテンシー
    pub p95_latency: Duration,     // P95レイテンシー
    pub p99_latency: Duration,     // P99レイテンシー
    pub memory_usage: usize,       // メモリ使用量（bytes）
    pub cache_hit_ratio: f64,      // キャッシュヒット率
    pub worker_utilization: f64,   // ワーカー利用率
}

/// パフォーマンス最適化のエラー
#[derive(Debug, thiserror::Error)]
pub enum OptimizationError {
    #[error("Worker pool initialization failed: {0}")]
    WorkerPoolError(String),
    
    #[error("Cache initialization failed: {0}")]
    CacheError(String),
    
    #[error("Resource pool error: {0}")]
    ResourcePoolError(String),
    
    #[error("Performance monitoring error: {0}")]
    MonitoringError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// 最適化コンテキスト
#[derive(Debug, Clone)]
pub struct OptimizationContext {
    pub connection_id: String,
    pub session_start: std::time::Instant,
    pub message_count: u64,
    pub config: OptimizationConfig,
}

impl OptimizationContext {
    pub fn new(connection_id: String, config: OptimizationConfig) -> Self {
        Self {
            connection_id,
            session_start: std::time::Instant::now(),
            message_count: 0,
            config,
        }
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
    }

    pub fn session_duration(&self) -> Duration {
        self.session_start.elapsed()
    }
}