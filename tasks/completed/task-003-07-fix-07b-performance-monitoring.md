# Task 3.7 Fix 07-B: パフォーマンス監視システム

## 概要
ストリーミングサービスのパフォーマンス監視システムを実装します。リアルタイムメトリクス収集、統計分析、履歴データ管理を通じて、最適化の効果測定と継続的改善のための基盤を提供します。

## 優先度
**🟡 高優先度** - 最適化効果の測定とデバッグに必須

## 実装時間見積もり
**60分** - 集中作業時間

## 依存関係
- Task 3.7 Fix 07-A (基盤インフラ整備) 完了必須

## 受け入れ基準

### 監視機能要件
- [ ] リアルタイムパフォーマンス監視
- [ ] スループット、レイテンシー、メモリ使用量の測定
- [ ] 同時接続数とワーカー利用率の追跡
- [ ] 統計データの履歴保持

### メトリクス収集要件
- [ ] P95/P99レイテンシー計算
- [ ] 毎秒リクエスト数（RPS）測定
- [ ] エラー率とキャッシュヒット率の計算
- [ ] バックプレッシャー発生回数の記録

### 統計分析要件
- [ ] 移動平均とトレンド分析
- [ ] パフォーマンス異常の検出
- [ ] リアルタイム統計の更新
- [ ] 定期的なレポート生成

### 監視データ出力要件
- [ ] 構造化ログ出力
- [ ] パフォーマンスサマリーの提供
- [ ] デバッグ用詳細メトリクス
- [ ] Prometheus互換メトリクス（オプション）

## 技術的詳細

### StreamPerformanceMonitor 実装

