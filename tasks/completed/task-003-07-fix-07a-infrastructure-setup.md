# Task 3.7 Fix 07-A: 基盤インフラ整備

## 概要
パフォーマンス最適化実装の基盤となるインフラを整備します。必要な依存関係の追加、モジュール構造の定義、基本的な設定管理システムを構築し、後続のパフォーマンス最適化タスクの土台を作成します。

## 優先度
**🔴 最高優先度** - 全ての後続タスクの前提条件

## 実装時間見積もり
**45分** - 集中作業時間

## 受け入れ基準

### 依存関係要件
- [ ] Cargo.tomlに必要なパフォーマンス最適化ライブラリを追加
- [ ] すべての依存関係が正常にビルドできることを確認
- [ ] バージョン互換性の検証完了

### モジュール構造要件
- [ ] `src/grpc/performance.rs` モジュール定義ファイルの作成
- [ ] `src/grpc/performance/` ディレクトリ構造の確立
- [ ] 各サブモジュールの公開インターフェース定義

### 設定管理要件
- [ ] `OptimizationConfig` 構造体の定義
- [ ] 環境に応じたデフォルト値の設定
- [ ] 設定可能パラメータの文書化

### コードベース統合要件
- [ ] 既存のservice.rsとの互換性確保
- [ ] ビルドエラーなしでの正常コンパイル
- [ ] 既存テストの継続実行可能性

## 技術的詳細

### 追加する依存関係

#### Cargo.toml への追加
```toml
[dependencies]
# パフォーマンス最適化用ライブラリ
lru = "0.12"                    # LRUキャッシュ実装
num_cpus = "1.16"              # CPU コア数取得
futures = "0.3"                # 高度な並行処理機能
tokio-util = { version = "0.7", features = ["rt"] } # Tokio拡張機能

# メトリクス・監視用
metrics = "0.21"               # メトリクス収集フレームワーク
metrics-exporter-prometheus = "0.12" # Prometheus エクスポート
```

バージョンは仮です。最新リリースバージョンを使用したいのでcargo addコマンドで追加します。

### モジュール構造定義

#### src/grpc/performance.rs
```rust
//! パフォーマンス最適化モジュール
//! 
//! このモジュールは Unity MCP Server の gRPC ストリーミングサービスの
//! パフォーマンスを大幅に向上させるための最適化コンポーネントを提供します。

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tonic::{Request, Response, Status, Streaming};

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
```

#### src/grpc/performance/config.rs
```rust
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
```

### モジュール統合

#### src/grpc/mod.rs への追加
```rust
pub mod performance;
```

#### src/lib.rs への追加
既存のgrpcモジュール公開に含まれるため追加不要。

## 実装計画

### Step 1: 依存関係追加 (10分)
1. Cargo.toml の更新
2. ビルド確認とエラー対応

### Step 2: モジュール構造構築 (15分)
1. performance.rs モジュール定義作成
2. performance/ ディレクトリ作成
3. config.rs 基本実装

### Step 3: 設定システム実装 (15分)
1. OptimizationConfig の詳細実装
2. デフォルト値とバリデーション
3. 各種プリセット設定

### Step 4: 統合テスト (5分)
1. コンパイル確認
2. 既存テストの実行確認
3. モジュール公開API確認

## テスト要件

### 設定テスト
```rust
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
    }
}
```

## 成功基準

### 技術基準
- すべての依存関係が正常にビルド
- 新しいモジュール構造が適切に定義
- 設定システムが期待通り動作
- 既存のコードに影響なし

### パフォーマンス基準
- ビルド時間の大幅な増加なし（<10%）
- 基本機能のオーバーヘッド最小限
- メモリフットプリント増加最小限

## 次のステップ

基盤インフラ整備完了後：
1. Task 3.7 Fix 07-B: パフォーマンス監視システム実装
2. 各最適化コンポーネントの段階的実装
3. 統合テストとベンチマーク実行

## 関連ドキュメント
- Task 3.7 Fix 07 (元のパフォーマンス最適化仕様)
- 現在のservice.rs実装レビュー結果
- Rust パフォーマンス最適化ベストプラクティス