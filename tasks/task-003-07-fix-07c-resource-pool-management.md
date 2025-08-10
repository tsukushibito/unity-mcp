# Task 3.7 Fix 07-C: リソースプール管理

## 概要
効率的なリソースプール管理システムを実装します。オブジェクトプール、メモリ効率的なバッファ管理、リソースライフサイクル制御を通じて、メモリ使用量の最適化と GC プレッシャーの軽減を実現します。

## 優先度
**🟡 高優先度** - メモリ効率化とスケーラビリティに直接影響

## 実装時間見積もり
**45分** - 集中作業時間

## 依存関係
- Task 3.7 Fix 07-A (基盤インフラ整備) 完了必須

## 受け入れ基準

### オブジェクトプール要件
- [ ] 汎用的なオブジェクトプール実装
- [ ] サービスインスタンス、バリデーター、バッファのプール
- [ ] プールサイズ制限と動的拡張
- [ ] スレッドセーフなプール操作

### メモリ管理要件
- [ ] メモリ効率的なバッファ再利用
- [ ] バッファサイズの動的調整
- [ ] ゼロコピー操作の最大化
- [ ] メモリリーク完全防止

### ライフサイクル管理要件
- [ ] オブジェクトの自動返却機能
- [ ] 使用中リソースの追跡
- [ ] 期限切れリソースのクリーンアップ
- [ ] グレースフルシャットダウン

### 統計・監視要件
- [ ] プール利用統計の収集
- [ ] リソース取得/返却パフォーマンス監視
- [ ] メモリ使用量の継続的追跡
- [ ] プールヒット率の計算

## 技術的詳細

### ResourcePool 実装