#### src/grpc/performance/monitor.rs
```rust
//! パフォーマンス監視システム
//! 
//! ストリーミングサービスのパフォーマンスを詳細に監視し、
//! 最適化の効果測定とデバッグのための情報を提供します。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use tokio::time::interval;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

/// 包括的なパフォーマンス監視システム
pub struct StreamPerformanceMonitor {
    // 現在のメトリクス
    current_metrics: Arc<Mutex<PerformanceMetrics>>,
    
    // リアルタイム統計
    real_time_stats: Arc<Mutex<RealTimeStats>>,
    
    // 履歴データ（環状バッファ）
    historical_data: Arc<Mutex<HistoricalData>>,
    
    // セッション追跡
    active_sessions: Arc<Mutex<SessionTracker>>,
    
    // 設定
    monitoring_config: MonitoringConfig,
}

/// 現在のパフォーマンスメトリクス
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // 基本統計
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub active_connections: u64,
    
    // レイテンシー統計
    pub latency_sum_ms: f64,
    pub latency_min_ms: f64,
    pub latency_max_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    
    // スループット統計
    pub current_rps: f64,
    pub peak_rps: f64,
    pub avg_rps: f64,
    
    // リソース使用量
    pub memory_usage_bytes: usize,
    pub peak_memory_bytes: usize,
    pub cpu_usage_percent: f64,
    
    // システム健全性
    pub worker_utilization: f64,
    pub queue_depth: usize,
    pub cache_hit_ratio: f64,
    pub backpressure_events: u64,
    
    // タイミング
    pub monitoring_start: Option<Instant>,
    pub last_update: Option<Instant>,
}

/// リアルタイム統計処理
#[derive(Debug)]
pub struct RealTimeStats {
    // スループット計算用
    request_timestamps: VecDeque<Instant>,
    throughput_window: Duration,
    
    // レイテンシー計算用
    latency_samples: VecDeque<Duration>,
    max_latency_samples: usize,
    
    // 移動平均
    rps_moving_avg: MovingAverage,
    latency_moving_avg: MovingAverage,
    
    // パフォーマンス異常検出
    anomaly_detector: AnomalyDetector,
}

/// 履歴データ管理
#[derive(Debug)]
pub struct HistoricalData {
    // データポイント（環状バッファ）
    data_points: VecDeque<HistoricalDataPoint>,
    max_history_size: usize,
    
    // 集約データ
    hourly_aggregates: VecDeque<HourlyAggregate>,
    daily_aggregates: VecDeque<DailyAggregate>,
}

/// 履歴データポイント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalDataPoint {
    pub timestamp: Instant,
    pub rps: f64,
    pub avg_latency_ms: f64,
    pub memory_mb: f64,
    pub active_connections: u64,
    pub error_rate: f64,
}

/// セッション追跡
#[derive(Debug)]
pub struct SessionTracker {
    sessions: std::collections::HashMap<String, SessionInfo>,
    session_stats: SessionStatistics,
}

/// 個別セッション情報
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub connection_id: String,
    pub start_time: Instant,
    pub message_count: u64,
    pub last_activity: Instant,
    pub bytes_processed: usize,
}

/// 監視設定
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub collection_interval: Duration,
    pub history_retention: Duration,
    pub throughput_window: Duration,
    pub max_latency_samples: usize,
    pub enable_detailed_logging: bool,
    pub anomaly_detection_threshold: f64,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(1),
            history_retention: Duration::from_hours(24),
            throughput_window: Duration::from_secs(10),
            max_latency_samples: 1000,
            enable_detailed_logging: false,
            anomaly_detection_threshold: 2.0, // 標準偏差の倍数
        }
    }
}

impl StreamPerformanceMonitor {
    /// 新しいパフォーマンス監視インスタンスを作成
    pub fn new() -> Self {
        Self::with_config(MonitoringConfig::default())
    }

    /// 設定付きでインスタンスを作成
    pub fn with_config(config: MonitoringConfig) -> Self {
        let monitor = Self {
            current_metrics: Arc::new(Mutex::new(PerformanceMetrics {
                monitoring_start: Some(Instant::now()),
                latency_min_ms: f64::INFINITY,
                ..Default::default()
            })),
            real_time_stats: Arc::new(Mutex::new(RealTimeStats::new(&config))),
            historical_data: Arc::new(Mutex::new(HistoricalData::new(&config))),
            active_sessions: Arc::new(Mutex::new(SessionTracker::new())),
            monitoring_config: config,
        };

        // 定期的なメトリクス更新を開始
        monitor.start_periodic_updates();
        
        info!("Performance monitoring system initialized");
        monitor
    }

    /// ストリームセッションを記録
    pub fn record_stream_session(
        &self,
        connection_id: String,
        message_count: u64,
        duration: Duration,
        bytes_processed: usize,
    ) {
        let avg_latency = if message_count > 0 {
            duration.as_secs_f64() * 1000.0 / message_count as f64
        } else {
            0.0
        };

        // 現在のメトリクスを更新
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.total_requests += message_count;
            metrics.successful_requests += message_count; // 簡略化
            
            // レイテンシー統計更新
            metrics.latency_sum_ms += duration.as_secs_f64() * 1000.0;
            metrics.latency_min_ms = metrics.latency_min_ms.min(avg_latency);
            metrics.latency_max_ms = metrics.latency_max_ms.max(avg_latency);
            
            metrics.last_update = Some(Instant::now());
        }

        // リアルタイム統計更新
        if let Ok(mut stats) = self.real_time_stats.lock() {
            stats.update_throughput(message_count, duration);
            stats.update_latency(Duration::from_secs_f64(avg_latency / 1000.0));
        }

        // セッション追跡更新
        if let Ok(mut sessions) = self.active_sessions.lock() {
            sessions.complete_session(connection_id, message_count, bytes_processed);
        }

        debug!(
            connection_id = %connection_id,
            message_count = message_count,
            duration_ms = duration.as_millis(),
            avg_latency_ms = avg_latency,
            "Stream session completed"
        );
    }

    /// バッチ処理を記録
    pub fn record_batch_processing(&self, batch_size: usize, duration: Duration) {
        let throughput = batch_size as f64 / duration.as_secs_f64();
        
        if let Ok(mut stats) = self.real_time_stats.lock() {
            stats.rps_moving_avg.add_sample(throughput);
        }

        debug!(
            batch_size = batch_size,
            duration_ms = duration.as_millis(),
            throughput = throughput,
            "Batch processing recorded"
        );
    }

    /// バックプレッシャーイベントを記録
    pub fn record_backpressure_event(&self) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.backpressure_events += 1;
        }

        warn!("Backpressure event recorded");
    }

    /// エラーを記録
    pub fn record_error(&self, error_type: &str) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.failed_requests += 1;
        }

        debug!(error_type = error_type, "Error recorded");
    }

    /// メモリ使用量を更新
    pub fn update_memory_usage(&self, current_bytes: usize) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.memory_usage_bytes = current_bytes;
            metrics.peak_memory_bytes = metrics.peak_memory_bytes.max(current_bytes);
        }
    }

    /// ワーカー利用率を更新
    pub fn update_worker_utilization(&self, utilization: f64) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.worker_utilization = utilization;
        }
    }

    /// キューの深さを更新
    pub fn update_queue_depth(&self, depth: usize) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.queue_depth = depth;
        }
    }

    /// キャッシュヒット率を更新
    pub fn update_cache_hit_ratio(&self, hit_ratio: f64) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.cache_hit_ratio = hit_ratio;
        }
    }

    /// 現在のメトリクスを取得
    pub fn get_current_metrics(&self) -> PerformanceMetrics {
        self.current_metrics.lock()
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    /// パフォーマンスサマリーを取得
    pub fn get_performance_summary(&self) -> PerformanceSummary {
        let metrics = self.get_current_metrics();
        let real_time = self.real_time_stats.lock()
            .map(|s| s.get_current_stats())
            .unwrap_or_default();

        PerformanceSummary {
            throughput_rps: real_time.current_rps,
            avg_latency_ms: if metrics.total_requests > 0 {
                metrics.latency_sum_ms / metrics.total_requests as f64
            } else {
                0.0
            },
            p95_latency_ms: metrics.latency_p95_ms,
            p99_latency_ms: metrics.latency_p99_ms,
            success_rate: if metrics.total_requests > 0 {
                metrics.successful_requests as f64 / metrics.total_requests as f64
            } else {
                1.0
            },
            memory_usage_mb: metrics.memory_usage_bytes as f64 / (1024.0 * 1024.0),
            active_connections: metrics.active_connections,
            worker_utilization: metrics.worker_utilization,
            cache_efficiency: metrics.cache_hit_ratio,
            uptime_seconds: metrics.monitoring_start
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0),
        }
    }

    /// 定期的なメトリクス更新を開始
    fn start_periodic_updates(&self) {
        let current_metrics = Arc::clone(&self.current_metrics);
        let real_time_stats = Arc::clone(&self.real_time_stats);
        let historical_data = Arc::clone(&self.historical_data);
        let interval = self.monitoring_config.collection_interval;

        tokio::spawn(async move {
            let mut ticker = interval(interval);
            
            loop {
                ticker.tick().await;
                
                // パーセンタイル計算
                if let (Ok(mut metrics), Ok(stats)) = (
                    current_metrics.lock(),
                    real_time_stats.lock()
                ) {
                    let percentiles = stats.calculate_latency_percentiles();
                    metrics.latency_p95_ms = percentiles.0;
                    metrics.latency_p99_ms = percentiles.1;
                    
                    // 現在のRPS更新
                    metrics.current_rps = stats.get_current_rps();
                    metrics.peak_rps = metrics.peak_rps.max(metrics.current_rps);
                }

                // 履歴データ追加
                if let (Ok(metrics), Ok(mut history)) = (
                    current_metrics.lock(),
                    historical_data.lock()
                ) {
                    let data_point = HistoricalDataPoint {
                        timestamp: Instant::now(),
                        rps: metrics.current_rps,
                        avg_latency_ms: if metrics.total_requests > 0 {
                            metrics.latency_sum_ms / metrics.total_requests as f64
                        } else {
                            0.0
                        },
                        memory_mb: metrics.memory_usage_bytes as f64 / (1024.0 * 1024.0),
                        active_connections: metrics.active_connections,
                        error_rate: if metrics.total_requests > 0 {
                            metrics.failed_requests as f64 / metrics.total_requests as f64
                        } else {
                            0.0
                        },
                    };
                    
                    history.add_data_point(data_point);
                }
            }
        });
    }
}

/// パフォーマンスサマリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub throughput_rps: f64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub success_rate: f64,           // 0.0-1.0
    pub memory_usage_mb: f64,
    pub active_connections: u64,
    pub worker_utilization: f64,     // 0.0-1.0
    pub cache_efficiency: f64,       // 0.0-1.0
    pub uptime_seconds: u64,
}

impl PerformanceSummary {
    /// パフォーマンス目標との比較
    pub fn compare_with_targets(&self, targets: &PerformanceTargets) -> PerformanceComparison {
        PerformanceComparison {
            throughput_achievement: self.throughput_rps / targets.target_rps,
            latency_achievement: targets.target_p95_latency_ms / self.p95_latency_ms,
            memory_efficiency: targets.target_memory_mb / self.memory_usage_mb,
            overall_score: self.calculate_overall_score(targets),
        }
    }

    fn calculate_overall_score(&self, targets: &PerformanceTargets) -> f64 {
        let throughput_score = (self.throughput_rps / targets.target_rps).min(2.0);
        let latency_score = (targets.target_p95_latency_ms / self.p95_latency_ms).min(2.0);
        let memory_score = (targets.target_memory_mb / self.memory_usage_mb).min(2.0);
        let success_score = self.success_rate;

        (throughput_score + latency_score + memory_score + success_score) / 4.0
    }
}

/// パフォーマンス目標
#[derive(Debug, Clone)]
pub struct PerformanceTargets {
    pub target_rps: f64,
    pub target_p95_latency_ms: f64,
    pub target_memory_mb: f64,
    pub target_success_rate: f64,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            target_rps: 2000.0,
            target_p95_latency_ms: 25.0,
            target_memory_mb: 100.0, // 仮の値
            target_success_rate: 0.99,
        }
    }
}

/// パフォーマンス比較結果
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    pub throughput_achievement: f64,  // 1.0 = 目標達成
    pub latency_achievement: f64,     // 1.0 = 目標達成  
    pub memory_efficiency: f64,       // 1.0 = 目標達成
    pub overall_score: f64,           // 全体スコア
}

// 補助的な構造体と実装は省略...
// （MovingAverage, AnomalyDetector, HourlyAggregate, DailyAggregate, SessionStatistics等）
```

