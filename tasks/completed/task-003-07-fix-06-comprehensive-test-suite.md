# Task 3.7 Fix 06: 包括的テストスイート実装

## 概要
Task 3.7のストリーミングサービス実装で特定されたテストカバレッジ不足を解決します。ストリーミング機能に対する専用テスト、エラーハンドリングテスト、パフォーマンステスト、メモリリークテストの実装により、品質保証を強化します。

## 優先度
**🟡 重要優先度** - 品質保証と回帰防止に重要な影響

## 実装時間見積もり
**4-5時間** - 集中作業時間

## 受け入れ基準

### テストカバレッジ要件
- [ ] ストリーミング機能のコードカバレッジ90%以上
- [ ] 全エラーケースのテスト実装
- [ ] 境界値テストの包括的実装
- [ ] 異常系テストの充実

### テスト品質要件
- [ ] テストの独立性確保
- [ ] 決定論的テスト実行
- [ ] テスト実行時間の最適化
- [ ] テストデータの適切な管理

### 継続的品質保証要件
- [ ] 回帰テストの自動化
- [ ] パフォーマンス基準の監視
- [ ] メモリリーク検出の自動化
- [ ] 負荷テストの実装

## 技術的詳細

### テスト構造設計

#### 1. テストモジュール構成
```rust
#[cfg(test)]
mod streaming_tests {
    use super::*;
    
    mod unit_tests {
        mod stream_connection_handler_tests;
        mod stream_message_processor_tests;
        mod individual_handler_tests;
        mod validation_engine_tests;
        mod error_handling_tests;
    }
    
    mod integration_tests {
        mod end_to_end_streaming_tests;
        mod concurrent_stream_tests;
        mod error_recovery_tests;
        mod resource_management_tests;
    }
    
    mod performance_tests {
        mod throughput_tests;
        mod memory_usage_tests;
        mod latency_tests;
        mod stress_tests;
    }
    
    mod security_tests {
        mod input_validation_tests;
        mod rate_limiting_tests;
        mod attack_simulation_tests;
    }
}
```