#### src/grpc/performance/resource_pool.rs
```rust
//! 効率的なリソースプール管理
//! 
//! Unity MCP Server のパフォーマンス最適化のためのリソースプール実装。
//! サービスインスタンス、バリデーター、バッファのプールを管理し、
//! メモリ効率と GC プレッシャーの軽減を実現します。

use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use std::collections::VecDeque;
use tokio::time::sleep;
use tracing::{debug, info, warn, error};
use crate::grpc::service::UnityMcpServiceImpl;
use crate::grpc::validation::StreamValidationEngine;

/// 効率的なリソースプール管理システム
pub struct ResourcePool {
    // サービスインスタンスプール
    service_pool: Arc<ObjectPool<UnityMcpServiceImpl>>,
    
    // バリデーションエンジンプール  
    validator_pool: Arc<ObjectPool<StreamValidationEngine>>,
    
    // 汎用バッファプール
    buffer_pool: Arc<ObjectPool<Vec<u8>>>,
    
    // 文字列バッファプール
    string_buffer_pool: Arc<ObjectPool<String>>,
    
    // プール統計
    pool_stats: Arc<Mutex<PoolStatistics>>,
    
    // 設定
    config: ResourcePoolConfig,
    
    // ライフサイクル管理
    lifecycle_manager: Arc<LifecycleManager>,
}

/// オブジェクトプールの汎用実装
pub struct ObjectPool<T> {
    // 利用可能オブジェクト
    available_objects: Arc<Mutex<VecDeque<PooledItem<T>>>>,
    
    // オブジェクト作成ファクトリ
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    
    // リセット関数（オブジェクト返却時に呼び出し）
    reset_fn: Option<Arc<dyn Fn(&mut T) + Send + Sync>>,
    
    // プール設定
    max_size: usize,
    min_size: usize,
    
    // 統計情報
    stats: Arc<Mutex<ObjectPoolStats>>,
    
    // アクティブなオブジェクトの弱参照（デバッグ用）
    active_objects: Arc<Mutex<Vec<Weak<PooledObject<T>>>>>,
}

/// プールされたアイテムの包装
#[derive(Debug)]
struct PooledItem<T> {
    object: T,
    created_at: Instant,
    last_used: Instant,
    use_count: u64,
}

/// プールされたオブジェクトのスマートポインター
pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<ObjectPool<T>>,
    stats: Arc<Mutex<ObjectPoolStats>>,
    acquired_at: Instant,
}

/// オブジェクトプール統計
#[derive(Debug, Default, Clone)]
pub struct ObjectPoolStats {
    pub total_created: u64,
    pub total_acquired: u64,
    pub total_returned: u64,
    pub current_active: u64,
    pub current_available: usize,
    pub hit_ratio: f64,
    pub avg_acquisition_time_ns: u64,
    pub avg_hold_time_ms: f64,
}

/// リソースプール設定
#[derive(Debug, Clone)]
pub struct ResourcePoolConfig {
    // サービスプール設定
    pub service_pool_max: usize,
    pub service_pool_min: usize,
    
    // バリデータープール設定
    pub validator_pool_max: usize,
    pub validator_pool_min: usize,
    
    // バッファプール設定  
    pub buffer_pool_max: usize,
    pub buffer_pool_min: usize,
    pub buffer_initial_capacity: usize,
    pub buffer_max_capacity: usize,
    
    // 文字列バッファプール設定
    pub string_buffer_pool_max: usize,
    pub string_buffer_initial_capacity: usize,
    
    // ライフサイクル管理
    pub cleanup_interval: Duration,
    pub max_idle_time: Duration,
    pub enable_preallocation: bool,
}

impl Default for ResourcePoolConfig {
    fn default() -> Self {
        Self {
            service_pool_max: 20,
            service_pool_min: 2,
            validator_pool_max: 10,
            validator_pool_min: 1,
            buffer_pool_max: 100,
            buffer_pool_min: 10,
            buffer_initial_capacity: 8192,
            buffer_max_capacity: 1024 * 1024, // 1MB
            string_buffer_pool_max: 50,
            string_buffer_initial_capacity: 1024,
            cleanup_interval: Duration::from_secs(60),
            max_idle_time: Duration::from_secs(300), // 5分
            enable_preallocation: true,
        }
    }
}

/// プール全体の統計情報
#[derive(Debug, Default, Clone)]
pub struct PoolStatistics {
    pub service_pool_stats: ObjectPoolStats,
    pub validator_pool_stats: ObjectPoolStats,
    pub buffer_pool_stats: ObjectPoolStats,
    pub string_buffer_pool_stats: ObjectPoolStats,
    pub total_memory_allocated: usize,
    pub total_memory_in_use: usize,
    pub pool_efficiency: f64,
}

/// ライフサイクル管理
#[derive(Debug)]
pub struct LifecycleManager {
    cleanup_handles: Vec<tokio::task::JoinHandle<()>>,
    shutdown_signal: Arc<tokio::sync::Notify>,
}

impl ResourcePool {
    /// 新しいリソースプールを作成
    pub fn new() -> Self {
        Self::with_config(ResourcePoolConfig::default())
    }

    /// 設定付きでリソースプールを作成
    pub fn with_config(config: ResourcePoolConfig) -> Self {
        info!("Initializing resource pool with configuration: {:?}", config);

        let service_pool = Arc::new(ObjectPool::new(
            config.service_pool_max,
            config.service_pool_min,
            Arc::new(|| UnityMcpServiceImpl::new()),
            None, // サービスインスタンスはリセット不要
        ));

        let validator_pool = Arc::new(ObjectPool::new(
            config.validator_pool_max,
            config.validator_pool_min,
            Arc::new(|| StreamValidationEngine::new()),
            None, // バリデーターもリセット不要
        ));

        let buffer_initial_capacity = config.buffer_initial_capacity;
        let buffer_pool = Arc::new(ObjectPool::new(
            config.buffer_pool_max,
            config.buffer_pool_min,
            Arc::new(move || Vec::with_capacity(buffer_initial_capacity)),
            Some(Arc::new(|buffer: &mut Vec<u8>| {
                buffer.clear();
                // 大きすぎるバッファは縮小
                if buffer.capacity() > buffer_initial_capacity * 4 {
                    buffer.shrink_to(buffer_initial_capacity);
                }
            })),
        ));

        let string_initial_capacity = config.string_buffer_initial_capacity;
        let string_buffer_pool = Arc::new(ObjectPool::new(
            config.string_buffer_pool_max,
            0, // 最小サイズは0（オンデマンド作成）
            Arc::new(move || String::with_capacity(string_initial_capacity)),
            Some(Arc::new(|s: &mut String| {
                s.clear();
                if s.capacity() > string_initial_capacity * 4 {
                    s.shrink_to(string_initial_capacity);
                }
            })),
        ));

        let lifecycle_manager = Arc::new(LifecycleManager::new());
        let pool_stats = Arc::new(Mutex::new(PoolStatistics::default()));

        let resource_pool = Self {
            service_pool,
            validator_pool, 
            buffer_pool,
            string_buffer_pool,
            pool_stats,
            config: config.clone(),
            lifecycle_manager,
        };

        // 事前割り当て
        if config.enable_preallocation {
            resource_pool.preallocate_resources();
        }

        // クリーンアップタスクを開始
        resource_pool.start_cleanup_tasks();

        info!("Resource pool initialized successfully");
        resource_pool
    }

    /// サービスインスタンスを取得
    pub async fn get_service(&self) -> Result<PooledObject<UnityMcpServiceImpl>, ResourcePoolError> {
        let start = Instant::now();
        let service = self.service_pool.get().await?;
        let acquisition_time = start.elapsed();

        self.update_service_stats(acquisition_time);
        
        debug!("Service acquired in {:?}", acquisition_time);
        Ok(service)
    }

    /// バリデーションエンジンを取得
    pub async fn get_validator(&self) -> Result<PooledObject<StreamValidationEngine>, ResourcePoolError> {
        let start = Instant::now();
        let validator = self.validator_pool.get().await?;
        let acquisition_time = start.elapsed();

        self.update_validator_stats(acquisition_time);
        
        debug!("Validator acquired in {:?}", acquisition_time);
        Ok(validator)
    }

    /// バッファを取得
    pub async fn get_buffer(&self) -> Result<PooledObject<Vec<u8>>, ResourcePoolError> {
        let buffer = self.buffer_pool.get().await?;
        debug!("Buffer acquired");
        Ok(buffer)
    }

    /// 文字列バッファを取得
    pub async fn get_string_buffer(&self) -> Result<PooledObject<String>, ResourcePoolError> {
        let buffer = self.string_buffer_pool.get().await?;
        debug!("String buffer acquired");
        Ok(buffer)
    }

    /// プール統計を取得
    pub fn get_pool_statistics(&self) -> PoolStatistics {
        if let Ok(stats) = self.pool_stats.lock() {
            let mut combined_stats = stats.clone();
            
            // 各プールの最新統計を取得
            combined_stats.service_pool_stats = self.service_pool.get_stats();
            combined_stats.validator_pool_stats = self.validator_pool.get_stats();
            combined_stats.buffer_pool_stats = self.buffer_pool.get_stats();
            combined_stats.string_buffer_pool_stats = self.string_buffer_pool.get_stats();
            
            // 効率性を計算
            let total_requests = combined_stats.service_pool_stats.total_acquired +
                               combined_stats.validator_pool_stats.total_acquired +
                               combined_stats.buffer_pool_stats.total_acquired;
                               
            let total_hits = combined_stats.service_pool_stats.total_acquired - 
                           combined_stats.service_pool_stats.total_created +
                           combined_stats.validator_pool_stats.total_acquired - 
                           combined_stats.validator_pool_stats.total_created +
                           combined_stats.buffer_pool_stats.total_acquired - 
                           combined_stats.buffer_pool_stats.total_created;

            combined_stats.pool_efficiency = if total_requests > 0 {
                total_hits as f64 / total_requests as f64
            } else {
                0.0
            };

            combined_stats
        } else {
            PoolStatistics::default()
        }
    }

    /// リソースの事前割り当て
    fn preallocate_resources(&self) {
        debug!("Pre-allocating resources");

        // 各プールの最小サイズまで事前作成
        self.service_pool.preallocate(self.config.service_pool_min);
        self.validator_pool.preallocate(self.config.validator_pool_min);
        self.buffer_pool.preallocate(self.config.buffer_pool_min);

        debug!("Resource pre-allocation completed");
    }

    /// クリーンアップタスクを開始
    fn start_cleanup_tasks(&self) {
        let service_pool = Arc::clone(&self.service_pool);
        let validator_pool = Arc::clone(&self.validator_pool);
        let buffer_pool = Arc::clone(&self.buffer_pool);
        let string_buffer_pool = Arc::clone(&self.string_buffer_pool);
        let cleanup_interval = self.config.cleanup_interval;
        let max_idle_time = self.config.max_idle_time;

        tokio::spawn(async move {
            let mut cleanup_ticker = tokio::time::interval(cleanup_interval);
            
            loop {
                cleanup_ticker.tick().await;
                
                // 各プールの期限切れオブジェクトをクリーンアップ
                service_pool.cleanup_idle_objects(max_idle_time);
                validator_pool.cleanup_idle_objects(max_idle_time);
                buffer_pool.cleanup_idle_objects(max_idle_time);
                string_buffer_pool.cleanup_idle_objects(max_idle_time);
                
                debug!("Resource pool cleanup completed");
            }
        });
    }

    /// サービス統計の更新
    fn update_service_stats(&self, acquisition_time: Duration) {
        if let Ok(mut stats) = self.pool_stats.lock() {
            // 取得時間の移動平均を更新
            let new_time_ns = acquisition_time.as_nanos() as u64;
            stats.service_pool_stats.avg_acquisition_time_ns = 
                (stats.service_pool_stats.avg_acquisition_time_ns + new_time_ns) / 2;
        }
    }

    /// バリデーター統計の更新
    fn update_validator_stats(&self, acquisition_time: Duration) {
        if let Ok(mut stats) = self.pool_stats.lock() {
            let new_time_ns = acquisition_time.as_nanos() as u64;
            stats.validator_pool_stats.avg_acquisition_time_ns = 
                (stats.validator_pool_stats.avg_acquisition_time_ns + new_time_ns) / 2;
        }
    }
}

impl<T> ObjectPool<T> {
    /// 新しいオブジェクトプールを作成
    pub fn new<F>(
        max_size: usize,
        min_size: usize,
        factory: Arc<F>,
        reset_fn: Option<Arc<dyn Fn(&mut T) + Send + Sync>>,
    ) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            available_objects: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            factory: factory as Arc<dyn Fn() -> T + Send + Sync>,
            reset_fn,
            max_size,
            min_size,
            stats: Arc::new(Mutex::new(ObjectPoolStats::default())),
            active_objects: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// オブジェクトを取得
    pub async fn get(&self) -> Result<PooledObject<T>, ResourcePoolError> {
        let start = Instant::now();
        
        // まず利用可能なオブジェクトを確認
        if let Ok(mut available) = self.available_objects.lock() {
            if let Some(pooled_item) = available.pop_front() {
                // プールヒット
                if let Ok(mut stats) = self.stats.lock() {
                    stats.total_acquired += 1;
                    stats.current_active += 1;
                    stats.current_available = available.len();
                    stats.hit_ratio = (stats.total_acquired - stats.total_created) as f64 / stats.total_acquired as f64;
                }

                return Ok(PooledObject::new_from_pool(
                    pooled_item.object,
                    Arc::new(self.clone_for_return()),
                    Arc::clone(&self.stats),
                ));
            }
        }

        // プールミス：新しいオブジェクトを作成
        let new_object = (self.factory)();
        
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_created += 1;
            stats.total_acquired += 1;
            stats.current_active += 1;
            stats.hit_ratio = (stats.total_acquired - stats.total_created) as f64 / stats.total_acquired as f64;
        }

        Ok(PooledObject::new_from_pool(
            new_object,
            Arc::new(self.clone_for_return()),
            Arc::clone(&self.stats),
        ))
    }

    /// オブジェクトをプールに返却
    fn return_object(&self, object: T) {
        let now = Instant::now();
        
        if let Ok(mut available) = self.available_objects.lock() {
            if available.len() < self.max_size {
                // リセット処理
                let mut reset_object = object;
                if let Some(ref reset_fn) = self.reset_fn {
                    reset_fn(&mut reset_object);
                }

                let pooled_item = PooledItem {
                    object: reset_object,
                    created_at: now, // リセット時刻として記録
                    last_used: now,
                    use_count: 0,
                };

                available.push_back(pooled_item);
            }
            // プールが満杯の場合は単純に破棄
        }

        if let Ok(mut stats) = self.stats.lock() {
            stats.total_returned += 1;
            stats.current_active = stats.current_active.saturating_sub(1);
        }
    }

    /// 統計情報を取得
    pub fn get_stats(&self) -> ObjectPoolStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// 期限切れオブジェクトのクリーンアップ
    pub fn cleanup_idle_objects(&self, max_idle_time: Duration) {
        if let Ok(mut available) = self.available_objects.lock() {
            let now = Instant::now();
            let initial_count = available.len();
            
            // 期限切れオブジェクトを削除（最小サイズは保持）
            while available.len() > self.min_size {
                if let Some(item) = available.back() {
                    if now.duration_since(item.last_used) > max_idle_time {
                        available.pop_back();
                    } else {
                        break; // 新しいものから古いものの順なので、ここで終了
                    }
                } else {
                    break;
                }
            }

            let cleaned_count = initial_count - available.len();
            if cleaned_count > 0 {
                debug!("Cleaned up {} idle objects from pool", cleaned_count);
            }
        }
    }

    /// 事前割り当て
    pub fn preallocate(&self, count: usize) {
        if let Ok(mut available) = self.available_objects.lock() {
            let now = Instant::now();
            
            for _ in 0..count {
                if available.len() >= self.max_size {
                    break;
                }

                let object = (self.factory)();
                let pooled_item = PooledItem {
                    object,
                    created_at: now,
                    last_used: now,
                    use_count: 0,
                };

                available.push_back(pooled_item);
            }
        }
    }

    // ヘルパーメソッド（実装のためのクローン）
    fn clone_for_return(&self) -> ObjectPoolCloneProxy<T> {
        ObjectPoolCloneProxy {
            available_objects: Arc::clone(&self.available_objects),
            reset_fn: self.reset_fn.clone(),
            max_size: self.max_size,
            stats: Arc::clone(&self.stats),
        }
    }
}

/// オブジェクト返却用のプロキシ
struct ObjectPoolCloneProxy<T> {
    available_objects: Arc<Mutex<VecDeque<PooledItem<T>>>>,
    reset_fn: Option<Arc<dyn Fn(&mut T) + Send + Sync>>,
    max_size: usize,
    stats: Arc<Mutex<ObjectPoolStats>>,
}

impl<T> ObjectPoolCloneProxy<T> {
    fn return_object(&self, object: T) {
        let now = Instant::now();
        
        if let Ok(mut available) = self.available_objects.lock() {
            if available.len() < self.max_size {
                let mut reset_object = object;
                if let Some(ref reset_fn) = self.reset_fn {
                    reset_fn(&mut reset_object);
                }

                let pooled_item = PooledItem {
                    object: reset_object,
                    created_at: now,
                    last_used: now,
                    use_count: 0,
                };

                available.push_back(pooled_item);
            }
        }

        if let Ok(mut stats) = self.stats.lock() {
            stats.total_returned += 1;
            stats.current_active = stats.current_active.saturating_sub(1);
        }
    }
}

impl<T> PooledObject<T> {
    fn new_from_pool(
        object: T,
        pool_proxy: Arc<ObjectPoolCloneProxy<T>>,
        stats: Arc<Mutex<ObjectPoolStats>>,
    ) -> Self {
        Self {
            object: Some(object),
            pool: pool_proxy,
            stats,
            acquired_at: Instant::now(),
        }
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.object.as_ref().expect("PooledObject already consumed")
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.object.as_mut().expect("PooledObject already consumed")
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            let hold_time = self.acquired_at.elapsed();
            
            // 保持時間統計を更新
            if let Ok(mut stats) = self.stats.lock() {
                stats.avg_hold_time_ms = (stats.avg_hold_time_ms + hold_time.as_secs_f64() * 1000.0) / 2.0;
            }
            
            self.pool.return_object(object);
        }
    }
}

/// リソースプールエラー
#[derive(Debug, thiserror::Error)]
pub enum ResourcePoolError {
    #[error("Pool is at capacity")]
    PoolAtCapacity,
    
    #[error("Object creation failed: {0}")]
    ObjectCreationFailed(String),
    
    #[error("Pool is shutting down")]
    PoolShuttingDown,
    
    #[error("Internal pool error: {0}")]
    InternalError(String),
}

impl LifecycleManager {
    fn new() -> Self {
        Self {
            cleanup_handles: Vec::new(),
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
        }
    }
}

// 実装続行...
```

