//! パフォーマンス監視システム
//! 
//! ストリーミングサービスのパフォーマンスを詳細に監視し、
//! 最適化の効果測定とデバッグのための情報を提供します。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::{VecDeque, HashMap};
use tokio::time::interval;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

use crate::grpc::performance::OptimizationResult;

/// パフォーマンス監視エラー
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Failed to acquire lock due to contention")]
    LockContention,
    #[error("Invalid metrics data: {0}")]
    InvalidData(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

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
#[derive(Debug, Default, Clone)]
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
    pub backpressure_events: u64,
    
    // キャッシュ統計（正確な管理）
    pub cache_hits: u64,
    pub cache_total_ops: u64,
    
    // タイミング
    pub monitoring_start: Option<Instant>,
    pub last_update: Option<Instant>,
}

impl PerformanceMetrics {
    /// キャッシュヒット率を計算
    pub fn cache_hit_ratio(&self) -> f64 {
        if self.cache_total_ops == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.cache_total_ops as f64
        }
    }
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
#[derive(Debug, Clone, Serialize)]
pub struct HistoricalDataPoint {
    #[serde(skip)]
    pub timestamp: Instant,
    pub timestamp_millis: u64, // Unix timestamp in milliseconds for serialization
    pub rps: f64,
    pub avg_latency_ms: f64,
    pub memory_mb: f64,
    pub active_connections: u64,
    pub error_rate: f64,
}

impl Default for HistoricalDataPoint {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            timestamp_millis: 0,
            rps: 0.0,
            avg_latency_ms: 0.0,
            memory_mb: 0.0,
            active_connections: 0,
            error_rate: 0.0,
        }
    }
}

/// セッション追跡
#[derive(Debug)]
pub struct SessionTracker {
    sessions: HashMap<String, SessionInfo>,
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

/// セッション統計
#[derive(Debug, Default)]
pub struct SessionStatistics {
    pub total_sessions: u64,
    pub active_sessions: u64,
    pub avg_session_duration: Duration,
    pub total_messages_processed: u64,
    pub total_bytes_processed: usize,
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
            history_retention: Duration::from_secs(24 * 60 * 60),
            throughput_window: Duration::from_secs(10),
            max_latency_samples: 1000,
            enable_detailed_logging: false,
            anomaly_detection_threshold: 2.0, // 標準偏差の倍数
        }
    }
}

/// 移動平均計算機
#[derive(Debug)]
pub struct MovingAverage {
    samples: VecDeque<f64>,
    window_size: usize,
    sum: f64,
}

impl MovingAverage {
    pub fn new(window_size: usize) -> Self {
        Self {
            samples: VecDeque::new(),
            window_size,
            sum: 0.0,
        }
    }

    pub fn add_sample(&mut self, value: f64) {
        // NaN/Infinity値をスキップ
        if !value.is_finite() {
            return;
        }
        
        self.samples.push_back(value);
        self.sum += value;

        if self.samples.len() > self.window_size {
            if let Some(old_value) = self.samples.pop_front() {
                self.sum -= old_value;
            }
        }
    }

    pub fn get_average(&self) -> f64 {
        if self.samples.is_empty() {
            0.0
        } else {
            self.sum / self.samples.len() as f64
        }
    }
}

/// パフォーマンス異常検出
#[derive(Debug)]
pub struct AnomalyDetector {
    baseline_mean: f64,
    baseline_std: f64,
    threshold_multiplier: f64,
    sample_count: usize,
}

impl AnomalyDetector {
    pub fn new(threshold_multiplier: f64) -> Self {
        Self {
            baseline_mean: 0.0,
            baseline_std: 0.0,
            threshold_multiplier,
            sample_count: 0,
        }
    }

    pub fn add_sample(&mut self, value: f64) {
        // NaN/Infinity値をスキップ
        if !value.is_finite() {
            return;
        }
        
        // 整数オーバーフロー保護
        self.sample_count = self.sample_count.saturating_add(1);
        let delta = value - self.baseline_mean;
        self.baseline_mean += delta / self.sample_count as f64;
        
        if self.sample_count > 1 {
            let delta2 = value - self.baseline_mean;
            // NaN/Infinity保護と安全な分散計算
            if self.baseline_std.is_finite() && delta.is_finite() && delta2.is_finite() {
                let prev_count = self.sample_count.saturating_sub(1);
                let variance = (prev_count as f64 * self.baseline_std.powi(2) + delta * delta2) / self.sample_count as f64;
                if variance >= 0.0 && variance.is_finite() {
                    self.baseline_std = variance.sqrt();
                }
            }
        }
    }

    pub fn is_anomaly(&self, value: f64) -> bool {
        if self.sample_count < 10 {
            return false; // 十分なサンプルがない場合は異常とは判定しない
        }
        
        let threshold = self.baseline_std * self.threshold_multiplier;
        (value - self.baseline_mean).abs() > threshold
    }
}

