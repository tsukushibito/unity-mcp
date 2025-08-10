#[cfg(test)]
mod tests {
    use crate::grpc::performance::cache::{StreamCache, CacheConfig, DefaultCacheKeyHasher, CacheKeyHasher, OperationType};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio;
    use crate::grpc::{StreamRequest, StreamResponse, ImportAssetRequest, ImportAssetResponse, UnityAsset};

    fn create_test_request() -> StreamRequest {
        StreamRequest {
            message: Some(crate::grpc::stream_request::Message::ImportAsset(
                ImportAssetRequest {
                    asset_path: "Assets/TestAsset.png".to_string(),
                }
            )),
        }
    }

    fn create_test_response() -> StreamResponse {
        StreamResponse {
            message: Some(crate::grpc::stream_response::Message::ImportAsset(
                ImportAssetResponse {
                    asset: Some(UnityAsset {
                        guid: "test-guid-12345".to_string(),
                        asset_path: "Assets/TestAsset.png".to_string(),
                        r#type: "Texture2D".to_string(),
                    }),
                    error: None,
                }
            )),
        }
    }

    fn create_different_test_request() -> StreamRequest {
        StreamRequest {
            message: Some(crate::grpc::stream_request::Message::ImportAsset(
                ImportAssetRequest {
                    asset_path: "Assets/DifferentAsset.png".to_string(),
                }
            )),
        }
    }

    #[tokio::test]
    async fn test_basic_cache_operations() {
        let cache = StreamCache::new().expect("Failed to create cache");
        
        // まずはミスを確認
        let request = create_test_request();
        assert!(cache.get(&request).await.is_none());
        
        // 保存して取得
        let response = create_test_response();
        cache.put(&request, response.clone()).await;
        
        let cached_response = cache.get(&request).await;
        assert!(cached_response.is_some());
        
        // レスポンスの内容を比較（簡略化）
        let cached = cached_response.unwrap();
        match (&response.message, &cached.message) {
            (
                Some(crate::grpc::stream_response::Message::ImportAsset(orig)),
                Some(crate::grpc::stream_response::Message::ImportAsset(cached))
            ) => {
                assert_eq!(orig.asset.as_ref().unwrap().asset_path, 
                          cached.asset.as_ref().unwrap().asset_path);
            }
            _ => panic!("Response message types don't match"),
        }
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let config = CacheConfig {
            default_ttl: Duration::from_millis(10),
            ..Default::default()
        };
        let cache = StreamCache::with_config(config).expect("Failed to create cache");
        
        let request = create_test_request();
        let response = create_test_response();
        
        cache.put(&request, response).await;
        assert!(cache.get(&request).await.is_some());
        
        // TTL期限切れを待機
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert!(cache.get(&request).await.is_none());
    }