#### 2. テスト基盤とヘルパー
```rust
/// Test utilities for streaming functionality
pub struct StreamTestHarness {
    service: Arc<UnityMcpServiceImpl>,
    runtime: tokio::runtime::Runtime,
    test_data_generator: TestDataGenerator,
    performance_monitor: TestPerformanceMonitor,
}

impl StreamTestHarness {
    pub fn new() -> Self {
        Self {
            service: Arc::new(UnityMcpServiceImpl::new()),
            runtime: tokio::runtime::Runtime::new().unwrap(),
            test_data_generator: TestDataGenerator::new(),
            performance_monitor: TestPerformanceMonitor::new(),
        }
    }

    pub async fn create_test_stream(
        &self,
        requests: Vec<StreamRequest>,
    ) -> (
        tokio::sync::mpsc::Sender<Result<StreamRequest, Status>>,
        tokio::sync::mpsc::Receiver<Result<StreamResponse, Status>>,
    ) {
        let (req_tx, req_rx) = tokio::sync::mpsc::channel(1000);
        let (resp_tx, resp_rx) = tokio::sync::mpsc::channel(1000);

        // テスト用ストリーミング接続のシミュレーション
        tokio::spawn(async move {
            for request in requests {
                let _ = req_tx.send(Ok(request)).await;
            }
        });

        (req_tx, resp_rx)
    }

    pub async fn simulate_client_disconnect(&self) {
        // クライアント切断のシミュレーション
    }

    pub async fn simulate_network_error(&self) -> Status {
        Status::unavailable("Simulated network error")
    }

    pub fn generate_stress_test_requests(&self, count: usize) -> Vec<StreamRequest> {
        self.test_data_generator.generate_mixed_requests(count)
    }
}

/// Generate test data for various scenarios
pub struct TestDataGenerator {
    asset_paths: Vec<String>,
    invalid_paths: Vec<String>,
    edge_case_paths: Vec<String>,
}

impl TestDataGenerator {
    pub fn new() -> Self {
        Self {
            asset_paths: vec![
                "Assets/Textures/player.png".to_string(),
                "Assets/Scripts/GameController.cs".to_string(),
                "Assets/Models/character.fbx".to_string(),
                "Assets/Prefabs/Weapon.prefab".to_string(),
            ],
            invalid_paths: vec![
                "".to_string(),                              // Empty path
                "../secret".to_string(),                     // Path traversal
                "Assets/../secret".to_string(),              // Relative traversal
                "Assets/file<script>".to_string(),           // Invalid characters
                "Assets/".repeat(100) + "toolong.png",       // Too long path
            ],
            edge_case_paths: vec![
                "Assets/Textures/file with spaces.png".to_string(),
                "Assets/Scripts/日本語ファイル.cs".to_string(),
                "Assets/Models/file.with.dots.fbx".to_string(),
                format!("Assets/Textures/{}.png", "a".repeat(200)), // Near max length
            ],
        }
    }

    pub fn generate_valid_import_request(&self) -> StreamRequest {
        let path = self.asset_paths.choose(&mut rand::thread_rng()).unwrap().clone();
        StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest { asset_path: path },
            )),
        }
    }

    pub fn generate_invalid_import_request(&self) -> StreamRequest {
        let path = self.invalid_paths.choose(&mut rand::thread_rng()).unwrap().clone();
        StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest { asset_path: path },
            )),
        }
    }

    pub fn generate_valid_move_request(&self) -> StreamRequest {
        let src = self.asset_paths.choose(&mut rand::thread_rng()).unwrap().clone();
        let dst = format!("Assets/Moved/{}", 
                         src.split('/').last().unwrap_or("file.txt"));
        
        StreamRequest {
            message: Some(stream_request::Message::MoveAsset(
                MoveAssetRequest {
                    src_path: src,
                    dst_path: dst,
                },
            )),
        }
    }

    pub fn generate_edge_case_request(&self) -> StreamRequest {
        let path = self.edge_case_paths.choose(&mut rand::thread_rng()).unwrap().clone();
        StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest { asset_path: path },
            )),
        }
    }

    pub fn generate_mixed_requests(&self, count: usize) -> Vec<StreamRequest> {
        let mut requests = Vec::new();
        let mut rng = rand::thread_rng();
        
        for _ in 0..count {
            let request_type: u32 = rng.gen_range(0..4);
            let request = match request_type {
                0 => self.generate_valid_import_request(),
                1 => self.generate_valid_move_request(),
                2 => {
                    let path = self.asset_paths.choose(&mut rng).unwrap().clone();
                    StreamRequest {
                        message: Some(stream_request::Message::DeleteAsset(
                            DeleteAssetRequest { asset_path: path },
                        )),
                    }
                }
                3 => StreamRequest {
                    message: Some(stream_request::Message::Refresh(RefreshRequest {})),
                },
                _ => unreachable!(),
            };
            requests.push(request);
        }
        
        requests
    }
}
```