/// 時間別集約データ
#[derive(Debug, Clone)]
pub struct HourlyAggregate {
    pub timestamp: Instant,
    pub avg_rps: f64,
    pub avg_latency_ms: f64,
    pub peak_memory_mb: f64,
    pub total_requests: u64,
    pub error_count: u64,
}

/// 日別集約データ
#[derive(Debug, Clone)]
pub struct DailyAggregate {
    pub timestamp: Instant,
    pub avg_rps: f64,
    pub avg_latency_ms: f64,
    pub peak_memory_mb: f64,
    pub total_requests: u64,
    pub error_count: u64,
    pub uptime_percentage: f64,
}

impl RealTimeStats {
    pub fn new(config: &MonitoringConfig) -> Self {
        // 事前容量予約によるメモリ最適化
        let mut request_timestamps = VecDeque::new();
        let estimated_capacity = (config.throughput_window.as_secs() * 1000) as usize;
        request_timestamps.reserve(estimated_capacity.min(10000)); // 最大10K要素まで
        
        let mut latency_samples = VecDeque::new();
        latency_samples.reserve(config.max_latency_samples);
        
        Self {
            request_timestamps,
            throughput_window: config.throughput_window,
            latency_samples,
            max_latency_samples: config.max_latency_samples,
            rps_moving_avg: MovingAverage::new(10),
            latency_moving_avg: MovingAverage::new(100),
            anomaly_detector: AnomalyDetector::new(config.anomaly_detection_threshold),
        }
    }

    pub fn update_throughput(&mut self, message_count: u64, _duration: Duration) {
        let now = Instant::now();
        
        // 古いタイムスタンプを削除
        while let Some(&front_time) = self.request_timestamps.front() {
            if now.duration_since(front_time) > self.throughput_window {
                self.request_timestamps.pop_front();
            } else {
                break;
            }
        }
        
        // 新しいリクエストを記録
        for _ in 0..message_count {
            self.request_timestamps.push_back(now);
        }
    }

    pub fn update_latency(&mut self, latency: Duration) {
        self.latency_samples.push_back(latency);
        
        if self.latency_samples.len() > self.max_latency_samples {
            self.latency_samples.pop_front();
        }

        let latency_ms = latency.as_secs_f64() * 1000.0;
        self.latency_moving_avg.add_sample(latency_ms);
        self.anomaly_detector.add_sample(latency_ms);
    }

    pub fn get_current_rps(&self) -> f64 {
        if self.throughput_window.is_zero() {
            return 0.0;
        }
        self.request_timestamps.len() as f64 / self.throughput_window.as_secs_f64()
    }

    pub fn calculate_latency_percentiles(&self) -> (f64, f64) {
        if self.latency_samples.is_empty() {
            return (0.0, 0.0);
        }

        let mut sorted_latencies: Vec<_> = self.latency_samples
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .filter(|&x| x.is_finite()) // NaN/Infinity値を除外
            .collect();
        
        if sorted_latencies.is_empty() {
            return (0.0, 0.0);
        }
        
        // unwrap除去と安全な比較
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = sorted_latencies.len();
        let p95_idx = ((len as f64 * 0.95).ceil() as usize).saturating_sub(1).min(len - 1);
        let p99_idx = ((len as f64 * 0.99).ceil() as usize).saturating_sub(1).min(len - 1);

        (sorted_latencies[p95_idx], sorted_latencies[p99_idx])
    }

    pub fn get_current_stats(&self) -> RealTimeStatsSnapshot {
        RealTimeStatsSnapshot {
            current_rps: self.get_current_rps(),
            avg_latency_ms: self.latency_moving_avg.get_average(),
            active_requests: self.request_timestamps.len(),
        }
    }
}

#[derive(Debug, Default)]
pub struct RealTimeStatsSnapshot {
    pub current_rps: f64,
    pub avg_latency_ms: f64,
    pub active_requests: usize,
}

impl HistoricalData {
    pub fn new(config: &MonitoringConfig) -> Self {
        let max_history_size = (config.history_retention.as_secs() / config.collection_interval.as_secs()) as usize;
        
        Self {
            data_points: VecDeque::new(),
            max_history_size,
            hourly_aggregates: VecDeque::new(),
            daily_aggregates: VecDeque::new(),
        }
    }

