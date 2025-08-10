//! リソースプールモジュール
//! 
//! 再利用可能なリソースのプールを提供します。

use std::sync::Arc;
use tokio::sync::Mutex;

/// リソースプール
pub struct ResourcePool<T> {
    resources: Arc<Mutex<Vec<T>>>,
    max_size: usize,
}

impl<T> ResourcePool<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            resources: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
        }
    }

    pub async fn get(&self) -> Option<T> {
        let mut resources = self.resources.lock().await;
        resources.pop()
    }

    pub async fn put(&self, resource: T) {
        let mut resources = self.resources.lock().await;
        if resources.len() < self.max_size {
            resources.push(resource);
        }
    }

    pub async fn size(&self) -> usize {
        let resources = self.resources.lock().await;
        resources.len()
    }
}