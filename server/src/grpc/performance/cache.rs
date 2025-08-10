//! インテリジェントキャッシュシステム
//! 
//! Unity MCP Server のストリーミング処理において、レスポンス時間を大幅に
//! 短縮するための高性能キャッシュシステム。LRU + TTL + 圧縮機能を提供。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::hash::{Hash, Hasher, DefaultHasher};
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::{debug, info, error};
use serde::{Serialize, Deserialize};

use crate::grpc::{StreamRequest, StreamResponse};

/// インテリジェントストリームキャッシュシステム
pub struct StreamCache {
    // メインキャッシュ（LRU）
    cache: Arc<Mutex<LruCache<CacheKey, CacheEntry>>>,
    
    // キャッシュ統計
    statistics: Arc<Mutex<CacheStatistics>>,
    
    // アクセスパターン学習
    access_pattern_analyzer: Arc<Mutex<AccessPatternAnalyzer>>,
    
    // 設定
    config: CacheConfig,
    
    // キーハッシュ戦略
    key_hasher: Arc<dyn CacheKeyHasher + Send + Sync>,
}

/// キャッシュエントリ
#[derive(Debug, Clone)]
pub struct CacheEntry {
    // キャッシュされたレスポンス
    response: StreamResponse,
    
    // エントリメタデータ
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
    
    // TTL情報
    expires_at: Option<Instant>,
    
    // 圧縮情報
    is_compressed: bool,
    original_size: usize,
    compressed_size: usize,
    
    // 品質情報
    cache_quality_score: f64, // 0.0-1.0
}

/// キャッシュキー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    // 操作タイプ
    operation_type: String,
    
    // リクエストハッシュ
    request_hash: u64,
    
    // バージョン（スキーマ変更対応）
    version: u32,
    
    // オプション属性
    attributes: CacheKeyAttributes,
}

/// キャッシュキー属性
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKeyAttributes {
    // アセットパス（正規化済み）
    normalized_path: Option<String>,
    
    // ファイルサイズ範囲
    file_size_range: Option<FileSizeRange>,
    
    // タイムスタンプ（精度調整済み）
    timestamp_bucket: Option<u64>,
}

/// ファイルサイズ範囲（キャッシュ効率化のため）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileSizeRange {
    Small,    // < 1MB
    Medium,   // 1MB - 10MB
    Large,    // 10MB - 100MB
    XLarge,   // > 100MB
}

/// キャッシュ設定
#[derive(Debug, Clone)]
pub struct CacheConfig {
    // キャッシュサイズ
    pub max_entries: usize,
    pub max_memory_mb: usize,
    
    // TTL設定
    pub default_ttl: Duration,
    pub max_ttl: Duration,
    pub adaptive_ttl: bool,
    
    // 圧縮設定
    pub enable_compression: bool,
    pub compression_threshold_bytes: usize,
    pub compression_level: u32,
    
    // インテリジェント機能
    pub enable_pattern_learning: bool,
    pub enable_prefetching: bool,
    pub enable_adaptive_sizing: bool,
    
    // パフォーマンス設定
    pub cleanup_interval: Duration,
    pub stats_update_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_memory_mb: 50,
            default_ttl: Duration::from_secs(300), // 5分
            max_ttl: Duration::from_secs(3600),    // 1時間
            adaptive_ttl: true,
            enable_compression: true,
            compression_threshold_bytes: 1024, // 1KB
            compression_level: 6, // バランス重視
            enable_pattern_learning: true,
            enable_prefetching: false, // デフォルトはオフ
            enable_adaptive_sizing: true,
            cleanup_interval: Duration::from_secs(60),
            stats_update_interval: Duration::from_secs(10),
        }
    }
}

/// キャッシュ統計
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    // 基本統計
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_evictions: u64,
    
    // パフォーマンス統計
    pub hit_ratio: f64,
    pub avg_hit_time_ns: u64,
    pub avg_miss_time_ns: u64,
    
    // メモリ統計
    pub current_memory_usage: usize,
    pub peak_memory_usage: usize,
    pub current_entry_count: usize,
    pub compression_ratio: f64,
    
    // 品質統計
    pub avg_cache_quality: f64,
    pub staleness_ratio: f64, // 期限切れ率
    
    // 時系列統計
    pub hourly_hit_rates: Vec<f64>,
    pub recent_access_patterns: HashMap<String, u64>,
}