#### 3. 単体テスト実装
```rust
#[cfg(test)]
mod unit_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_stream_connection_handler_creation() {
        let harness = StreamTestHarness::new();
        let requests = vec![harness.test_data_generator.generate_valid_import_request()];
        let (req_stream, _) = create_test_streaming_request(requests);
        
        let handler = StreamConnectionHandler::new(req_stream);
        assert!(handler.is_ok());
    }

    #[tokio::test]
    async fn test_stream_message_processor_basic_flow() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        let connection_id = "test-connection-001".to_string();
        
        let processor = StreamMessageProcessor::new(connection_id.clone(), service);
        
        // 正常なリクエストの処理テスト
        let request = harness.test_data_generator.generate_valid_import_request();
        let response = processor.handle_stream_request(request, 1).await;
        
        assert!(matches!(response.message, Some(stream_response::Message::ImportAsset(_))));
    }

    #[tokio::test]
    async fn test_individual_operation_handlers() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        // ImportAsset handler test
        let import_req = ImportAssetRequest {
            asset_path: "Assets/Textures/test.png".to_string(),
        };
        
        let response = ImportAssetStreamHandler::handle(
            &service,
            import_req,
            "test-connection".to_string(),
            1,
        ).await;
        
        match response.message {
            Some(stream_response::Message::ImportAsset(import_resp)) => {
                assert!(import_resp.asset.is_some());
                assert!(import_resp.error.is_none());
            }
            _ => panic!("Expected ImportAsset response"),
        }
        
        // MoveAsset handler test
        let move_req = MoveAssetRequest {
            src_path: "Assets/Scripts/old.cs".to_string(),
            dst_path: "Assets/Scripts/new.cs".to_string(),
        };
        
        let response = MoveAssetStreamHandler::handle(
            &service,
            move_req,
            "test-connection".to_string(),
            2,
        ).await;
        
        match response.message {
            Some(stream_response::Message::MoveAsset(move_resp)) => {
                assert!(move_resp.asset.is_some());
                assert!(move_resp.error.is_none());
            }
            _ => panic!("Expected MoveAsset response"),
        }
    }

    #[tokio::test]
    async fn test_validation_engine() {
        let validation_engine = StreamValidationEngine::new();
        let context = ValidationContext {
            client_id: "test-client".to_string(),
            connection_id: "test-connection".to_string(),
            message_id: 1,
            timestamp: std::time::SystemTime::now(),
            client_info: None,
        };
        
        // 有効なリクエストのテスト
        let valid_request = StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest {
                    asset_path: "Assets/Textures/valid.png".to_string(),
                },
            )),
        };
        
        let result = validation_engine.validate_stream_request(&valid_request, &context).await;
        assert!(result.is_ok());
        
        // 無効なリクエストのテスト
        let invalid_request = StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest {
                    asset_path: "../invalid/path".to_string(),
                },
            )),
        };
        
        let result = validation_engine.validate_stream_request(&invalid_request, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_error_handling_edge_cases() {
        let harness = StreamTestHarness::new();
        
        // 空メッセージのテスト
        let empty_request = StreamRequest { message: None };
        let processor = StreamMessageProcessor::new(
            "test-connection".to_string(),
            Arc::clone(&harness.service),
        );
        
        let response = processor.handle_stream_request(empty_request, 1).await;
        
        // エラーレスポンスが返されることを確認
        match response.message {
            Some(stream_response::Message::ImportAsset(resp)) => {
                assert!(resp.asset.is_none());
                assert!(resp.error.is_some());
            }
            _ => panic!("Expected error response"),
        }
        
        // 無効なパスのテスト
        let invalid_path_request = harness.test_data_generator.generate_invalid_import_request();
        let response = processor.handle_stream_request(invalid_path_request, 2).await;
        
        // エラーレスポンスが返されることを確認
        match response.message {
            Some(stream_response::Message::ImportAsset(resp)) => {
                assert!(resp.asset.is_none());
                assert!(resp.error.is_some());
                
                let error = resp.error.unwrap();
                assert_eq!(error.code, 400); // Validation error
            }
            _ => panic!("Expected error response"),
        }
    }
}
```

