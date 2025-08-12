# Task 3.7 Fix 07-E: 既存WorkerPool並列処理拡張（Rayon統合版）

## 概要
既存のWorkerPoolを拡張し、Rayonクレートを活用したCPU集約的並列処理機能を追加します。既存の非同期タスク処理機能を維持しながら、CPU集約的なワークロード専用の並列処理メソッドを統合することで、アーキテクチャの一貫性を保ちつつパフォーマンス向上を実現します。

## 優先度
**🔴 最高優先度** - スループット向上の中核コンポーネント

## 実装時間見積もり
**30分** - 集中作業時間（既存WorkerPool拡張アプローチによる効率化）

## 依存関係
- Task 3.7 Fix 07-A (基盤インフラ整備) 完了必須
- Task 3.7 Fix 07-B (パフォーマンス監視システム) 完了推奨

## 受け入れ基準

### 並列処理要件
- [ ] 既存WorkerPoolへのRayon統合
- [ ] CPU集約的タスク専用メソッド追加
- [ ] Work-stealingアルゴリズムによる効率的負荷分散
- [ ] Parallel iteratorによるバッチ処理最適化

### 既存機能互換性要件
- [ ] 非同期タスク処理機能の完全保持
- [ ] 既存APIとの後方互換性維持
- [ ] グレースフルシャットダウン機能継続
- [ ] 統計収集とパフォーマンス監視統合

### 安定性要件
- [ ] Rayonの内蔵パニック復旧機能活用
- [ ] グレースフルシャットダウン
- [ ] Rustの型安全性によるデッドロック回避
- [ ] 軽量なエラーハンドリング

### パフォーマンス要件
- [ ] スループット 2000 req/s 達成
- [ ] タスク分散遅延 < 1ms
- [ ] ワーカー利用率 > 80%
- [ ] バッチ処理効率 > 90%

## 技術的詳細

### WorkerPool拡張実装（Rayon統合）

#### server/Cargo.toml への依存追加
```toml
[dependencies]
rayon = "1.8"
tokio-rayon = "2.1"  # tokioとの統合用
```

#### src/grpc/performance/worker_pool.rs への拡張
```rust
//! ワーカープールモジュール（Rayon並列処理拡張）
//! 
//! 非同期タスクのワーカープールに加えて、CPU集約的並列処理機能を提供します。

use futures::future::BoxFuture;
use rayon::prelude::*;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot};
use tracing::{debug, info, instrument};

/// ワーカープール（Rayon拡張版）
pub struct WorkerPool {
    // 既存フィールド（変更なし）
    sender: mpsc::Sender<BoxFuture<'static, ()>>,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

/// 並列処理エラー
#[derive(Debug, thiserror::Error)]
pub enum ParallelError {
    #[error("Task processing failed: {0}")]
    TaskFailed(String),
    
    #[error("Task was cancelled")]
    TaskCancelled,
    
    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}

impl WorkerPool {
    // 既存メソッド（変更なし）
    pub fn new(worker_count: usize, queue_capacity: usize) -> Self {
        // 既存実装を保持
    }

    pub async fn spawn<F>(&self, task: F) -> Result<(), mpsc::error::SendError<BoxFuture<'static, ()>>>
    where
        F: futures::Future<Output = ()> + Send + 'static,
    {
        // 既存実装を保持
    }

    // 新機能：CPU集約的並列処理
    /// CPU集約的な作業を並列実行
    #[instrument(skip(self, work))]
    pub async fn spawn_cpu_work<F, T>(&self, work: F) -> Result<T, ParallelError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        
        // Rayonで並列実行し、結果を非同期で返す
        rayon::spawn(move || {
            let result = work();
            let _ = tx.send(result);
        });
        
        rx.await.map_err(|_| ParallelError::TaskCancelled)
    }
    
    /// バッチデータを並列処理
    #[instrument(skip(self, items, process_fn))]
    pub async fn spawn_parallel_batch<T, F, R>(
        &self, 
        items: Vec<T>, 
        process_fn: F
    ) -> Result<Vec<R>, ParallelError>
    where
        T: Send + 'static,
        F: Fn(T) -> R + Send + Sync + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let process_fn = Arc::new(process_fn);
        
        tokio::task::spawn_blocking(move || {
            let results: Vec<R> = items
                .into_par_iter()
                .map(|item| process_fn(item))
                .collect();
            let _ = tx.send(results);
        });
        
        rx.await.map_err(|_| ParallelError::TaskCancelled)
    }
    
    /// チャンク分割並列処理
    pub async fn spawn_chunked_parallel<T, F, R>(
        &self,
        items: Vec<T>,
        chunk_size: usize,
        process_fn: F,
    ) -> Result<Vec<R>, ParallelError>
    where
        T: Send + 'static,
        F: Fn(T) -> R + Send + Sync + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let process_fn = Arc::new(process_fn);
        
        tokio::task::spawn_blocking(move || {
            let results: Vec<R> = items
                .par_chunks(chunk_size)
                .flat_map(|chunk| {
                    chunk.par_iter()
                        .map(|item| process_fn(item.clone()))
                })
                .collect();
            let _ = tx.send(results);
        });
        
        rx.await.map_err(|_| ParallelError::TaskCancelled)
    }

    // 既存メソッド（変更なし）
    pub async fn shutdown(mut self) -> Result<(), tokio::task::JoinError> {
        // 既存実装を保持
    }

    pub fn worker_count(&self) -> usize {
        // 既存実装を保持
    }
}
```