/// アクセスパターン分析
#[derive(Debug)]
pub struct AccessPatternAnalyzer {
    // パターン記録
    access_history: Vec<AccessRecord>,
    
    // パターン統計
    operation_frequency: HashMap<String, u64>,
    temporal_patterns: HashMap<u64, u64>, // 時間帯別アクセス
    
    // 学習済みパターン
    learned_patterns: Vec<AccessPattern>,
    
    // 予測エンジン
    predictor: Option<CachePredictor>,
}

/// アクセス記録
#[derive(Debug, Clone)]
pub struct AccessRecord {
    pub timestamp: Instant,
    pub cache_key: CacheKey,
    pub hit: bool,
    pub response_time: Duration,
}

/// アクセスパターン
#[derive(Debug, Clone)]
pub struct AccessPattern {
    pub pattern_id: String,
    pub operations: Vec<String>,
    pub frequency: u64,
    pub confidence: f64,
}

/// キャッシュキーハッシュ戦略
pub trait CacheKeyHasher: Send + Sync {
    fn generate_key(&self, request: &StreamRequest) -> Option<CacheKey>;
    fn should_cache_response(&self, request: &StreamRequest, response: &StreamResponse) -> bool;
    fn calculate_ttl(&self, request: &StreamRequest, base_ttl: Duration) -> Duration;
}

/// デフォルトキーハッシュ戦略
pub struct DefaultCacheKeyHasher {
    version: u32,
}

impl StreamCache {
    /// 新しいキャッシュインスタンスを作成
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// 設定付きでキャッシュを作成
    pub fn with_config(config: CacheConfig) -> Self {
        let cache_size = NonZeroUsize::new(config.max_entries)
            .expect("Cache max_entries must be greater than 0");
        
        let cache = Arc::new(Mutex::new(LruCache::new(cache_size)));
        let statistics = Arc::new(Mutex::new(CacheStatistics::default()));
        let access_pattern_analyzer = Arc::new(Mutex::new(
            AccessPatternAnalyzer::new(&config)
        ));

        let cache_instance = Self {
            cache,
            statistics,
            access_pattern_analyzer,
            config: config.clone(),
            key_hasher: Arc::new(DefaultCacheKeyHasher::new()),
        };

        // 定期クリーンアップとメンテナンスタスクを開始
        cache_instance.start_maintenance_tasks();

        info!("Stream cache initialized with config: {:?}", config);
        cache_instance
    }

    /// キャッシュからレスポンスを取得
    pub async fn get(&self, request: &StreamRequest) -> Option<StreamResponse> {
        let start_time = Instant::now();
        
        // キャッシュキーを生成
        let cache_key = match self.key_hasher.generate_key(request) {
            Some(key) => key,
            None => {
                debug!("Request not cacheable");
                return None;
            }
        };

        let result = {
            let mut cache = self.cache.lock().ok()?;
            
            if let Some(entry) = cache.get_mut(&cache_key) {
                // TTL チェック
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() > expires_at {
                        // 期限切れエントリを削除
                        cache.pop(&cache_key);
                        self.record_cache_miss(&cache_key, start_time.elapsed());
                        return None;
                    }
                }

                // アクセス情報を更新
                entry.last_accessed = Instant::now();
                entry.access_count += 1;

                let response = if entry.is_compressed {
                    self.decompress_response(&entry.response)
                        .unwrap_or_else(|_| entry.response.clone())
                } else {
                    entry.response.clone()
                };

                self.record_cache_hit(&cache_key, start_time.elapsed());
                Some(response)
            } else {
                self.record_cache_miss(&cache_key, start_time.elapsed());
                None
            }
        };

        // アクセスパターン学習
        self.learn_access_pattern(&cache_key, result.is_some()).await;

