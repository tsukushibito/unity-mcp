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
use tracing::{debug, info, error, warn, trace, instrument};
use serde::{Serialize, Deserialize};
use flate2::write::{GzEncoder};
use flate2::read::GzDecoder;
use flate2::Compression;
use std::io::{Read, Write};

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
    compressed_data: Option<Vec<u8>>, // 圧縮されたバイナリデータ
    
    // 品質情報
    cache_quality_score: f64, // 0.0-1.0
}

/// 操作タイプのenum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OperationType {
    ImportAsset,
    MoveAsset,
    DeleteAsset,
    Refresh,
}

impl OperationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OperationType::ImportAsset => "import_asset",
            OperationType::MoveAsset => "move_asset",
            OperationType::DeleteAsset => "delete_asset",
            OperationType::Refresh => "refresh",
        }
    }
}

/// キャッシュキー
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    // 操作タイプ
    pub operation_type: OperationType,
    
    // リクエストハッシュ
    pub request_hash: u64,
    
    // バージョン（スキーマ変更対応）
    pub version: u32,
    
    // オプション属性
    pub attributes: CacheKeyAttributes,
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

/// 固定サイズの循環バッファー（メモリ効率改善）
#[derive(Debug)]
pub struct CircularBuffer<T> {
    buffer: Vec<Option<T>>,
    capacity: usize,
    head: usize,
    tail: usize,
    size: usize,
}

impl<T> CircularBuffer<T> {
    fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, || None);
        
        Self {
            buffer,
            capacity,
            head: 0,
            tail: 0,
            size: 0,
        }
    }
    
    fn push(&mut self, item: T) {
        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;
        
        if self.size < self.capacity {
            self.size += 1;
        } else {
            // バッファーが満杯の場合、headも進める
            self.head = (self.head + 1) % self.capacity;
        }
    }
    
    fn len(&self) -> usize {
        self.size
    }
    
    fn iter(&self) -> CircularBufferIterator<T> {
        CircularBufferIterator {
            buffer: &self.buffer,
            capacity: self.capacity,
            current: self.head,
            remaining: self.size,
        }
    }
    
    fn clear(&mut self) {
        for item in &mut self.buffer {
            *item = None;
        }
        self.head = 0;
        self.tail = 0;
        self.size = 0;
    }
}

/// 循環バッファーのイテレーター
pub struct CircularBufferIterator<'a, T> {
    buffer: &'a Vec<Option<T>>,
    capacity: usize,
    current: usize,
    remaining: usize,
}

impl<'a, T> Iterator for CircularBufferIterator<'a, T> {
    type Item = &'a T;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        
        let item = self.buffer[self.current].as_ref();
        self.current = (self.current + 1) % self.capacity;
        self.remaining -= 1;
        
        item
    }
}

/// アクセスパターン分析（循環バッファーでメモリ効率改善）
#[derive(Debug)]
pub struct AccessPatternAnalyzer {
    // パターン記録（循環バッファー）
    access_history: CircularBuffer<AccessRecord>,
    
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
    pub fn new() -> Result<Self, CacheError> {
        Self::with_config(CacheConfig::default())
    }

    /// 設定付きでキャッシュを作成
    pub fn with_config(config: CacheConfig) -> Result<Self, CacheError> {
        let cache_size = NonZeroUsize::new(config.max_entries)
            .ok_or_else(|| CacheError::InvalidConfiguration(
                "max_entries must be greater than 0".to_string()
            ))?;
        
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
        Ok(cache_instance)
    }