## 実装計画

### Step 1: 基本オブジェクトプール (20分)
1. ObjectPool の汎用実装
2. PooledObject スマートポインター
3. 基本的な取得・返却機能

### Step 2: 特化プール実装 (15分)  
1. サービス、バリデーター、バッファプール
2. 各プール特有のリセット処理
3. プール統計収集

### Step 3: ライフサイクル管理 (10分)
1. 期限切れオブジェクトのクリーンアップ
2. 事前割り当て機能
3. グレースフルシャットダウン

## テスト要件

### プール動作テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_object_pool_basic_operations() {
        let pool = ObjectPool::new(
            5, 2, 
            Arc::new(|| String::from("test")),
            Some(Arc::new(|s: &mut String| s.clear()))
        );

        let obj1 = pool.get().await.unwrap();
        assert_eq!(**obj1, "test");
        
        drop(obj1); // 返却
        
        let obj2 = pool.get().await.unwrap();
        assert_eq!(**obj2, ""); // リセット済み
    }

    #[tokio::test]
    async fn test_resource_pool_integration() {
        let resource_pool = ResourcePool::new();
        
        let service = resource_pool.get_service().await.unwrap();
        let validator = resource_pool.get_validator().await.unwrap();
        let buffer = resource_pool.get_buffer().await.unwrap();
        
        // すべて正常に取得できることを確認
        assert!(service.is_ok());
        assert!(validator.is_ok());
        assert!(buffer.is_ok());
    }
}
```

## 成功基準

### 機能基準
- オブジェクトプールが正常動作
- リソースの自動返却機能
- メモリリーク完全防止
- スレッドセーフ性確保

### パフォーマンス基準
- オブジェクト取得オーバーヘッド < 1μs
- プールヒット率 > 80%
- メモリ使用量削減効果確認

## 次のステップ

リソースプール管理完了後：
1. Task 3.7 Fix 07-D: インテリジェントキャッシュシステム実装
2. 他の最適化コンポーネントでのプール活用
3. 統合パフォーマンステスト実行

## 関連ドキュメント  
- Task 3.7 Fix 07-A (基盤インフラ整備)
- Rust オブジェクトプール設計パターン
- メモリ効率化ベストプラクティス