    #[test]
    fn test_cache_statistics() {
        let cache = StreamCache::new().expect("Failed to create cache");
        let stats = cache.get_statistics();
        
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.hit_ratio, 0.0);
    }

    #[tokio::test]
    async fn test_cache_hit_statistics() {
        let cache = StreamCache::new().expect("Failed to create cache");
        let request = create_test_request();
        let response = create_test_response();
        
        // 最初のリクエスト（ミス）
        assert!(cache.get(&request).await.is_none());
        
        // レスポンスを保存
        cache.put(&request, response).await;
        
        // 2回目のリクエスト（ヒット）
        assert!(cache.get(&request).await.is_some());
        
        let stats = cache.get_statistics();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.hit_ratio, 0.5);
    }

    #[tokio::test]
    async fn test_multiple_different_requests() {
        let cache = StreamCache::new().expect("Failed to create cache");
        let request1 = create_test_request();
        let request2 = create_different_test_request();
        let response1 = create_test_response();
        let response2 = create_test_response();
        
        // 2つの異なるリクエストをキャッシュ
        cache.put(&request1, response1).await;
        cache.put(&request2, response2).await;
        
        // 両方のレスポンスが取得できることを確認
        assert!(cache.get(&request1).await.is_some());
        assert!(cache.get(&request2).await.is_some());
        
        let stats = cache.get_statistics();
        assert_eq!(stats.cache_hits, 2);
        assert_eq!(stats.hit_ratio, 1.0);
    }

    #[test]
    fn test_cache_key_generation() {
        let hasher = DefaultCacheKeyHasher::new();
        let request = create_test_request();
        
        let key = hasher.generate_key(&request);
        assert!(key.is_some());
        
        let cache_key = key.unwrap();
        assert_eq!(cache_key.operation_type.as_str(), "import_asset");
        assert_eq!(cache_key.version, 1);
        assert!(cache_key.attributes.normalized_path.is_some());
    }

    #[test]
    fn test_cache_key_consistency() {
        let hasher = DefaultCacheKeyHasher::new();
        let request = create_test_request();
        
        let key1 = hasher.generate_key(&request);
        let key2 = hasher.generate_key(&request);
        
        assert_eq!(key1, key2);
    }

    #[tokio::test]
    async fn test_cache_resize() {
        let cache = StreamCache::new().expect("Failed to create cache");
        
        // 初期サイズを確認
        let initial_stats = cache.get_statistics();
        assert_eq!(initial_stats.current_entry_count, 0);
        
        // キャッシュサイズを変更
        cache.resize_cache(500).expect("Failed to resize cache");
        
        // リサイズ後もキャッシュが正常に動作することを確認
        let request = create_test_request();
        let response = create_test_response();
        
        cache.put(&request, response).await;
        assert!(cache.get(&request).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = StreamCache::new().expect("Failed to create cache");
        let request = create_test_request();
        let response = create_test_response();
        
        // データをキャッシュ
        cache.put(&request, response).await;
        assert!(cache.get(&request).await.is_some());
        
        // キャッシュをクリア
        cache.clear();
        
        // データが削除されたことを確認
        assert!(cache.get(&request).await.is_none());
        
        let stats = cache.get_statistics();
        // clear()後の新しいリクエストなので、total_requestsは1になる
        assert_eq!(stats.total_requests, 1);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 1);
    }

    #[test]
    fn test_cache_efficiency_report() {
        let cache = StreamCache::new().expect("Failed to create cache");
        let report = cache.generate_efficiency_report();
        
        assert_eq!(report.hit_ratio, 0.0);
        assert_eq!(report.cache_utilization, 0.0);
        assert_eq!(report.memory_efficiency, 1.0);
        assert!(!report.recommendations.is_empty());
    }

    #[tokio::test]
    async fn test_compression_configuration() {
        let config = CacheConfig {
            enable_compression: true,
            compression_threshold_bytes: 100, // 小さな閾値
            ..Default::default()
        };
        
        let cache = StreamCache::with_config(config).expect("Failed to create cache");
        let request = create_test_request();
        let response = create_test_response();
        
        cache.put(&request, response).await;
        let cached_response = cache.get(&request).await;
        
        // 圧縮が有効でもレスポンスが正常に取得できることを確認
        assert!(cached_response.is_some());
    }

    #[tokio::test]
    async fn test_access_pattern_learning() {
        let config = CacheConfig {
            enable_pattern_learning: true,
            ..Default::default()
        };
        
        let cache = StreamCache::with_config(config).expect("Failed to create cache");
        let request = create_test_request();
        let response = create_test_response();
        
        // パターン学習が有効な状態でキャッシュ操作を実行
        cache.put(&request, response).await;
        assert!(cache.get(&request).await.is_some());
        
        // パターン学習機能の動作確認（詳細な検証は実装に依存）
        let stats = cache.get_statistics();
        assert_eq!(stats.cache_hits, 1);
    }

    #[test]
    fn test_different_operation_types() {
        let hasher = DefaultCacheKeyHasher::new();
        
        // ImportAssetリクエスト
        let import_request = StreamRequest {
            message: Some(crate::grpc::stream_request::Message::ImportAsset(
                ImportAssetRequest {
                    asset_path: "Assets/Test.png".to_string(),
                }
            )),
        };
        
        // MoveAssetリクエスト
        let move_request = StreamRequest {
            message: Some(crate::grpc::stream_request::Message::MoveAsset(
                crate::grpc::MoveAssetRequest {
                    src_path: "Assets/Test.png".to_string(),
                    dst_path: "Assets/Moved.png".to_string(),
                }
            )),
        };
        
        let import_key = hasher.generate_key(&import_request).unwrap();
        let move_key = hasher.generate_key(&move_request).unwrap();
        
        assert_eq!(import_key.operation_type.as_str(), "import_asset");
        assert_eq!(move_key.operation_type.as_str(), "move_asset");
        assert_ne!(import_key, move_key);
    }

    #[tokio::test]
    async fn test_get_by_key_direct_access() {
        let cache = StreamCache::new().expect("Failed to create cache");
        let request = create_test_request();
        let response = create_test_response();
        
        // レスポンスをキャッシュ
        cache.put(&request, response.clone()).await;
        
        // キーを生成して直接アクセス
        let hasher = DefaultCacheKeyHasher::new();
        let key = hasher.generate_key(&request).unwrap();
        
        let cached_response = cache.get_by_key(&key);
        assert!(cached_response.is_some());
        
        // 統計が更新されることを確認
        let stats = cache.get_statistics();
        assert!(stats.cache_hits > 0);
    }

    #[tokio::test]
    async fn test_concurrent_cache_access() {
        let cache = Arc::new(StreamCache::new().expect("Failed to create cache"));
        let request = create_test_request();
        let response = create_test_response();
        
        // 初期データをキャッシュ
        cache.put(&request, response.clone()).await;
        
        // 並行読み取りタスクを生成
        let read_tasks: Vec<_> = (0..10)
            .map(|i| {
                let cache_clone = Arc::clone(&cache);
                let request_clone = request.clone();
                tokio::spawn(async move {
                    for _ in 0..100 {
                        let result = cache_clone.get(&request_clone).await;
                        assert!(result.is_some(), "Task {} failed to get cached response", i);
                    }
                })
            })
            .collect();
        
        // 並行書き込みタスクを生成
        let write_tasks: Vec<_> = (0..5)
            .map(|i| {
                let cache_clone = Arc::clone(&cache);
                let response_clone = response.clone();
                tokio::spawn(async move {
                    for j in 0..50 {
                        let varied_request = StreamRequest {
                            message: Some(crate::grpc::stream_request::Message::ImportAsset(
                                ImportAssetRequest {
                                    asset_path: format!("Assets/Concurrent/Task{}_Item{}.png", i, j),
                                }
                            )),
                        };
                        cache_clone.put(&varied_request, response_clone.clone()).await;
                    }
                })
            })
            .collect();
        
        // すべてのタスクの完了を待機
        for task in read_tasks {
            task.await.expect("Read task failed");
        }
        
        for task in write_tasks {
            task.await.expect("Write task failed");
        }
        
        // キャッシュ統計の確認
        let stats = cache.get_statistics();
        assert!(stats.total_requests >= 1000); // 読み取りリクエスト数
        assert!(stats.cache_hits >= 1000);     // 初期データへのヒット
        assert_eq!(stats.hit_ratio, 1.0);      // 全てヒットするはず
    }

    #[tokio::test]
    async fn test_cache_resize_under_load() {
        let cache = Arc::new(StreamCache::new().expect("Failed to create cache"));
        
        // バックグラウンドでキャッシュ操作を実行
        let cache_clone = Arc::clone(&cache);
        let background_task = tokio::spawn(async move {
            for i in 0..200 {
                let request = StreamRequest {
                    message: Some(crate::grpc::stream_request::Message::ImportAsset(
                        ImportAssetRequest {
                            asset_path: format!("Assets/Background/Item{}.png", i),
                        }
                    )),
                };
                let response = create_test_response();
                
                cache_clone.put(&request, response).await;
                
                // 一部の読み取り操作も実行
                if i % 10 == 0 {
                    cache_clone.get(&request).await;
                }
                
                // 小さなスリープで他のタスクに実行時間を与える
                tokio::task::yield_now().await;
            }
        });
        
        // 並行してキャッシュサイズを変更
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let resize_result = cache.resize_cache(2000);
        assert!(resize_result.is_ok(), "Cache resize should succeed under load");
        
        // バックグラウンドタスクの完了を待機
        background_task.await.expect("Background task should complete");
        
        // リサイズ後もキャッシュが正常に動作することを確認
        let test_request = create_test_request();
        let test_response = create_test_response();
        cache.put(&test_request, test_response).await;
        
        let retrieved = cache.get(&test_request).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear_under_concurrent_access() {
        let cache = Arc::new(StreamCache::new().expect("Failed to create cache"));
        
        // 初期データを追加
        for i in 0..50 {
            let request = StreamRequest {
                message: Some(crate::grpc::stream_request::Message::ImportAsset(
                    ImportAssetRequest {
                        asset_path: format!("Assets/Initial/Item{}.png", i),
                    }
                )),
            };
            cache.put(&request, create_test_response()).await;
        }
        
        let stats_before = cache.get_statistics();
        assert!(stats_before.current_entry_count > 0);
        
        // 並行アクセスタスクを開始
        let cache_clone = Arc::clone(&cache);
        let access_task = tokio::spawn(async move {
            for i in 0..100 {
                let request = StreamRequest {
                    message: Some(crate::grpc::stream_request::Message::ImportAsset(
                        ImportAssetRequest {
                            asset_path: format!("Assets/Concurrent/Item{}.png", i),
                        }
                    )),
                };
                
                // 読み取りまたは書き込み（ランダム）
                if i % 2 == 0 {
                    cache_clone.get(&request).await;
                } else {
                    cache_clone.put(&request, create_test_response()).await;
                }
                
                tokio::task::yield_now().await;
            }
        });
        
        // 少し待ってからクリア
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        cache.clear();
        
        // アクセスタスクの完了を待機
        access_task.await.expect("Concurrent access task should complete");
        
        // クリア後の動作を確認
        let test_request = create_test_request();
        let test_response = create_test_response();
        cache.put(&test_request, test_response).await;
        
        let retrieved = cache.get(&test_request).await;
        assert!(retrieved.is_some(), "Cache should work normally after clear");
    }

    #[tokio::test]
    async fn test_cache_performance_requirements() {
        use std::time::Instant;
        
        let cache = StreamCache::new().expect("Failed to create cache");
        let request = create_test_request();
        let response = create_test_response();
        
        // 初期データをキャッシュ
        cache.put(&request, response).await;
        
        // キャッシュヒット時の<1ms要件テスト
        let mut hit_times = Vec::new();
        for _ in 0..1000 {
            let start = Instant::now();
            let result = cache.get(&request).await;
            let elapsed = start.elapsed();
            
            assert!(result.is_some(), "Cache hit should return response");
            hit_times.push(elapsed);
        }
        
        // 統計計算
        let avg_hit_time = hit_times.iter().sum::<std::time::Duration>() / hit_times.len() as u32;
        let p95_hit_time = {
            let mut sorted_times = hit_times.clone();
            sorted_times.sort();
            sorted_times[(sorted_times.len() as f64 * 0.95) as usize]
        };
        let p99_hit_time = {
            let mut sorted_times = hit_times;
            sorted_times.sort();
            sorted_times[(sorted_times.len() as f64 * 0.99) as usize]
        };
        
        println!("Cache Hit Performance:");
        println!("  Average: {:?}", avg_hit_time);
        println!("  P95: {:?}", p95_hit_time);
        println!("  P99: {:?}", p99_hit_time);
        
        // 1ms未満要件の検証
        assert!(
            avg_hit_time.as_micros() < 1000,
            "Average cache hit time should be < 1ms, got {:?}",
            avg_hit_time
        );
        assert!(
            p95_hit_time.as_micros() < 1000,
            "P95 cache hit time should be < 1ms, got {:?}",
            p95_hit_time
        );
    }

    #[tokio::test]
    async fn test_cache_miss_performance() {
        use std::time::Instant;
        
        let cache = StreamCache::new().expect("Failed to create cache");
        
        let mut miss_times = Vec::new();
        for i in 0..100 {
            let request = StreamRequest {
                message: Some(crate::grpc::stream_request::Message::ImportAsset(
                    ImportAssetRequest {
                        asset_path: format!("Assets/NonExistent/Item{}.png", i),
                    }
                )),
            };
            
            let start = Instant::now();
            let result = cache.get(&request).await;
            let elapsed = start.elapsed();
            
            assert!(result.is_none(), "Cache miss should return None");
            miss_times.push(elapsed);
        }
        
        let avg_miss_time = miss_times.iter().sum::<std::time::Duration>() / miss_times.len() as u32;
        println!("Cache Miss Average Time: {:?}", avg_miss_time);
        
        // ミスはヒットより遅いが、それでも高速であることを確認
        assert!(
            avg_miss_time.as_micros() < 100,  // 100μs未満
            "Average cache miss time should be very fast, got {:?}",
            avg_miss_time
        );
    }

    #[tokio::test]
    async fn test_cache_put_performance() {
        use std::time::Instant;
        
        let cache = StreamCache::new().expect("Failed to create cache");
        let response = create_test_response();
        
        let mut put_times = Vec::new();
        for i in 0..500 {
            let request = StreamRequest {
                message: Some(crate::grpc::stream_request::Message::ImportAsset(
                    ImportAssetRequest {
                        asset_path: format!("Assets/Performance/Item{}.png", i),
                    }
                )),
            };
            
            let start = Instant::now();
            cache.put(&request, response.clone()).await;
            let elapsed = start.elapsed();
            
            put_times.push(elapsed);
        }
        
        let avg_put_time = put_times.iter().sum::<std::time::Duration>() / put_times.len() as u32;
        let p95_put_time = {
            let mut sorted_times = put_times;
            sorted_times.sort();
            sorted_times[(sorted_times.len() as f64 * 0.95) as usize]
        };
        
        println!("Cache Put Performance:");
        println!("  Average: {:?}", avg_put_time);
        println!("  P95: {:?}", p95_put_time);
        
        // put操作も高速であることを確認（5ms未満）
        assert!(
            avg_put_time.as_millis() < 5,
            "Average cache put time should be < 5ms, got {:?}",
            avg_put_time
        );
        assert!(
            p95_put_time.as_millis() < 10,
            "P95 cache put time should be < 10ms, got {:?}",
            p95_put_time
        );
    }

    #[tokio::test]
    async fn test_memory_usage_efficiency() {
        let config = CacheConfig {
            max_memory_mb: 10,  // 10MBに制限
            enable_compression: true,
            compression_threshold_bytes: 100,
            ..Default::default()
        };
        
        let cache = StreamCache::with_config(config).expect("Failed to create cache");
        let response = create_test_response();
        
        // メモリ制限内で多くのエントリを保存
        for i in 0..2000 {
            let request = StreamRequest {
                message: Some(crate::grpc::stream_request::Message::ImportAsset(
                    ImportAssetRequest {
                        asset_path: format!("Assets/MemoryTest/LargeAsset{}.png", i),
                    }
                )),
            };
            cache.put(&request, response.clone()).await;
        }
        
        let stats = cache.get_statistics();
        let memory_mb = stats.current_memory_usage as f64 / (1024.0 * 1024.0);
        
        println!("Memory Usage Stats:");
        println!("  Current Memory: {:.2} MB", memory_mb);
        println!("  Entry Count: {}", stats.current_entry_count);
        println!("  Compression Ratio: {:.3}", stats.compression_ratio);
        
        // メモリ制限を守っていることを確認
        assert!(
            memory_mb <= 12.0, // 多少のオーバーヘッドを許容
            "Memory usage should be within limits, got {:.2} MB",
            memory_mb
        );
        
        // 圧縮が有効に働いていることを確認
        assert!(
            stats.compression_ratio < 1.0,
            "Compression should reduce memory usage, ratio: {}",
            stats.compression_ratio
        );
    }
}