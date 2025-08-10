#[cfg(test)]
mod tests {
    use crate::grpc::performance::cache::{StreamCache, CacheConfig, DefaultCacheKeyHasher, CacheKeyHasher};
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
        let cache = StreamCache::new();
        
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
        let cache = StreamCache::with_config(config);
        
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
        let cache = StreamCache::new();
        let stats = cache.get_statistics();
        
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.hit_ratio, 0.0);
    }

    #[tokio::test]
    async fn test_cache_hit_statistics() {
        let cache = StreamCache::new();
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
        let cache = StreamCache::new();
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
        assert_eq!(cache_key.operation_type, "import_asset");
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
        let cache = StreamCache::new();
        
        // 初期サイズを確認
        let initial_stats = cache.get_statistics();
        assert_eq!(initial_stats.current_entry_count, 0);
        
        // キャッシュサイズを変更
        cache.resize_cache(500);
        
        // リサイズ後もキャッシュが正常に動作することを確認
        let request = create_test_request();
        let response = create_test_response();
        
        cache.put(&request, response).await;
        assert!(cache.get(&request).await.is_some());
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = StreamCache::new();
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
        let cache = StreamCache::new();
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
        
        let cache = StreamCache::with_config(config);
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
        
        let cache = StreamCache::with_config(config);
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
        
        assert_eq!(import_key.operation_type, "import_asset");
        assert_eq!(move_key.operation_type, "move_asset");
        assert_ne!(import_key, move_key);
    }

    #[tokio::test]
    async fn test_get_by_key_direct_access() {
        let cache = StreamCache::new();
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
}