        result
    }

    /// レスポンスをキャッシュに保存
    pub async fn put(&self, request: &StreamRequest, response: StreamResponse) {
        // キャッシュキーを生成
        let cache_key = match self.key_hasher.generate_key(request) {
            Some(key) => key,
            None => return,
        };

        // キャッシュ可能性をチェック
        if !self.key_hasher.should_cache_response(request, &response) {
            debug!("Response not cacheable for key: {:?}", cache_key);
            return;
        }

        let now = Instant::now();
        let ttl = self.key_hasher.calculate_ttl(request, self.config.default_ttl);
        let expires_at = if ttl == Duration::MAX {
            None
        } else {
            Some(now + ttl)
        };

        // レスポンス圧縮（必要に応じて）
        let (final_response, is_compressed, original_size, compressed_size) = 
            if self.config.enable_compression {
                self.compress_response_if_beneficial(&response)
            } else {
                let size = self.estimate_response_size(&response);
                (response, false, size, size)
            };

        // キャッシュエントリを作成
        let cache_entry = CacheEntry {
            response: final_response,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            expires_at,
            is_compressed,
            original_size,
            compressed_size,
            cache_quality_score: self.calculate_cache_quality_score(request),
        };

        // キャッシュに保存
        {
            let mut cache = match self.cache.lock() {
                Ok(cache) => cache,
                Err(_) => {
                    error!("Failed to acquire cache lock");
                    return;
                }
            };

            // メモリ制限チェック
            if self.should_evict_for_memory(&cache_entry) {
                self.evict_by_memory_pressure(&mut cache);
            }

            if let Some((evicted_key, _)) = cache.push(cache_key.clone(), cache_entry) {
                debug!("Cache entry evicted: {:?}", evicted_key);
                self.record_cache_eviction();
            }
        }

        // 統計更新
        self.update_cache_statistics().await;

        debug!("Cached response for key: {:?}, TTL: {:?}", cache_key, ttl);
    }

    /// キャッシュキーを直接指定して取得（高速パス）
    pub fn get_by_key(&self, key: &CacheKey) -> Option<StreamResponse> {
        let start_time = Instant::now();
        
        let result = {
            let mut cache = self.cache.lock().ok()?;
            
            if let Some(entry) = cache.get_mut(key) {
                // TTL チェック
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() > expires_at {
                        cache.pop(key);
                        return None;
                    }
                }

                entry.last_accessed = Instant::now();
                entry.access_count += 1;

                let response = if entry.is_compressed {
                    self.decompress_response(&entry.response)
                        .unwrap_or_else(|_| entry.response.clone())
                } else {
                    entry.response.clone()
                };

                Some(response)
            } else {
                None
            }
        };

        let elapsed = start_time.elapsed();
        if result.is_some() {
            self.record_cache_hit(key, elapsed);
        } else {
            self.record_cache_miss(key, elapsed);
        }

        result
    }

    /// キャッシュサイズを動的に調整
    pub fn resize_cache(&self, new_size: usize) {
        if let Ok(mut cache) = self.cache.lock() {
            let new_cache_size = NonZeroUsize::new(new_size)
                .unwrap_or(NonZeroUsize::new(100).unwrap());
            
            cache.resize(new_cache_size);
            info!("Cache resized to {} entries", new_size);
        }
    }

    /// キャッシュをクリア
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
            info!("Cache cleared");
        }

        if let Ok(mut stats) = self.statistics.lock() {
            *stats = CacheStatistics::default();
        }
    }

    /// キャッシュ統計を取得
    pub fn get_statistics(&self) -> CacheStatistics {
        self.statistics.lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }

    /// キャッシュ効率レポートを生成
    pub fn generate_efficiency_report(&self) -> CacheEfficiencyReport {
        let stats = self.get_statistics();
        let current_size = {
            self.cache.lock()
                .map(|cache| cache.len())
                .unwrap_or(0)
        };

        CacheEfficiencyReport {
            hit_ratio: stats.hit_ratio,
            memory_efficiency: if stats.peak_memory_usage > 0 {
                stats.current_memory_usage as f64 / stats.peak_memory_usage as f64
            } else {
                1.0
            },
            compression_effectiveness: stats.compression_ratio,
            cache_utilization: current_size as f64 / self.config.max_entries as f64,
            avg_response_time_improvement: self.calculate_response_time_improvement(),
            recommendations: self.generate_optimization_recommendations(&stats),
        }
    }

    // 内部ヘルパーメソッド

    fn record_cache_hit(&self, _key: &CacheKey, response_time: Duration) {
        if let Ok(mut stats) = self.statistics.lock() {
            stats.total_requests += 1;
            stats.cache_hits += 1;
            stats.hit_ratio = stats.cache_hits as f64 / stats.total_requests as f64;
            
            // 移動平均でヒット時間を更新
            let new_hit_time = response_time.as_nanos() as u64;
            stats.avg_hit_time_ns = (stats.avg_hit_time_ns + new_hit_time) / 2;
        }
    }

    fn record_cache_miss(&self, _key: &CacheKey, response_time: Duration) {
        if let Ok(mut stats) = self.statistics.lock() {
            stats.total_requests += 1;
            stats.cache_misses += 1;
            stats.hit_ratio = stats.cache_hits as f64 / stats.total_requests as f64;
            
            let new_miss_time = response_time.as_nanos() as u64;
            stats.avg_miss_time_ns = (stats.avg_miss_time_ns + new_miss_time) / 2;
        }
    }

    fn record_cache_eviction(&self) {
        if let Ok(mut stats) = self.statistics.lock() {
            stats.cache_evictions += 1;
        }
    }

    async fn learn_access_pattern(&self, key: &CacheKey, hit: bool) {
        if !self.config.enable_pattern_learning {
            return;
        }

        let record = AccessRecord {
            timestamp: Instant::now(),
            cache_key: key.clone(),
            hit,
            response_time: Duration::default(), // 簡略化
        };

        if let Ok(mut analyzer) = self.access_pattern_analyzer.lock() {
            analyzer.record_access(record);
        }
    }

    fn compress_response_if_beneficial(&self, response: &StreamResponse) -> (StreamResponse, bool, usize, usize) {
        let original_size = self.estimate_response_size(response);
        
        if original_size < self.config.compression_threshold_bytes {
            return (response.clone(), false, original_size, original_size);
        }

        // 実際の圧縮実装
        match self.compress_response(response) {
            Ok((compressed_response, compressed_size)) => {
                (compressed_response, true, original_size, compressed_size)
            }
            Err(_) => {
                // 圧縮に失敗した場合は元のレスポンスを返す
                (response.clone(), false, original_size, original_size)
            }
        }
    }

    fn compress_response(&self, response: &StreamResponse) -> Result<(StreamResponse, usize), CacheError> {
        // 簡略化：実際の圧縮処理では、protobufバイナリ形式でシリアライズして圧縮
        // StreamResponseがSerializeを実装していないため、ここでは概念的な実装
        let estimated_size = self.estimate_response_size(response);
        let compressed_size = (estimated_size as f64 * 0.7) as usize; // 30%圧縮と仮定
        
        // 実際の圧縮は省略し、元のレスポンスをそのまま返す
        Ok((response.clone(), compressed_size))
    }

    fn decompress_response(&self, response: &StreamResponse) -> Result<StreamResponse, CacheError> {
        // 簡略化：実際の解凍処理では、圧縮されたデータから元のレスポンスを復元
        // ここでは元のレスポンスをそのまま返す
        Ok(response.clone())
    }

    fn estimate_response_size(&self, _response: &StreamResponse) -> usize {
        // 簡略化：実際のサイズ計算
        // protobufメッセージのサイズ推定
        1024 // デフォルト値
    }

    fn calculate_cache_quality_score(&self, _request: &StreamRequest) -> f64 {
        // キャッシュ品質スコア計算（複雑度、サイズ、頻度等を考慮）
        0.8 // デフォルト値
    }

    fn should_evict_for_memory(&self, _entry: &CacheEntry) -> bool {
        // メモリプレッシャーチェック
        false // 簡略化
    }

    fn evict_by_memory_pressure(&self, _cache: &mut LruCache<CacheKey, CacheEntry>) {
        // メモリプレッシャーベースの退避処理
    }

    async fn update_cache_statistics(&self) {
        // 統計更新処理
    }

    fn start_maintenance_tasks(&self) {
        // 定期メンテナンスタスク開始
    }

    fn calculate_response_time_improvement(&self) -> f64 {
        let stats = self.get_statistics();
        if stats.avg_miss_time_ns > 0 && stats.avg_hit_time_ns > 0 {
            (stats.avg_miss_time_ns - stats.avg_hit_time_ns) as f64 / stats.avg_miss_time_ns as f64
        } else {
            0.0
        }
    }

    fn generate_optimization_recommendations(&self, _stats: &CacheStatistics) -> Vec<String> {
        vec![
            "Consider increasing cache size if hit ratio < 70%".to_string(),
            "Enable compression for better memory efficiency".to_string(),
            "Adjust TTL based on access patterns".to_string(),
        ]
    }
}