#### 4. 統合テスト実装
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_end_to_end_streaming_flow() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        // テスト用のストリーミングリクエストを作成
        let requests = vec![
            harness.test_data_generator.generate_valid_import_request(),
            harness.test_data_generator.generate_valid_move_request(),
            StreamRequest {
                message: Some(stream_request::Message::Refresh(RefreshRequest {})),
            },
        ];
        
        let (mut request_stream, mut response_stream) = create_streaming_pair();
        
        // リクエストを送信
        let send_task = tokio::spawn(async move {
            for request in requests {
                request_stream.send(Ok(request)).await.unwrap();
            }
        });
        
        // ストリーミング処理を実行
        let stream_task = tokio::spawn(async move {
            let request = Request::new(response_stream);
            service.stream(request).await
        });
        
        // 結果を確認
        let (_, stream_result) = tokio::join!(send_task, stream_task);
        assert!(stream_result.is_ok());
        
        let response_stream = stream_result.unwrap().into_inner();
        let responses: Vec<_> = response_stream.collect().await;
        
        assert_eq!(responses.len(), 3);
        for response in responses {
            assert!(response.is_ok());
        }
    }

    #[tokio::test]
    async fn test_concurrent_streams() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        // 複数の同時ストリームをテスト
        let concurrent_streams = 10;
        let requests_per_stream = 50;
        
        let mut stream_handles = Vec::new();
        
        for stream_id in 0..concurrent_streams {
            let service_clone = Arc::clone(&service);
            let requests = harness.test_data_generator.generate_mixed_requests(requests_per_stream);
            
            let handle = tokio::spawn(async move {
                let (mut req_stream, resp_stream) = create_streaming_pair();
                
                // リクエスト送信
                let send_task = tokio::spawn(async move {
                    for request in requests {
                        req_stream.send(Ok(request)).await.unwrap();
                    }
                });
                
                // ストリーム処理
                let stream_task = tokio::spawn(async move {
                    let request = Request::new(resp_stream);
                    service_clone.stream(request).await
                });
                
                let (_, stream_result) = tokio::join!(send_task, stream_task);
                (stream_id, stream_result)
            });
            
            stream_handles.push(handle);
        }
        
        // すべてのストリームが正常に完了することを確認
        for handle in stream_handles {
            let (stream_id, result) = handle.await.unwrap();
            assert!(result.is_ok(), "Stream {} failed", stream_id);
        }
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        // エラーが発生してもストリームが継続することをテスト
        let requests = vec![
            harness.test_data_generator.generate_valid_import_request(),
            harness.test_data_generator.generate_invalid_import_request(), // エラーケース
            harness.test_data_generator.generate_valid_import_request(),
        ];
        
        let (mut req_stream, resp_stream) = create_streaming_pair();
        
        // リクエスト送信
        tokio::spawn(async move {
            for request in requests {
                let _ = req_stream.send(Ok(request)).await;
            }
        });
        
        // ストリーム処理
        let request = Request::new(resp_stream);
        let result = service.stream(request).await;
        assert!(result.is_ok());
        
        let response_stream = result.unwrap().into_inner();
        let responses: Vec<_> = response_stream.collect().await;
        
        // 3つのレスポンス（エラーケース含む）が返されることを確認
        assert_eq!(responses.len(), 3);
        
        // 最初と最後のレスポンスは成功
        assert!(responses[0].is_ok());
        assert!(responses[2].is_ok());
        
        // 中間のレスポンスはエラー（ただし、ストリームは継続）
        assert!(responses[1].is_ok()); // ストリーム自体は継続するが、内容はエラーレスポンス
    }

    #[tokio::test]
    async fn test_resource_cleanup() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        let initial_memory = get_memory_usage();
        
        // 複数のストリームを作成・終了
        for _ in 0..100 {
            let requests = harness.test_data_generator.generate_mixed_requests(10);
            let (mut req_stream, resp_stream) = create_streaming_pair();
            
            let send_task = tokio::spawn(async move {
                for request in requests {
                    let _ = req_stream.send(Ok(request)).await;
                }
            });
            
            let stream_task = tokio::spawn(async move {
                let request = Request::new(resp_stream);
                let result = service.stream(request).await;
                if let Ok(response_stream) = result {
                    let _: Vec<_> = response_stream.collect().await;
                }
            });
            
            let _ = tokio::join!(send_task, stream_task);
        }
        
        // ガベージコレクションを促進
        for _ in 0..3 {
            tokio::task::yield_now().await;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        
        let final_memory = get_memory_usage();
        
        // メモリリークがないことを確認（許容範囲内の増加）
        let memory_increase = final_memory - initial_memory;
        assert!(memory_increase < 10 * 1024 * 1024, // 10MB以下
                "Memory leak detected: {} bytes", memory_increase);
    }
}
```

#### 5. パフォーマンステスト実装
```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_throughput_performance() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        let request_count = 10000;
        let requests = harness.test_data_generator.generate_mixed_requests(request_count);
        
        let start_time = std::time::Instant::now();
        
        let (mut req_stream, resp_stream) = create_streaming_pair();
        
        let send_task = tokio::spawn(async move {
            for request in requests {
                req_stream.send(Ok(request)).await.unwrap();
            }
        });
        
        let stream_task = tokio::spawn(async move {
            let request = Request::new(resp_stream);
            let result = service.stream(request).await.unwrap();
            let response_stream = result.into_inner();
            let responses: Vec<_> = response_stream.collect().await;
            responses.len()
        });
        
        let (_, response_count) = tokio::join!(send_task, stream_task);
        let elapsed = start_time.elapsed();
        
        assert_eq!(response_count, request_count);
        
        let throughput = request_count as f64 / elapsed.as_secs_f64();
        println!("Throughput: {:.2} requests/second", throughput);
        
        // 最低スループット要件（調整可能）
        assert!(throughput > 1000.0, "Throughput too low: {:.2} req/s", throughput);
    }

    #[tokio::test]
    async fn test_memory_usage_under_load() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        let initial_memory = get_memory_usage();
        let mut max_memory = initial_memory;
        
        // 高負荷状態でのメモリ使用量をモニタリング
        let monitoring_task = tokio::spawn(async move {
            let mut max_observed = 0;
            for _ in 0..100 {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let current_memory = get_memory_usage();
                max_observed = max_observed.max(current_memory);
            }
            max_observed
        });
        
        // 同時に多数のストリームを実行
        let stream_tasks: Vec<_> = (0..50).map(|_| {
            let service_clone = Arc::clone(&service);
            let requests = harness.test_data_generator.generate_mixed_requests(200);
            
            tokio::spawn(async move {
                let (mut req_stream, resp_stream) = create_streaming_pair();
                
                let send_task = tokio::spawn(async move {
                    for request in requests {
                        req_stream.send(Ok(request)).await.unwrap();
                    }
                });
                
                let stream_task = tokio::spawn(async move {
                    let request = Request::new(resp_stream);
                    let result = service_clone.stream(request).await.unwrap();
                    let response_stream = result.into_inner();
                    let _: Vec<_> = response_stream.collect().await;
                });
                
                let _ = tokio::join!(send_task, stream_task);
            })
        }).collect();
        
        // すべてのタスクが完了するまで待機
        for task in stream_tasks {
            task.await.unwrap();
        }
        
        max_memory = monitoring_task.await.unwrap();
        
        let memory_increase = max_memory - initial_memory;
        println!("Maximum memory increase: {} MB", memory_increase / 1024 / 1024);
        
        // メモリ使用量が許容範囲内であることを確認
        assert!(memory_increase < 100 * 1024 * 1024, // 100MB以下
                "Memory usage too high: {} bytes", memory_increase);
    }

    #[tokio::test]
    async fn test_latency_performance() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        let mut latencies = Vec::new();
        
        for _ in 0..1000 {
            let request = harness.test_data_generator.generate_valid_import_request();
            
            let start_time = std::time::Instant::now();
            
            let (mut req_stream, resp_stream) = create_streaming_pair();
            req_stream.send(Ok(request)).await.unwrap();
            
            let request = Request::new(resp_stream);
            let result = service.stream(request).await.unwrap();
            let mut response_stream = result.into_inner();
            let _response = response_stream.next().await.unwrap().unwrap();
            
            let latency = start_time.elapsed();
            latencies.push(latency);
        }
        
        // 統計計算
        latencies.sort();
        let p50 = latencies[latencies.len() / 2];
        let p95 = latencies[latencies.len() * 95 / 100];
        let p99 = latencies[latencies.len() * 99 / 100];
        
        println!("Latency P50: {:?}", p50);
        println!("Latency P95: {:?}", p95);
        println!("Latency P99: {:?}", p99);
        
        // レイテンシー要件（調整可能）
        assert!(p50 < std::time::Duration::from_millis(10), "P50 latency too high");
        assert!(p95 < std::time::Duration::from_millis(50), "P95 latency too high");
        assert!(p99 < std::time::Duration::from_millis(100), "P99 latency too high");
    }

    #[tokio::test]
    async fn test_stress_test() {
        let harness = StreamTestHarness::new();
        let service = Arc::clone(&harness.service);
        
        // 長時間の負荷テスト（実際のテストでは時間を短縮）
        let test_duration = std::time::Duration::from_secs(10);
        let start_time = std::time::Instant::now();
        
        let mut request_count = 0;
        let mut error_count = 0;
        
        while start_time.elapsed() < test_duration {
            let concurrent_streams = 20;
            let requests_per_stream = 50;
            
            let tasks: Vec<_> = (0..concurrent_streams).map(|_| {
                let service_clone = Arc::clone(&service);
                let requests = harness.test_data_generator.generate_mixed_requests(requests_per_stream);
                
                tokio::spawn(async move {
                    let (mut req_stream, resp_stream) = create_streaming_pair();
                    
                    let send_task = tokio::spawn(async move {
                        for request in requests {
                            req_stream.send(Ok(request)).await.unwrap();
                        }
                    });
                    
                    let stream_task = tokio::spawn(async move {
                        let request = Request::new(resp_stream);
                        service_clone.stream(request).await
                    });
                    
                    let (_, stream_result) = tokio::join!(send_task, stream_task);
                    stream_result
                })
            }).collect();
            
            for task in tasks {
                match task.await.unwrap() {
                    Ok(_) => request_count += requests_per_stream,
                    Err(_) => error_count += 1,
                }
            }
        }
        
        let total_time = start_time.elapsed();
        let success_rate = (request_count as f64) / ((request_count + error_count) as f64) * 100.0;
        
        println!("Stress test results:");
        println!("  Duration: {:?}", total_time);
        println!("  Total requests: {}", request_count);
        println!("  Errors: {}", error_count);
        println!("  Success rate: {:.2}%", success_rate);
        
        // 最低成功率要件
        assert!(success_rate > 95.0, "Success rate too low: {:.2}%", success_rate);
    }
}
```

#### 6. セキュリティテスト実装
```rust
#[cfg(test)]
mod security_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_path_traversal_prevention() {
        let harness = StreamTestHarness::new();
        let validation_engine = StreamValidationEngine::new();
        
