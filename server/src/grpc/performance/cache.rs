//! キャッシュモジュール
//! 
//! ストリームデータのキャッシングを提供します。

use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::grpc::performance::OptimizationError;

/// ストリームキャッシュ
pub struct StreamCache<K, V> 
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    cache: Arc<RwLock<LruCache<K, V>>>,
}

impl<K, V> StreamCache<K, V> 
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Result<Self, OptimizationError> {
        let capacity = NonZeroUsize::new(capacity)
            .ok_or_else(|| OptimizationError::CacheError("Capacity must be greater than 0".to_string()))?;
        
        Ok(Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
        })
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;
        cache.get(key).cloned()
    }

    pub async fn put(&self, key: K, value: V) {
        let mut cache = self.cache.write().await;
        cache.put(key, value);
    }

    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    pub async fn cap(&self) -> usize {
        let cache = self.cache.read().await;
        cache.cap().get()
    }
}