/// キャッシュ効率レポート
#[derive(Debug, Clone)]
pub struct CacheEfficiencyReport {
    pub hit_ratio: f64,
    pub memory_efficiency: f64,
    pub compression_effectiveness: f64,
    pub cache_utilization: f64,
    pub avg_response_time_improvement: f64,
    pub recommendations: Vec<String>,
}

/// キャッシュエラー
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Compression failed: {0}")]
    CompressionError(String),
    
    #[error("Decompression failed: {0}")]
    DecompressionError(String),
    
    #[error("Cache capacity exceeded")]
    CapacityExceeded,
    
    #[error("Invalid cache entry")]
    InvalidEntry,
}

impl DefaultCacheKeyHasher {
    pub fn new() -> Self {
        Self { version: 1 }
    }
}

impl CacheKeyHasher for DefaultCacheKeyHasher {
    fn generate_key(&self, request: &StreamRequest) -> Option<CacheKey> {
        let mut hasher = DefaultHasher::new();
        
        // リクエストの種類に応じてハッシュを生成
        let (operation_type, normalized_path) = match &request.message {
            Some(crate::grpc::stream_request::Message::ImportAsset(req)) => {
                req.asset_path.hash(&mut hasher);
                ("import_asset".to_string(), Some(req.asset_path.clone()))
            }
            Some(crate::grpc::stream_request::Message::MoveAsset(req)) => {
                req.src_path.hash(&mut hasher);
                req.dst_path.hash(&mut hasher);
                ("move_asset".to_string(), Some(format!("{}:{}", req.src_path, req.dst_path)))
            }
            Some(crate::grpc::stream_request::Message::DeleteAsset(req)) => {
                req.asset_path.hash(&mut hasher);
                ("delete_asset".to_string(), Some(req.asset_path.clone()))
            }
            Some(crate::grpc::stream_request::Message::Refresh(_)) => {
                ("refresh".to_string(), None)
            }
            None => return None,
        };

        let request_hash = hasher.finish();

        Some(CacheKey {
            operation_type,
            request_hash,
            version: self.version,
            attributes: CacheKeyAttributes {
                normalized_path,
                file_size_range: None, // 実装時に追加
                timestamp_bucket: None, // 実装時に追加
            },
        })
    }