        let malicious_paths = vec![
            "../../../etc/passwd",
            "Assets/../../../secret",
            "Assets/..\\..\\windows\\system32",
            "Assets/normal/../../../secret",
            "Assets\\..\\..\\secret",
        ];
        
        for path in malicious_paths {
            let request = StreamRequest {
                message: Some(stream_request::Message::ImportAsset(
                    ImportAssetRequest { asset_path: path.clone() },
                )),
            };
            
            let context = ValidationContext {
                client_id: "security-test".to_string(),
                connection_id: "test-connection".to_string(),
                message_id: 1,
                timestamp: std::time::SystemTime::now(),
                client_info: None,
            };
            
            let result = validation_engine.validate_stream_request(&request, &context).await;
            assert!(result.is_err(), "Path traversal attack not blocked: {}", path);
        }
    }

    #[tokio::test]
    async fn test_injection_attack_prevention() {
        let harness = StreamTestHarness::new();
        let validation_engine = StreamValidationEngine::new();
        
        let malicious_inputs = vec![
            "Assets/Scripts/<script>alert('xss')</script>.cs",
            "Assets/Models/file'; DROP TABLE assets; --.fbx",
            "Assets/Textures/javascript:alert('xss').png",
            "Assets/Data/data:text/html,<script>alert('xss')</script>.json",
        ];
        
        for input in malicious_inputs {
            let request = StreamRequest {
                message: Some(stream_request::Message::ImportAsset(
                    ImportAssetRequest { asset_path: input.clone() },
                )),
            };
            
            let context = ValidationContext {
                client_id: "security-test".to_string(),
                connection_id: "test-connection".to_string(),
                message_id: 1,
                timestamp: std::time::SystemTime::now(),
                client_info: None,
            };
            
            let result = validation_engine.validate_stream_request(&request, &context).await;
            assert!(result.is_err(), "Injection attack not blocked: {}", input);
        }
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let harness = StreamTestHarness::new();
        let validation_engine = StreamValidationEngine::new();
        
        let context = ValidationContext {
            client_id: "rate-limit-test".to_string(),
            connection_id: "test-connection".to_string(),
            message_id: 1,
            timestamp: std::time::SystemTime::now(),
            client_info: None,
        };
        
        let request = StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest {
                    asset_path: "Assets/Textures/test.png".to_string(),
                },
            )),
        };
        
        // 大量のリクエストを送信してレート制限をテスト
        let mut success_count = 0;
        let mut rate_limited_count = 0;
        
        for _ in 0..200 {
            match validation_engine.validate_stream_request(&request, &context).await {
                Ok(_) => success_count += 1,
                Err(StreamValidationError::Security(SecurityValidationError::RateLimitExceeded { .. })) => {
                    rate_limited_count += 1;
                }
                Err(_) => {} // Other errors
            }
        }
        
        // レート制限が機能していることを確認
        assert!(rate_limited_count > 0, "Rate limiting not working");
        assert!(success_count < 200, "All requests succeeded - rate limiting failed");
        
        println!("Rate limiting test: {} succeeded, {} rate limited", 
                 success_count, rate_limited_count);
    }

    #[tokio::test]
    async fn test_large_input_handling() {
        let harness = StreamTestHarness::new();
        let validation_engine = StreamValidationEngine::new();
        
        // 非常に長いパスでのテスト
        let long_path = format!("Assets/{}/{}.png", "a".repeat(1000), "b".repeat(1000));
        
        let request = StreamRequest {
            message: Some(stream_request::Message::ImportAsset(
                ImportAssetRequest { asset_path: long_path },
            )),
        };
        
        let context = ValidationContext {
            client_id: "large-input-test".to_string(),
            connection_id: "test-connection".to_string(),
            message_id: 1,
            timestamp: std::time::SystemTime::now(),
            client_info: None,
        };
        
        let result = validation_engine.validate_stream_request(&request, &context).await;
        
        // 長すぎる入力が適切に拒否されることを確認
        assert!(result.is_err(), "Large input not rejected");
        
        match result {
            Err(StreamValidationError::Security(SecurityValidationError::PathTooLong { .. })) => {
                // 期待される結果
            }
            _ => panic!("Unexpected error type for large input"),
        }
    }
}
```

#### 7. テスト実行とレポート機能
```rust
/// Test performance monitoring
pub struct TestPerformanceMonitor {
    test_results: Arc<Mutex<Vec<TestResult>>>,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_name: String,
    pub duration: std::time::Duration,
    pub success: bool,
    pub memory_usage: Option<usize>,
    pub error_message: Option<String>,
}

