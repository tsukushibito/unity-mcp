//! ワーカープールモジュール
//! 
//! 非同期タスクのワーカープールを提供します。

use futures::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// ワーカープール
pub struct WorkerPool {
    sender: mpsc::Sender<BoxFuture<'static, ()>>,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl WorkerPool {
    pub fn new(worker_count: usize, queue_capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel::<BoxFuture<'static, ()>>(queue_capacity);
        let rx = Arc::new(Mutex::new(rx));
        
        let mut handles = Vec::with_capacity(worker_count);
        
        for worker_id in 0..worker_count {
            let rx_clone = Arc::clone(&rx);
            let handle = tokio::spawn(async move {
                tracing::debug!(worker_id = worker_id, "Worker started");
                
                loop {
                    let mut rx_guard = rx_clone.lock().await;
                    match rx_guard.recv().await {
                        Some(task) => {
                            drop(rx_guard); // ロックを早く解放
                            task.await;
                        }
                        None => {
                            tracing::debug!(worker_id = worker_id, "Worker shutting down");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }

        Self {
            sender: tx,
            handles,
        }
    }

    pub async fn spawn<F>(&self, task: F) -> Result<(), mpsc::error::SendError<BoxFuture<'static, ()>>>
    where
        F: futures::Future<Output = ()> + Send + 'static,
    {
        let boxed_task: BoxFuture<'static, ()> = Box::pin(task);
        self.sender.send(boxed_task).await
    }

    /// グレースフルシャットダウン - 全てのワーカーが完了を待つ
    pub async fn shutdown(mut self) -> Result<(), tokio::task::JoinError> {
        // チャネルを閉じることでワーカーにシャットダウンを通知
        drop(self.sender);
        
        // 全てのワーカーハンドルの完了を待つ
        for handle in self.handles.drain(..) {
            handle.await?;
        }
        
        Ok(())
    }

    /// アクティブなワーカー数を取得
    pub fn worker_count(&self) -> usize {
        self.handles.len()
    }
}