## 実装計画

### Step 1: 基本構造実装 (20分)
1. StreamPerformanceMonitor の基本構造
2. PerformanceMetrics データ構造
3. 基本的なメトリクス記録機能

### Step 2: リアルタイム統計 (20分)
1. RealTimeStats 実装
2. 移動平均とパーセンタイル計算
3. スループット測定機能

### Step 3: 履歴データ管理 (10分)
1. HistoricalData 環状バッファ実装
2. データポイント記録機能
3. 定期的な集約処理

### Step 4: セッション追跡と統合 (10分)
1. SessionTracker 実装
2. 定期更新タスクの実装
3. パフォーマンスサマリー生成

## テスト要件

### 基本機能テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_stream_session_recording() {
        let monitor = StreamPerformanceMonitor::new();
        
        monitor.record_stream_session(
            "test-connection".to_string(),
            100,
            Duration::from_millis(50),
            1024,
        );
        
        let metrics = monitor.get_current_metrics();
        assert_eq!(metrics.total_requests, 100);
        assert!(metrics.latency_sum_ms > 0.0);
    }

    #[tokio::test]
    async fn test_performance_summary() {
        let monitor = StreamPerformanceMonitor::new();
        
        // 複数のセッションを記録
        for i in 0..10 {
            monitor.record_stream_session(
                format!("connection-{}", i),
                10,
                Duration::from_millis(20),
                512,
            );
        }

        let summary = monitor.get_performance_summary();
        assert!(summary.throughput_rps >= 0.0);
        assert!(summary.avg_latency_ms >= 0.0);
        assert_eq!(summary.success_rate, 1.0);
    }

    #[test]
    fn test_performance_targets_comparison() {
        let summary = PerformanceSummary {
            throughput_rps: 2500.0,
            p95_latency_ms: 20.0,
            success_rate: 0.995,
            ..Default::default()
        };

        let targets = PerformanceTargets::default();
        let comparison = summary.compare_with_targets(&targets);
        
        assert!(comparison.throughput_achievement > 1.0);
        assert!(comparison.latency_achievement > 1.0);
        assert!(comparison.overall_score > 1.0);
    }
}
```

## 成功基準

### 機能基準
- すべての主要メトリクスが正確に収集される
- リアルタイム統計が適切に更新される
- パフォーマンスサマリーが期待通り生成される
- 長時間運用でメモリリークがない

### パフォーマンス基準
- 監視オーバーヘッド < 2%
- メトリクス更新遅延 < 100ms
- 履歴データ管理の効率性

## 次のステップ

パフォーマンス監視システム完了後：
1. Task 3.7 Fix 07-C: リソースプール管理実装
2. 他の最適化コンポーネントでの監視統合
3. 最適化効果の継続的測定開始

## 関連ドキュメント
- Task 3.7 Fix 07-A (基盤インフラ整備)
- パフォーマンス目標仕様書
- メトリクス収集ベストプラクティス