    pub fn add_data_point(&mut self, data_point: HistoricalDataPoint) {
        // 環状バッファの正確な実装
        while self.data_points.len() >= self.max_history_size {
            self.data_points.pop_front();
        }
        self.data_points.push_back(data_point);
    }
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_stats: SessionStatistics::default(),
        }
    }

    pub fn start_session(&mut self, connection_id: String) {
        let session_info = SessionInfo {
            connection_id: connection_id.clone(),
            start_time: Instant::now(),
            message_count: 0,
            last_activity: Instant::now(),
            bytes_processed: 0,
        };
        
        self.sessions.insert(connection_id, session_info);
        self.session_stats.active_sessions += 1;
        self.session_stats.total_sessions += 1;
    }

    pub fn complete_session(&mut self, connection_id: String, message_count: u64, bytes_processed: usize) {
        if let Some(_session) = self.sessions.remove(&connection_id) {
            self.session_stats.active_sessions = self.session_stats.active_sessions.saturating_sub(1);
            self.session_stats.total_messages_processed += message_count;
            self.session_stats.total_bytes_processed += bytes_processed;
        }
    }

    pub fn get_statistics(&self) -> &SessionStatistics {
        &self.session_stats
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
            sessions.complete_session(connection_id.clone(), message_count, bytes_processed);
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

    /// キャッシュ統計を直接設定（レガシー互換性のため）
    pub fn update_cache_hit_ratio(&self, hit_ratio: f64) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            // 既存のAPIとの互換性のため、概算で逆算
            if hit_ratio >= 0.0 && hit_ratio <= 1.0 {
                let estimated_total = if metrics.cache_total_ops > 0 { 
                    metrics.cache_total_ops 
                } else { 
                    100 
                };
                metrics.cache_hits = (estimated_total as f64 * hit_ratio) as u64;
                metrics.cache_total_ops = estimated_total;
            }
        }
    }

    /// 現在のメトリクスを取得
    pub fn get_current_metrics(&self) -> Result<PerformanceMetrics, MonitoringError> {
        self.current_metrics.lock()
            .map(|m| m.clone())
            .map_err(|_| MonitoringError::LockContention)
    }

    /// パフォーマンスサマリーを取得
    pub fn get_performance_summary(&self) -> Result<PerformanceSummary, MonitoringError> {
        let metrics = self.get_current_metrics()?;
        let real_time = self.real_time_stats.lock()
            .map(|s| s.get_current_stats())
            .map_err(|_| MonitoringError::LockContention)?;

        Ok(PerformanceSummary {
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
            cache_efficiency: metrics.cache_hit_ratio(),
            uptime_seconds: metrics.monitoring_start
                .map(|start| start.elapsed().as_secs())
                .unwrap_or(0),
        })
    }

    /// 定期的なメトリクス更新を開始
    fn start_periodic_updates(&self) {
        let current_metrics = Arc::clone(&self.current_metrics);
        let real_time_stats = Arc::clone(&self.real_time_stats);
        let historical_data = Arc::clone(&self.historical_data);
        let interval_duration = self.monitoring_config.collection_interval;

        tokio::spawn(async move {
            let mut ticker = interval(interval_duration);
            
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
                    let now = Instant::now();
                    let data_point = HistoricalDataPoint {
                        timestamp: now,
                        timestamp_millis: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64,
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

    // 既存のメソッドとの互換性のために残す
    pub fn record_request(&self, latency: Duration) {
        let connection_id = format!("legacy-{}", Instant::now().elapsed().as_nanos());
        self.record_stream_session(connection_id, 1, latency, 0);
    }

    pub fn record_cache_hit(&self) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.cache_hits = metrics.cache_hits.saturating_add(1);
            metrics.cache_total_ops = metrics.cache_total_ops.saturating_add(1);
        }
    }

    pub fn record_cache_miss(&self) {
        if let Ok(mut metrics) = self.current_metrics.lock() {
            metrics.cache_total_ops = metrics.cache_total_ops.saturating_add(1);
        }
    }

    pub fn get_optimization_result(&self) -> Result<OptimizationResult, MonitoringError> {
        let summary = self.get_performance_summary()?;
        Ok(OptimizationResult {
            throughput: summary.throughput_rps,
            avg_latency: Duration::from_secs_f64(summary.avg_latency_ms / 1000.0),
            p95_latency: Duration::from_secs_f64(summary.p95_latency_ms / 1000.0),
            p99_latency: Duration::from_secs_f64(summary.p99_latency_ms / 1000.0),
            memory_usage: summary.memory_usage_mb as usize * 1024 * 1024,
            cache_hit_ratio: summary.cache_efficiency,
            worker_utilization: summary.worker_utilization,
        })
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
            target_memory_mb: 100.0,
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
        
        let metrics = monitor.get_current_metrics().expect("Should get metrics");
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

        let summary = monitor.get_performance_summary().expect("Should get summary");
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
            memory_usage_mb: 80.0,
            ..Default::default()
        };

        let targets = PerformanceTargets::default();
        let comparison = summary.compare_with_targets(&targets);
        
        assert!(comparison.throughput_achievement > 1.0);
        assert!(comparison.latency_achievement > 1.0);
        assert!(comparison.overall_score > 1.0);
    }

    #[test]
    fn test_moving_average() {
        let mut avg = MovingAverage::new(3);
        
        avg.add_sample(1.0);
        assert_eq!(avg.get_average(), 1.0);
        
        avg.add_sample(2.0);
        assert_eq!(avg.get_average(), 1.5);
        
        avg.add_sample(3.0);
        assert_eq!(avg.get_average(), 2.0);
        
        avg.add_sample(4.0); // ウィンドウサイズを超える
        assert_eq!(avg.get_average(), 3.0);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut detector = AnomalyDetector::new(2.0);
        
        // ベースラインを確立
        for i in 1..=10 {
            detector.add_sample(i as f64);
        }
        
        // 正常値は異常として検出されない
        assert!(!detector.is_anomaly(5.0));
        
        // 明らかに異常な値は検出される
        assert!(detector.is_anomaly(100.0));
    }
    
    #[test]
    fn test_anomaly_detector_edge_cases() {
        let mut detector = AnomalyDetector::new(2.0);
        
        // NaN/Infinityテスト - 正常動作を確認
        detector.add_sample(f64::NAN);
        detector.add_sample(f64::INFINITY);
        detector.add_sample(f64::NEG_INFINITY);
        
        // サンプルカウントが変更されていないことを確認
        assert_eq!(detector.sample_count, 0);
        
        // 正常値を追加
        for i in 1..=15 {
            detector.add_sample(i as f64);
        }
        
        // 正常動作を確認
        assert!(!detector.is_anomaly(8.0));
        assert!(detector.is_anomaly(1000.0));
        
        // 整数オーバーフローテスト
        detector.sample_count = usize::MAX - 1;
        detector.add_sample(5.0); // saturating_addにより安全
        assert_eq!(detector.sample_count, usize::MAX);
    }
    
    #[tokio::test]
    async fn test_percentile_calculation_edge_cases() {
        let monitor = StreamPerformanceMonitor::new();
        let real_time_stats = monitor.real_time_stats.lock().unwrap();
        
        // 空のサンプルでのテスト
        let (p95, p99) = real_time_stats.calculate_latency_percentiles();
        assert_eq!(p95, 0.0);
        assert_eq!(p99, 0.0);
        
        drop(real_time_stats);
        
        // 通常のデータでのテスト（NaN/Infinityは実際のDurationでは作成不可）
        let mut stats = monitor.real_time_stats.lock().unwrap();
        stats.latency_samples.push_back(Duration::from_millis(10));
        stats.latency_samples.push_back(Duration::from_millis(15));
        stats.latency_samples.push_back(Duration::from_millis(20));
        stats.latency_samples.push_back(Duration::from_millis(25));
        stats.latency_samples.push_back(Duration::from_millis(30));
        
        let (p95, p99) = stats.calculate_latency_percentiles();
        // 正常値で計算されることを確認
        assert!(p95 > 0.0 && p95.is_finite());
        assert!(p99 > 0.0 && p99.is_finite());
        assert!(p99 >= p95); // P99はP95以上であることを確認
    }
    
    #[test]
    fn test_moving_average_safety() {
        let mut avg = MovingAverage::new(3);
        
        // NaN/Infinityが除外されることをテスト
        avg.add_sample(1.0);
        avg.add_sample(f64::NAN);
        avg.add_sample(2.0);
        avg.add_sample(f64::INFINITY);
        avg.add_sample(3.0);
        
        // NaN/Infinityは除外され、正常値のみで平均計算
        assert_eq!(avg.get_average(), 2.0); // (1.0 + 2.0 + 3.0) / 3 = 2.0
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let monitor = StreamPerformanceMonitor::new();
        
        // 正常ケース
        assert!(monitor.get_current_metrics().is_ok());
        assert!(monitor.get_performance_summary().is_ok());
        assert!(monitor.get_optimization_result().is_ok());
    }
    
    #[tokio::test]
    async fn test_cache_hit_ratio_accuracy() {
        let monitor = StreamPerformanceMonitor::new();
        
        // キャッシュヒットとミスを記録
        monitor.record_cache_hit();
        monitor.record_cache_hit();
        monitor.record_cache_miss();
        
        let metrics = monitor.get_current_metrics().expect("Should get metrics");
        assert_eq!(metrics.cache_hits, 2);
        assert_eq!(metrics.cache_total_ops, 3);
        assert!((metrics.cache_hit_ratio() - 2.0/3.0).abs() < f64::EPSILON);
    }
}

impl Default for PerformanceSummary {
    fn default() -> Self {
        Self {
            throughput_rps: 0.0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            success_rate: 1.0,
            memory_usage_mb: 0.0,
            active_connections: 0,
            worker_utilization: 0.0,
            cache_efficiency: 0.0,
            uptime_seconds: 0,
        }
    }
}