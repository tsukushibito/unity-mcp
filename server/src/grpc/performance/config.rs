//! パフォーマンス最適化設定
//! 
//! システムの動作を調整するための設定パラメータを提供します。

use std::time::Duration;
use serde::{Deserialize, Serialize};

/// パフォーマンス最適化の設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    // ワーカープール設定
    pub worker_count: usize,
    pub worker_queue_capacity: usize,
    pub worker_timeout: Duration,

    // バッチ処理設定
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub max_batch_wait: Duration,

    // キャッシュ設定
    pub cache_capacity: usize,
    pub cache_ttl: Duration,
    pub enable_cache_compression: bool,

    // リソースプール設定
    pub service_pool_size: usize,
    pub validator_pool_size: usize,
    pub buffer_pool_size: usize,
    pub buffer_initial_capacity: usize,

    // バックプレッシャー設定
    pub backpressure_threshold: f64,
    pub backpressure_window: Duration,
    pub max_concurrent_connections: usize,

    // パフォーマンス監視設定
    pub enable_metrics: bool,
    pub metrics_interval: Duration,
    pub enable_detailed_tracing: bool,

    // ストリーム処理設定
    pub stream_channel_capacity: usize,
    pub stream_timeout: Duration,
    pub enable_stream_compression: bool,
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get().max(2);
        
        Self {
            // ワーカープール設定（CPUコア数ベース）
            worker_count: cpu_count,
            worker_queue_capacity: 1000,
            worker_timeout: Duration::from_secs(30),

            // バッチ処理設定
            batch_size: 10,
            batch_timeout: Duration::from_millis(10),
            max_batch_wait: Duration::from_millis(50),

            // キャッシュ設定
            cache_capacity: 1000,
            cache_ttl: Duration::from_secs(300), // 5分
            enable_cache_compression: false,

            // リソースプール設定
            service_pool_size: 10,
            validator_pool_size: 5,
            buffer_pool_size: 50,
            buffer_initial_capacity: 8192,

            // バックプレッシャー設定
            backpressure_threshold: 0.8,
            backpressure_window: Duration::from_secs(1),
            max_concurrent_connections: 100,

            // パフォーマンス監視設定
            enable_metrics: true,
            metrics_interval: Duration::from_secs(10),
            enable_detailed_tracing: false,

            // ストリーム処理設定
            stream_channel_capacity: 10000,
            stream_timeout: Duration::from_secs(60),
            enable_stream_compression: false,
        }
    }
}

impl OptimizationConfig {
    /// 高パフォーマンス設定を作成
    pub fn high_performance() -> Self {
        let mut config = Self::default();
        config.worker_count = num_cpus::get() * 2;
        config.batch_size = 20;
        config.batch_timeout = Duration::from_millis(5);
        config.cache_capacity = 5000;
        config.backpressure_threshold = 0.9;
        config.enable_detailed_tracing = false; // オーバーヘッド削減
        config
    }

    /// メモリ効率重視設定を作成
    pub fn memory_efficient() -> Self {
        let mut config = Self::default();
        config.worker_count = 2;
        config.batch_size = 5;
        config.cache_capacity = 100;
        config.service_pool_size = 3;
        config.buffer_pool_size = 10;
        config.buffer_initial_capacity = 1024;
        config
    }

    /// デバッグ・開発用設定を作成
    pub fn development() -> Self {
        let mut config = Self::default();
        config.enable_detailed_tracing = true;
        config.metrics_interval = Duration::from_secs(1);
        config.worker_timeout = Duration::from_secs(5);
        config.stream_timeout = Duration::from_secs(10);
        config
    }

    /// 設定の妥当性を検証
    pub fn validate(&self) -> Result<(), String> {
        if self.worker_count == 0 {
            return Err("Worker count must be greater than 0".to_string());
        }

        if self.batch_size == 0 {
            return Err("Batch size must be greater than 0".to_string());
        }

        if self.cache_capacity == 0 {
            return Err("Cache capacity must be greater than 0".to_string());
        }

        if self.backpressure_threshold <= 0.0 || self.backpressure_threshold > 1.0 {
            return Err("Backpressure threshold must be between 0.0 and 1.0".to_string());
        }

        if self.max_concurrent_connections == 0 {
            return Err("Max concurrent connections must be greater than 0".to_string());
        }

        Ok(())
    }

    /// 現在の設定をログ出力
    pub fn log_configuration(&self) {
        tracing::info!(
            worker_count = self.worker_count,
            batch_size = self.batch_size,
            cache_capacity = self.cache_capacity,
            backpressure_threshold = self.backpressure_threshold,
            max_concurrent_connections = self.max_concurrent_connections,
            "Performance optimization configuration loaded"
        );
    }
}

/// パフォーマンスレベルの列挙
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PerformanceLevel {
    /// メモリ効率重視
    MemoryEfficient,
    /// バランス型（デフォルト）
    Balanced,
    /// 高パフォーマンス重視
    HighPerformance,
    /// カスタム設定
    Custom,
}

impl PerformanceLevel {
    pub fn to_config(self) -> OptimizationConfig {
        match self {
            Self::MemoryEfficient => OptimizationConfig::memory_efficient(),
            Self::Balanced => OptimizationConfig::default(),
            Self::HighPerformance => OptimizationConfig::high_performance(),
            Self::Custom => OptimizationConfig::default(), // カスタムは別途設定
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = OptimizationConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_high_performance_config() {
        let config = OptimizationConfig::high_performance();
        assert!(config.worker_count >= num_cpus::get());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_memory_efficient_config() {
        let config = OptimizationConfig::memory_efficient();
        assert_eq!(config.worker_count, 2);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_errors() {
        let mut config = OptimizationConfig::default();
        config.worker_count = 0;
        assert!(config.validate().is_err());
        
        let mut config = OptimizationConfig::default();
        config.batch_size = 0;
        assert!(config.validate().is_err());
        
        let mut config = OptimizationConfig::default();
        config.cache_capacity = 0;
        assert!(config.validate().is_err());
        
        let mut config = OptimizationConfig::default();
        config.backpressure_threshold = 1.5;
        assert!(config.validate().is_err());
        
        let mut config = OptimizationConfig::default();
        config.max_concurrent_connections = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_performance_level_to_config() {
        let mem_config = PerformanceLevel::MemoryEfficient.to_config();
        assert_eq!(mem_config.worker_count, 2);
        
        let high_perf_config = PerformanceLevel::HighPerformance.to_config();
        assert!(high_perf_config.worker_count >= num_cpus::get());
        
        let balanced_config = PerformanceLevel::Balanced.to_config();
        assert_eq!(balanced_config.worker_count, num_cpus::get().max(2));
    }
}