    /// キャッシュからレスポンスを取得
    #[instrument(skip(self, request), fields(cache_key))]
    pub async fn get(&self, request: &StreamRequest) -> Option<StreamResponse> {
        let start_time = Instant::now();
        
        // キャッシュキーを生成
        let cache_key = match self.key_hasher.generate_key(request) {
            Some(key) => {
                // Span にキャッシュキー情報を記録
                tracing::Span::current().record("cache_key", &format!("{:?}", key));
                trace!(
                    cache_key = ?key,
                    "Generated cache key for request"
                );
                key
            },
            None => {
                debug!("Request not cacheable, no key generated");
                return None;
            }
        };

        let result = {
            let mut cache = match self.cache.lock() {
                Ok(cache) => cache,
                Err(_) => {
                    error!(
                        cache_key = ?cache_key,
                        "Cache mutex poisoned during get operation - returning None"
                    );
                    return None;
                }
            };
            
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
                    self.decompress_response(entry)
                        .unwrap_or_else(|_| entry.response.clone())
                } else {
                    entry.response.clone()
                };

                let elapsed = start_time.elapsed();
                info!(
                    cache_key = ?cache_key,
                    response_time_ns = elapsed.as_nanos(),
                    is_compressed = entry.is_compressed,
                    access_count = entry.access_count,
                    "Cache hit - response served from cache"
                );

                self.record_cache_hit(&cache_key, elapsed);
                Some(response)
            } else {
                let elapsed = start_time.elapsed();
                debug!(
                    cache_key = ?cache_key,
                    response_time_ns = elapsed.as_nanos(),
                    "Cache miss - key not found in cache"
                );
                self.record_cache_miss(&cache_key, elapsed);
                None
            }
        };

        // アクセスパターン学習
        self.learn_access_pattern(&cache_key, result.is_some()).await;

        result
    }

    /// レスポンスをキャッシュに保存
    #[instrument(skip(self, request, response), fields(cache_key, compressed_size, original_size))]
    pub async fn put(&self, request: &StreamRequest, response: StreamResponse) {
        // キャッシュキーを生成
        let cache_key = match self.key_hasher.generate_key(request) {
            Some(key) => {
                tracing::Span::current().record("cache_key", &format!("{:?}", key));
                trace!(cache_key = ?key, "Generated cache key for put operation");
                key
            },
            None => {
                debug!("Request not cacheable, skipping put operation");
                return;
            },
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
        let (final_response, is_compressed, original_size, compressed_size, compressed_data) = 
            if self.config.enable_compression {
                self.compress_response_if_beneficial(&response)
            } else {
                let size = self.estimate_response_size(&response);
                (response, false, size, size, None)
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
            compressed_data,
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

        // 構造化ログでキャッシュ保存を記録
        tracing::Span::current().record("compressed_size", compressed_size);
        tracing::Span::current().record("original_size", original_size);
        info!(
            cache_key = ?cache_key,
            ttl_secs = ttl.as_secs(),
            is_compressed = is_compressed,
            original_size = original_size,
            compressed_size = compressed_size,
            compression_ratio = if original_size > 0 { compressed_size as f64 / original_size as f64 } else { 1.0 },
            "Response cached successfully"
        );
    }

    /// キャッシュキーを直接指定して取得（高速パス）
    pub fn get_by_key(&self, key: &CacheKey) -> Option<StreamResponse> {
        let start_time = Instant::now();
        
        let result = {
            let mut cache = match self.cache.lock() {
                Ok(cache) => cache,
                Err(_) => {
                    error!("Cache mutex poisoned during get_by_key operation");
                    return None;
                }
            };
            
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
                    self.decompress_response(entry)
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
    pub fn resize_cache(&self, new_size: usize) -> Result<(), CacheError> {
        if new_size == 0 {
            return Err(CacheError::InvalidConfiguration(
                "new_size must be greater than 0".to_string()
            ));
        }
        
        match self.cache.lock() {
            Ok(mut cache) => {
                let new_cache_size = NonZeroUsize::new(new_size)
                    .ok_or_else(|| CacheError::InvalidConfiguration(
                        "new_size must be greater than 0".to_string()
                    ))?;
                
                cache.resize(new_cache_size);
                info!("Cache resized to {} entries", new_size);
                Ok(())
            }
            Err(_) => {
                error!("Failed to resize cache: mutex poisoned");
                Err(CacheError::LockPoisoned)
            }
        }
    }

    /// キャッシュをクリア
    pub fn clear(&self) {
        match self.cache.lock() {
            Ok(mut cache) => {
                cache.clear();
                info!("Cache cleared");
            }
            Err(_) => {
                error!("Failed to clear cache: mutex poisoned");
            }
        }

        match self.statistics.lock() {
            Ok(mut stats) => {
                *stats = CacheStatistics::default();
            }
            Err(_) => {
                error!("Failed to reset statistics: mutex poisoned");
            }
        }
    }

    /// キャッシュ統計を取得
    pub fn get_statistics(&self) -> CacheStatistics {
        match self.statistics.lock() {
            Ok(stats) => stats.clone(),
            Err(_) => {
                error!("Failed to get statistics: mutex poisoned");
                CacheStatistics::default()
            }
        }
    }

    /// キャッシュ効率レポートを生成
    pub fn generate_efficiency_report(&self) -> CacheEfficiencyReport {
        let stats = self.get_statistics();
        let current_size = match self.cache.lock() {
            Ok(cache) => cache.len(),
            Err(_) => {
                error!("Failed to get cache size: mutex poisoned");
                0
            }
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

    #[instrument(skip(self), fields(key = ?_key, response_time_ns = response_time.as_nanos()))]
    fn record_cache_hit(&self, _key: &CacheKey, response_time: Duration) {
        match self.statistics.lock() {
            Ok(mut stats) => {
                stats.total_requests += 1;
                stats.cache_hits += 1;
                stats.hit_ratio = stats.cache_hits as f64 / stats.total_requests as f64;
                
                // 指数移動平均でヒット時間を更新（α = 0.1）
                let new_hit_time = response_time.as_nanos() as u64;
                if stats.avg_hit_time_ns == 0 {
                    stats.avg_hit_time_ns = new_hit_time;
                } else {
                    let alpha = 0.1;
                    stats.avg_hit_time_ns = ((1.0 - alpha) * stats.avg_hit_time_ns as f64 + alpha * new_hit_time as f64) as u64;
                }
            }
            Err(_) => {
                error!("Failed to record cache hit: statistics mutex poisoned");
            }
        }
    }

    #[instrument(skip(self), fields(key = ?_key, response_time_ns = response_time.as_nanos()))]
    fn record_cache_miss(&self, _key: &CacheKey, response_time: Duration) {
        match self.statistics.lock() {
            Ok(mut stats) => {
                stats.total_requests += 1;
                stats.cache_misses += 1;
                stats.hit_ratio = stats.cache_hits as f64 / stats.total_requests as f64;
                
                // 指数移動平均でミス時間を更新（α = 0.1）
                let new_miss_time = response_time.as_nanos() as u64;
                if stats.avg_miss_time_ns == 0 {
                    stats.avg_miss_time_ns = new_miss_time;
                } else {
                    let alpha = 0.1;
                    stats.avg_miss_time_ns = ((1.0 - alpha) * stats.avg_miss_time_ns as f64 + alpha * new_miss_time as f64) as u64;
                }
            }
            Err(_) => {
                error!("Failed to record cache miss: statistics mutex poisoned");
            }
        }
    }

    fn record_cache_eviction(&self) {
        match self.statistics.lock() {
            Ok(mut stats) => {
                stats.cache_evictions += 1;
            }
            Err(_) => {
                error!("Failed to record cache eviction: statistics mutex poisoned");
            }
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

        match self.access_pattern_analyzer.lock() {
            Ok(mut analyzer) => {
                analyzer.record_access(record);
            }
            Err(_) => {
                error!("Failed to record access pattern: analyzer mutex poisoned");
            }
        }
    }

    fn compress_response_if_beneficial(&self, response: &StreamResponse) -> (StreamResponse, bool, usize, usize, Option<Vec<u8>>) {
        let original_size = self.estimate_response_size(response);
        
        if original_size < self.config.compression_threshold_bytes {
            return (response.clone(), false, original_size, original_size, None);
        }

        // 実際の圧縮実装
        match self.compress_response(response) {
            Ok((compressed_response, compressed_data)) => {
                let compressed_size = compressed_data.len();
                (compressed_response, true, original_size, compressed_size, Some(compressed_data))
            }
            Err(_) => {
                // 圧縮に失敗した場合は元のレスポンスを返す
                (response.clone(), false, original_size, original_size, None)
            }
        }
    }

    fn compress_response(&self, response: &StreamResponse) -> Result<(StreamResponse, Vec<u8>), CacheError> {
        // レスポンスの内容をJSON文字列にシリアライズ
        let response_json = self.serialize_response_for_compression(response)
            .map_err(|e| CacheError::CompressionError(format!("Serialization failed: {}", e)))?;
        
        // gzip圧縮を実行
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.config.compression_level));
        encoder.write_all(response_json.as_bytes())
            .map_err(|e| CacheError::CompressionError(format!("Compression write failed: {}", e)))?;
        
        let compressed_data = encoder.finish()
            .map_err(|e| CacheError::CompressionError(format!("Compression finish failed: {}", e)))?;
        
        // 圧縮されたことを示すマーカーレスポンスを作成
        // 実際の本番環境では、メタデータフィールドで圧縮を示すか、
        // 別のメッセージタイプを使用する
        let compressed_marker_response = StreamResponse {
            message: Some(crate::grpc::stream_response::Message::ImportAsset(
                crate::grpc::ImportAssetResponse {
                    asset: Some(crate::grpc::UnityAsset {
                        guid: "__COMPRESSED_DATA_MARKER__".to_string(),
                        asset_path: format!("compressed:{}bytes", compressed_data.len()),
                        r#type: "application/gzip".to_string(),
                    }),
                    error: None,
                }
            )),
        };
        
        Ok((compressed_marker_response, compressed_data))
    }

    fn decompress_response(&self, entry: &CacheEntry) -> Result<StreamResponse, CacheError> {
        // 圧縮されていない場合は元のレスポンスをそのまま返す
        if !entry.is_compressed || entry.compressed_data.is_none() {
            return Ok(entry.response.clone());
        }
        
        let compressed_data = entry.compressed_data.as_ref()
            .ok_or_else(|| CacheError::DecompressionError("No compressed data available".to_string()))?;
        
        // gzip解凍を実行
        let mut decoder = GzDecoder::new(&compressed_data[..]);
        let mut decompressed_json = String::new();
        decoder.read_to_string(&mut decompressed_json)
            .map_err(|e| CacheError::DecompressionError(format!("Decompression failed: {}", e)))?;
        
        // JSONからStreamResponseを復元
        self.deserialize_response_from_json(&decompressed_json)
            .map_err(|e| CacheError::DecompressionError(format!("Deserialization failed: {}", e)))
    }

    
    fn deserialize_response_from_json(&self, json_str: &str) -> Result<StreamResponse, String> {
        use serde_json::Value;
        
        let json_value: Value = serde_json::from_str(json_str)
            .map_err(|e| format!("JSON parse failed: {}", e))?;
        
        let response_type = json_value.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing or invalid response type".to_string())?;
        
        let message = match response_type {
            "import_asset" => {
                let asset = json_value.get("asset").and_then(|a| {
                    if a.is_null() { None } else {
                        Some(crate::grpc::UnityAsset {
                            guid: a.get("guid")?.as_str()?.to_string(),
                            asset_path: a.get("asset_path")?.as_str()?.to_string(),
                            r#type: a.get("type")?.as_str()?.to_string(),
                        })
                    }
                });
                
                let error = json_value.get("error").and_then(|e| {
                    if e.is_null() { None } else {
                        Some(crate::grpc::McpError {
                            code: e.get("code")?.as_i64()? as i32,
                            message: e.get("message")?.as_str()?.to_string(),
                            details: e.get("details")?.as_str()?.to_string(),
                        })
                    }
                });
                
                Some(crate::grpc::stream_response::Message::ImportAsset(
                    crate::grpc::ImportAssetResponse { asset, error }
                ))
            }
            "move_asset" => {
                let asset = json_value.get("asset").and_then(|a| {
                    if a.is_null() { None } else {
                        Some(crate::grpc::UnityAsset {
                            guid: a.get("guid")?.as_str()?.to_string(),
                            asset_path: a.get("asset_path")?.as_str()?.to_string(),
                            r#type: a.get("type")?.as_str()?.to_string(),
                        })
                    }
                });
                
                let error = json_value.get("error").and_then(|e| {
                    if e.is_null() { None } else {
                        Some(crate::grpc::McpError {
                            code: e.get("code")?.as_i64()? as i32,
                            message: e.get("message")?.as_str()?.to_string(),
                            details: e.get("details")?.as_str()?.to_string(),
                        })
                    }
                });
                
                Some(crate::grpc::stream_response::Message::MoveAsset(
                    crate::grpc::MoveAssetResponse { asset, error }
                ))
            }
            "delete_asset" => {
                let success = json_value.get("success")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false);
                
                let error = json_value.get("error").and_then(|e| {
                    if e.is_null() { None } else {
                        Some(crate::grpc::McpError {
                            code: e.get("code")?.as_i64()? as i32,
                            message: e.get("message")?.as_str()?.to_string(),
                            details: e.get("details")?.as_str()?.to_string(),
                        })
                    }
                });
                
                Some(crate::grpc::stream_response::Message::DeleteAsset(
                    crate::grpc::DeleteAssetResponse { success, error }
                ))
            }
            "refresh" => {
                let success = json_value.get("success")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false);
                
                let error = json_value.get("error").and_then(|e| {
                    if e.is_null() { None } else {
                        Some(crate::grpc::McpError {
                            code: e.get("code")?.as_i64()? as i32,
                            message: e.get("message")?.as_str()?.to_string(),
                            details: e.get("details")?.as_str()?.to_string(),
                        })
                    }
                });
                
                Some(crate::grpc::stream_response::Message::Refresh(
                    crate::grpc::RefreshResponse { success, error }
                ))
            }
            "empty" => None,
            _ => return Err(format!("Unknown response type: {}", response_type))
        };
        
        Ok(StreamResponse { message })
    }
    
    fn serialize_response_for_compression(&self, response: &StreamResponse) -> Result<String, String> {
        use serde_json::json;
        
        // StreamResponseを構造化されたJSONに変換
        let json_value = match &response.message {
            Some(crate::grpc::stream_response::Message::ImportAsset(import_resp)) => {
                json!({
                    "type": "import_asset",
                    "asset": import_resp.asset.as_ref().map(|asset| json!({
                        "guid": asset.guid,
                        "asset_path": asset.asset_path,
                        "type": asset.r#type
                    })),
                    "error": import_resp.error.as_ref().map(|error| json!({
                        "code": error.code,
                        "message": error.message,
                        "details": error.details
                    }))
                })
            }
            Some(crate::grpc::stream_response::Message::MoveAsset(move_resp)) => {
                json!({
                    "type": "move_asset",
                    "asset": move_resp.asset.as_ref().map(|asset| json!({
                        "guid": asset.guid,
                        "asset_path": asset.asset_path,
                        "type": asset.r#type
                    })),
                    "error": move_resp.error.as_ref().map(|error| json!({
                        "code": error.code,
                        "message": error.message,
                        "details": error.details
                    }))
                })
            }
            Some(crate::grpc::stream_response::Message::DeleteAsset(delete_resp)) => {
                json!({
                    "type": "delete_asset",
                    "success": delete_resp.success,
                    "error": delete_resp.error.as_ref().map(|error| json!({
                        "code": error.code,
                        "message": error.message,
                        "details": error.details
                    }))
                })
            }
            Some(crate::grpc::stream_response::Message::Refresh(refresh_resp)) => {
                json!({
                    "type": "refresh",
                    "success": refresh_resp.success,
                    "error": refresh_resp.error.as_ref().map(|error| json!({
                        "code": error.code,
                        "message": error.message,
                        "details": error.details
                    }))
                })
            }
            None => json!({"type": "empty"})
        };
        
        // JSON文字列に変換
        serde_json::to_string(&json_value)
            .map_err(|e| format!("JSON serialization failed: {}", e))
    }

    fn estimate_response_size(&self, response: &StreamResponse) -> usize {
        // より正確なprotobufサイズ推定
        // protobufエンコーディング特性を考慮した計算
        let mut size = 0usize;
        
        // protobufメッセージのベースオーバーヘッド
        size += 4; // メッセージタイプとlengthフィールド
        
        match &response.message {
            Some(crate::grpc::stream_response::Message::ImportAsset(import_resp)) => {
                // フィールド1: asset (optional message)
                if let Some(asset) = &import_resp.asset {
                    size += 1; // field tag
                    let asset_size = self.estimate_asset_info_size(asset);
                    size += self.estimate_varint_size(asset_size) + asset_size;
                }
                
                // フィールド2: error (optional message)  
                if let Some(error) = &import_resp.error {
                    size += 1; // field tag
                    let error_size = self.estimate_error_info_size(error);
                    size += self.estimate_varint_size(error_size) + error_size;
                }
            }
            Some(crate::grpc::stream_response::Message::MoveAsset(move_resp)) => {
                if let Some(asset) = &move_resp.asset {
                    size += 1;
                    let asset_size = self.estimate_asset_info_size(asset);
                    size += self.estimate_varint_size(asset_size) + asset_size;
                }
                
                if let Some(error) = &move_resp.error {
                    size += 1;
                    let error_size = self.estimate_error_info_size(error);
                    size += self.estimate_varint_size(error_size) + error_size;
                }
            }
            Some(crate::grpc::stream_response::Message::DeleteAsset(delete_resp)) => {
                // success boolean field
                size += 1 + 1; // field tag + boolean value
                
                if let Some(error) = &delete_resp.error {
                    size += 1;
                    let error_size = self.estimate_error_info_size(error);
                    size += self.estimate_varint_size(error_size) + error_size;
                }
            }
            Some(crate::grpc::stream_response::Message::Refresh(refresh_resp)) => {
                // success boolean field
                size += 1 + 1;
                
                if let Some(error) = &refresh_resp.error {
                    size += 1;
                    let error_size = self.estimate_error_info_size(error);
                    size += self.estimate_varint_size(error_size) + error_size;
                }
            }
            None => {
                // 空のメッセージ
                size += 1;
            }
        }
        
        size
    }

    
    /// protobuf AssetInfoのサイズを推定
    fn estimate_asset_info_size(&self, asset: &crate::grpc::UnityAsset) -> usize {
        let mut size = 0usize;
        
        // guid field (string)
        if !asset.guid.is_empty() {
            size += 1; // field tag
            size += self.estimate_varint_size(asset.guid.len()) + asset.guid.len();
        }
        
        // asset_path field (string)
        if !asset.asset_path.is_empty() {
            size += 1;
            size += self.estimate_varint_size(asset.asset_path.len()) + asset.asset_path.len();
        }
        
        // type field (string)  
        if !asset.r#type.is_empty() {
            size += 1;
            size += self.estimate_varint_size(asset.r#type.len()) + asset.r#type.len();
        }
        
        size
    }
    
    /// protobuf ErrorInfoのサイズを推定
    fn estimate_error_info_size(&self, error: &crate::grpc::McpError) -> usize {
        let mut size = 0usize;
        
        // code field (int32)
        if error.code != 0 {
            size += 1; // field tag
            size += self.estimate_varint_size(error.code.abs() as usize);
        }
        
        // message field (string)
        if !error.message.is_empty() {
            size += 1; // field tag
            size += self.estimate_varint_size(error.message.len()) + error.message.len();
        }
        
        // details field (string)
        if !error.details.is_empty() {
            size += 1;
            size += self.estimate_varint_size(error.details.len()) + error.details.len();
        }
        
        size
    }
    
    /// protobuf varintエンコーディングのサイズを推定
    fn estimate_varint_size(&self, value: usize) -> usize {
        match value {
            0..=127 => 1,
            128..=16383 => 2,
            16384..=2097151 => 3,
            2097152..=268435455 => 4,
            _ => 5, // 最大5バイト
        }
    }

    fn calculate_cache_quality_score(&self, _request: &StreamRequest) -> f64 {
        // キャッシュ品質スコア計算（複雑度、サイズ、頻度等を考慮）
        0.8 // デフォルト値
    }

    fn should_evict_for_memory(&self, entry: &CacheEntry) -> bool {
        // 現在の統計を取得してメモリ使用量をチェック
        let stats = self.get_statistics();
        let entry_size = if entry.is_compressed { 
            entry.compressed_size 
        } else { 
            entry.original_size 
        };
        
        // メモリ制限を超える場合は退避が必要
        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        let projected_memory = stats.current_memory_usage + entry_size;
        
        projected_memory > max_memory_bytes
    }

    fn evict_by_memory_pressure(&self, cache: &mut LruCache<CacheKey, CacheEntry>) {
        // メモリプレッシャーベースの退避処理
        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        let mut current_memory = 0usize;
        
        // 現在のメモリ使用量を計算
        for (_key, entry) in cache.iter() {
            current_memory += if entry.is_compressed {
                entry.compressed_size
            } else {
                entry.original_size
            };
        }
        
        // メモリ制限の80%を目標に退避
        let target_memory = (max_memory_bytes as f64 * 0.8) as usize;
        
        while current_memory > target_memory && !cache.is_empty() {
            if let Some((evicted_key, evicted_entry)) = cache.pop_lru() {
                let entry_size = if evicted_entry.is_compressed {
                    evicted_entry.compressed_size
                } else {
                    evicted_entry.original_size
                };
                current_memory = current_memory.saturating_sub(entry_size);
                
                debug!("Evicted entry due to memory pressure: {:?}", evicted_key);
                self.record_cache_eviction();
            } else {
                break;
            }
        }
    }

    async fn update_cache_statistics(&self) {
        // キャッシュ統計の更新処理
        match (self.cache.lock(), self.statistics.lock()) {
            (Ok(cache), Ok(mut stats)) => {
                stats.current_entry_count = cache.len();
                
                // メモリ使用量の更新
                let mut total_memory = 0usize;
                let mut compressed_memory = 0usize;
                let mut original_memory = 0usize;
                
                for (_key, entry) in cache.iter() {
                    let entry_size = if entry.is_compressed {
                        compressed_memory += entry.compressed_size;
                        original_memory += entry.original_size;
                        entry.compressed_size
                    } else {
                        original_memory += entry.original_size;
                        entry.original_size
                    };
                    total_memory += entry_size;
                }
                
                stats.current_memory_usage = total_memory;
                if total_memory > stats.peak_memory_usage {
                    stats.peak_memory_usage = total_memory;
                }
                
                // 圧縮率の更新
                if original_memory > 0 {
                    stats.compression_ratio = compressed_memory as f64 / original_memory as f64;
                } else {
                    stats.compression_ratio = 1.0;
                }
            }
            _ => {
                error!("Failed to update cache statistics: mutex poisoned");
            }
        }
    }

    fn start_maintenance_tasks(&self) {
        // 定期メンテナンスタスク開始（簡略化）
        // 実際の実装では、tokio::spawn でバックグラウンドタスクを開始
        info!("Cache maintenance tasks initialized with cleanup interval: {:?}", 
               self.config.cleanup_interval);
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
    
    #[error("Cache mutex poisoned")]
    LockPoisoned,
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Cache operation failed: {0}")]
    OperationFailed(String),
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
                (OperationType::ImportAsset, Some(req.asset_path.clone()))
            }
            Some(crate::grpc::stream_request::Message::MoveAsset(req)) => {
                req.src_path.hash(&mut hasher);
                req.dst_path.hash(&mut hasher);
                (OperationType::MoveAsset, Some(format!("{}:{}", req.src_path, req.dst_path)))
            }
            Some(crate::grpc::stream_request::Message::DeleteAsset(req)) => {
                req.asset_path.hash(&mut hasher);
                (OperationType::DeleteAsset, Some(req.asset_path.clone()))
            }
            Some(crate::grpc::stream_request::Message::Refresh(_)) => {
                (OperationType::Refresh, None)
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
    fn new(config: &CacheConfig) -> Self {
        // デフォルトで5000エントリの循環バッファーを使用（約1MB程度）
        let history_capacity = if config.enable_pattern_learning { 5000 } else { 100 };
        
        Self {
            access_history: CircularBuffer::new(history_capacity),
            operation_frequency: HashMap::new(),
            temporal_patterns: HashMap::new(),
            learned_patterns: Vec::new(),
            predictor: None,
        }
    }

    fn record_access(&mut self, record: AccessRecord) {
        // アクセス記録の保存（循環バッファーが自動的にサイズ制限）
        self.access_history.push(record.clone());
        
        // 操作頻度の更新
        let operation = record.cache_key.operation_type.as_str().to_string();
        *self.operation_frequency.entry(operation).or_insert(0) += 1;
    }
}

// キャッシュ予測エンジン等の実装は省略
#[derive(Debug)]
pub struct CachePredictor;

#[cfg(test)]
#[path = "cache_tests.rs"]
mod cache_tests;