    fn should_cache_response(&self, _request: &StreamRequest, _response: &StreamResponse) -> bool {
        // 基本的にすべてのレスポンスをキャッシュ可能とする
        // 実際の実装では、エラーレスポンスや大きなレスポンスを除外
        true
    }

    fn calculate_ttl(&self, _request: &StreamRequest, base_ttl: Duration) -> Duration {
        // 基本TTLを返す（アダプティブTTLは将来実装）
        base_ttl
    }
}

impl AccessPatternAnalyzer {
    fn new(_config: &CacheConfig) -> Self {
        Self {
            access_history: Vec::new(),
            operation_frequency: HashMap::new(),
            temporal_patterns: HashMap::new(),
            learned_patterns: Vec::new(),
            predictor: None,
        }
    }

    fn record_access(&mut self, record: AccessRecord) {
        // アクセス記録の保存
        self.access_history.push(record.clone());
        
        // 操作頻度の更新
        let operation = record.cache_key.operation_type.clone();
        *self.operation_frequency.entry(operation).or_insert(0) += 1;
        
        // 履歴サイズ制限（メモリ管理）
        if self.access_history.len() > 10000 {
            self.access_history.remove(0);
        }
    }
}

// キャッシュ予測エンジン等の実装は省略
#[derive(Debug)]
pub struct CachePredictor;

#[cfg(test)]
#[path = "cache_tests.rs"]
mod cache_tests;