## 実装計画（WorkerPool拡張版）

### Step 1: Rayon依存関係追加 (5分)
1. `Cargo.toml` にrayon依存関係追加
2. 既存WorkerPoolモジュールへのRayonインポート追加

### Step 2: WorkerPool並列処理メソッド追加 (15分)
1. `spawn_cpu_work()` メソッド実装 - CPU集約的単一タスク
2. `spawn_parallel_batch()` メソッド実装 - バッチ並列処理
3. `spawn_chunked_parallel()` メソッド実装 - チャンク分割処理
4. `ParallelError` エラー型定義

### Step 3: テストと検証 (10分)
1. 既存メソッドの回帰テスト実行
2. 新並列処理メソッドのテスト実装
3. パフォーマンス比較ベンチマーク

## テスト要件（WorkerPool拡張版）

### 既存機能回帰テスト
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_existing_async_functionality() {
        let pool = WorkerPool::new(4, 100);
        
        // 既存の非同期タスク処理が正常動作することを確認
        let result = pool.spawn(async {
            // テストタスク
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(pool.worker_count(), 4);
    }

    #[tokio::test]
    async fn test_cpu_work_parallel_processing() {
        let pool = WorkerPool::new(4, 100);
        
        // CPU集約的タスクを並列実行
        let result = pool.spawn_cpu_work(|| {
            // 重い計算タスクのシミュレーション
            (0..1000000).sum::<u64>()
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 499999500000u64);
    }

    #[tokio::test]
    async fn test_parallel_batch_processing() {
        let pool = WorkerPool::new(4, 100);
        
        let items: Vec<u32> = (0..100).collect();
        let results = pool.spawn_parallel_batch(items, |x| x * 2).await;
        
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 100);
        assert_eq!(results[0], 0);
        assert_eq!(results[99], 198);
    }

    #[tokio::test]
    async fn test_chunked_parallel_processing() {
        let pool = WorkerPool::new(4, 100);
        
        let items: Vec<u32> = (0..50).collect();
        let results = pool.spawn_chunked_parallel(items, 10, |x| x * x).await;
        
        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 50);
        assert_eq!(results[0], 0);
        assert_eq!(results[49], 2401);
    }

    #[tokio::test]
    async fn test_performance_comparison() {
        let pool = WorkerPool::new(4, 100);
        
        let items: Vec<u32> = (0..10000).collect();
        
        // 逐次処理の時間測定
        let start = Instant::now();
        let _sequential: Vec<u64> = items.iter().map(|&x| expensive_computation(x)).collect();
        let sequential_time = start.elapsed();
        
        // 並列処理の時間測定
        let start = Instant::now();
        let _parallel = pool.spawn_parallel_batch(items, expensive_computation).await.unwrap();
        let parallel_time = start.elapsed();
        
        // 並列処理が高速であることを確認（理想的には）
        println!("Sequential: {:?}, Parallel: {:?}", sequential_time, parallel_time);
        
        // 最低限、並列処理が完了することを確認
        assert!(parallel_time < sequential_time * 2); // 寛大なチェック
    }

    fn expensive_computation(x: u32) -> u64 {
        // 簡単なCPU集約的計算
        (0..x % 1000).map(|i| i as u64).sum()
    }
}
```

## 成功基準（WorkerPool拡張版）

### 統合性基準
- 既存の非同期タスク処理機能に影響なし
- 全ての既存テストがパス
- 既存APIとの完全な後方互換性
- グレースフルシャットダウン機能の継続

### 並列処理機能基準
- CPU集約的タスクでのパフォーマンス向上 2-4倍
- バッチ処理効率 > 85%
- チャンク分割処理の正常動作
- メモリ使用量増加 < 20%

### 開発効率基準
- アーキテクチャの一貫性保持
- 新しいAPIの学習コスト最小化
- 単一クラスでの統一された概念
- コード重複の回避

## 次のステップ

WorkerPool並列処理拡張完了後：
1. Task 3.7 Fix 07-F: 最適化プロセッサーでの拡張WorkerPool統合
2. 既存システムとの統合テスト
3. 非同期 vs CPU並列処理のパフォーマンス比較ベンチマーク

## 関連ドキュメント
- Task 3.7 Fix 07-A (基盤インフラ整備)
- Task 3.7 Fix 07-B (パフォーマンス監視システム)
- [Rayon公式ドキュメント](https://docs.rs/rayon/)
- Rust並行プログラミングベストプラクティス