impl TestPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            test_results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record_test_result(&self, result: TestResult) {
        if let Ok(mut results) = self.test_results.lock() {
            results.push(result);
        }
    }

    pub fn generate_report(&self) -> TestReport {
        let results = self.test_results.lock().unwrap();
        
        let total_tests = results.len();
        let successful_tests = results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - successful_tests;
        
        let total_duration: std::time::Duration = results.iter().map(|r| r.duration).sum();
        let average_duration = if total_tests > 0 {
            total_duration / total_tests as u32
        } else {
            std::time::Duration::default()
        };
        
        TestReport {
            total_tests,
            successful_tests,
            failed_tests,
            total_duration,
            average_duration,
            failed_test_details: results
                .iter()
                .filter(|r| !r.success)
                .cloned()
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct TestReport {
    pub total_tests: usize,
    pub successful_tests: usize,
    pub failed_tests: usize,
    pub total_duration: std::time::Duration,
    pub average_duration: std::time::Duration,
    pub failed_test_details: Vec<TestResult>,
}

// Helper functions
fn create_streaming_pair() -> (
    tokio::sync::mpsc::Sender<Result<StreamRequest, Status>>,
    tokio_stream::wrappers::ReceiverStream<Result<StreamRequest, Status>>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(1000);
    (tx, tokio_stream::wrappers::ReceiverStream::new(rx))
}

fn get_memory_usage() -> usize {
    // プラットフォーム固有のメモリ使用量取得
    // 実装は実際の環境に応じて調整
    0 // スタブ実装
}

fn create_test_streaming_request(
    requests: Vec<StreamRequest>
) -> Request<Streaming<StreamRequest>> {
    // テスト用のストリーミングリクエスト作成
    // 実装は実際のgRPC設定に応じて調整
    unimplemented!() // スタブ実装
}
```

## 実装計画

### Step 1: テスト基盤の構築
1. `StreamTestHarness`の実装
2. `TestDataGenerator`の実装
3. テストヘルパー関数の作成

### Step 2: 単体テスト実装
1. 各コンポーネントの単体テスト
2. エラーケースのテスト
3. バリデーション機能のテスト

### Step 3: 統合テスト実装
1. エンドツーエンドテスト
2. 並行処理テスト
3. エラー回復テスト

### Step 4: パフォーマンステスト実装
1. スループットテスト
2. レイテンシーテスト
3. メモリ使用量テスト
4. ストレステスト

### Step 5: セキュリティテスト実装
1. 入力検証テスト
2. 攻撃シミュレーションテスト
3. レート制限テスト

### Step 6: テストレポート機能
1. 実行結果の記録
2. パフォーマンス監視
3. レポート生成機能

## 成功基準

### カバレッジ
- コードカバレッジ90%以上
- エラーケース100%カバー

### パフォーマンス
- スループット1000req/s以上
- P95レイテンシー50ms以下
- メモリ使用量増加100MB以下

### セキュリティ
- 既知攻撃パターン100%ブロック
- レート制限機能の確認

## 次のステップ
テスト実装完了後の最終統合テストとドキュメント更新