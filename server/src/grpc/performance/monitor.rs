//! パフォーマンス監視モジュール
//! 
//! ストリーミングパフォーマンスの監視機能を提供します。

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::grpc::performance::OptimizationResult;

/// ストリームパフォーマンス監視
pub struct StreamPerformanceMonitor {
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

#[derive(Debug, Default)]
struct PerformanceMetrics {
    request_count: u64,
    total_latency: Duration,
    latencies: Vec<Duration>,
    memory_usage: usize,
    cache_hits: u64,
    cache_misses: u64,
}

impl StreamPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    pub async fn record_request(&self, latency: Duration) {
        // 構造化メトリクス記録
        metrics::counter!("requests_total").increment(1);
        metrics::histogram!("request_duration_seconds").record(latency.as_secs_f64());
        
        let mut metrics = self.metrics.write().await;
        metrics.request_count += 1;
        metrics.total_latency += latency;
        metrics.latencies.push(latency);
        
        // 最新の1000件のみ保持
        if metrics.latencies.len() > 1000 {
            let excess_count = metrics.latencies.len() - 1000;
            metrics.latencies.drain(0..excess_count);
        }
    }

    pub async fn record_cache_hit(&self) {
        metrics::counter!("cache_hits_total").increment(1);
        
        let mut metrics = self.metrics.write().await;
        metrics.cache_hits += 1;
    }

    pub async fn record_cache_miss(&self) {
        metrics::counter!("cache_misses_total").increment(1);
        
        let mut metrics = self.metrics.write().await;
        metrics.cache_misses += 1;
    }

    pub async fn update_memory_usage(&self, bytes: usize) {
        metrics::gauge!("memory_usage_bytes").set(bytes as f64);
        
        let mut metrics = self.metrics.write().await;
        metrics.memory_usage = bytes;
    }

    pub async fn get_optimization_result(&self) -> OptimizationResult {
        let metrics = self.metrics.read().await;
        
        let avg_latency = if metrics.request_count > 0 {
            Duration::from_nanos(
                (metrics.total_latency.as_nanos() / metrics.request_count as u128) as u64
            )
        } else {
            Duration::ZERO
        };

        let mut sorted_latencies = metrics.latencies.clone();
        sorted_latencies.sort();

        let p95_latency = if !sorted_latencies.is_empty() {
            let idx = (sorted_latencies.len() as f64 * 0.95) as usize;
            sorted_latencies.get(idx).cloned().unwrap_or_default()
        } else {
            Duration::ZERO
        };

        let p99_latency = if !sorted_latencies.is_empty() {
            let idx = (sorted_latencies.len() as f64 * 0.99) as usize;
            sorted_latencies.get(idx).cloned().unwrap_or_default()
        } else {
            Duration::ZERO
        };

        let throughput = if !avg_latency.is_zero() {
            1.0 / avg_latency.as_secs_f64()
        } else {
            0.0
        };

        let cache_hit_ratio = if metrics.cache_hits + metrics.cache_misses > 0 {
            metrics.cache_hits as f64 / (metrics.cache_hits + metrics.cache_misses) as f64
        } else {
            0.0
        };

        OptimizationResult {
            throughput,
            avg_latency,
            p95_latency,
            p99_latency,
            memory_usage: metrics.memory_usage,
            cache_hit_ratio,
            worker_utilization: 0.0, // TODO: 実装
        }
    }
}