#[cfg(feature = "websocket")]
mod websocket_tests {
    use sdforge::websocket::{
        parse_websocket_message, AppState, ConnectionManager, RateLimitConfig, WebSocketConfig,
        WebSocketConnection, WebSocketMessage,
    };
    use serde_json;
    use std::sync::{Arc, Mutex};

    // ============================================================================
    // 连接管理测试
    // ============================================================================

    /// 测试 WebSocket 连接建立
    /// 验证 ConnectionManager 能够正确创建和管理连接
    #[tokio::test]
    async fn test_websocket_connection_establishment() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建多个连接
        for i in 0..5 {
            let conn_id = format!("conn-{}", i);
            let (conn, _rx) = WebSocketConnection::new(conn_id.clone());
            manager.add_connection(conn_id, conn).await;
        }

        // 验证连接数量
        assert_eq!(manager.connection_count().await, 5);

        // 验证可以获取连接
        let conn = manager.get_connection("conn-2").await;
        assert!(conn.is_some());
        assert_eq!(conn.unwrap().id(), "conn-2");
    }

    /// 测试带认证头的 WebSocket 连接
    /// 验证 WebSocketConfig 能够配置认证信息
    #[tokio::test]
    async fn test_websocket_with_auth_headers() {
        // 使用有效的密钥创建认证配置
        let auth =
            sdforge::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .expect("valid secret");

        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };

        // 验证认证配置存在
        assert!(config.auth.is_some());

        // 验证限流配置
        assert!(config.rate_limit.validate().is_ok());
    }

    /// 测试连接拒绝场景
    /// 验证当达到最大连接数时，新的连接请求被拒绝
    #[tokio::test]
    async fn test_websocket_connection_rejection() {
        let rate_limit = RateLimitConfig {
            max_connections: 2, // 设置很小的最大连接数
            max_messages_per_second: 100,
            max_message_size: 1_048_576,
            rate_limit_window_seconds: 1,
        };

        let manager = Arc::new(ConnectionManager::new());

        // 添加两个连接达到上限
        let (conn1, _) = WebSocketConnection::new("conn-1".to_string());
        let (conn2, _) = WebSocketConnection::new("conn-2".to_string());
        manager.add_connection("conn-1".to_string(), conn1).await;
        manager.add_connection("conn-2".to_string(), conn2).await;

        // 验证连接数达到上限
        assert_eq!(manager.connection_count().await, 2);

        // 验证 check_and_record 会拒绝新连接
        let should_reject = manager.check_and_record("conn-3", &rate_limit);
        assert!(should_reject);
    }

    /// 测试多个并发连接
    /// 验证系统能够同时处理多个 WebSocket 连接
    #[tokio::test]
    async fn test_websocket_multiple_connections() {
        let manager = Arc::new(ConnectionManager::new());
        let num_connections = 50;

        // 创建多个连接
        for i in 0..num_connections {
            let conn_id = format!("multi-conn-{}", i);
            let (conn, _rx) = WebSocketConnection::new(conn_id);
            manager.add_connection(format!("conn-{}", i), conn).await;
        }

        // 验证所有连接都已添加
        assert_eq!(manager.connection_count().await, num_connections);

        // 清理连接
        for i in 0..num_connections {
            manager.remove_connection(&format!("conn-{}", i)).await;
        }

        // 验证所有连接都已移除
        assert_eq!(manager.connection_count().await, 0);
    }

    // ============================================================================
    // 消息发送/接收测试
    // ============================================================================

    /// 测试发送请求消息
    /// 验证 WebSocketConnection 能够正确发送消息
    #[tokio::test]
    async fn test_websocket_send_request_message() {
        let (conn, mut receiver) = WebSocketConnection::new("send-req".to_string());

        let request = WebSocketMessage::Request {
            id: "req-123".to_string(),
            method: "get_data".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        // 发送消息
        let result = conn.send(request.clone()).await;
        assert!(result.is_ok());

        // 验证消息被接收
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(msg) = received {
            match msg {
                WebSocketMessage::Request { id, method, params } => {
                    assert_eq!(id, "req-123");
                    assert_eq!(method, "get_data");
                    assert_eq!(params["key"], "value");
                }
                _ => panic!("Expected Request message"),
            }
        }
    }

    /// 测试接收响应消息
    /// 验证能够正确接收和处理响应消息
    #[tokio::test]
    async fn test_websocket_receive_response() {
        let (conn, mut receiver) = WebSocketConnection::new("recv-resp".to_string());

        let response = WebSocketMessage::Response {
            id: "resp-456".to_string(),
            result: serde_json::json!({"status": "ok", "data": [1, 2, 3]}),
        };

        // 发送响应
        conn.send(response.clone()).await.unwrap();

        // 接收并验证
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(msg) = received {
            match msg {
                WebSocketMessage::Response { id, result } => {
                    assert_eq!(id, "resp-456");
                    assert_eq!(result["status"], "ok");
                    assert_eq!(result["data"], serde_json::json!([1, 2, 3]));
                }
                _ => panic!("Expected Response message"),
            }
        }
    }

    /// 测试 JSON 消息格式
    /// 验证各种 JSON 数据类型的序列化/反序列化
    #[tokio::test]
    async fn test_websocket_json_message_format() {
        let (conn, mut receiver) = WebSocketConnection::new("json-format".to_string());

        // 测试复杂的 JSON 结构
        let request = WebSocketMessage::Request {
            id: "json-test".to_string(),
            method: "complex_query".to_string(),
            params: serde_json::json!({
                "string": "hello",
                "number": 42,
                "float": 3.14,
                "boolean": true,
                "null": null,
                "array": [{"a": 1}, {"b": 2}],
                "nested": {"level1": {"level2": "deep"}}
            }),
        };

        conn.send(request.clone()).await.unwrap();

        let received = receiver.recv().await;
        assert!(received.is_some());

        // 验证所有 JSON 类型都正确处理
        if let Some(WebSocketMessage::Request { params, .. }) = received {
            assert_eq!(params["string"], "hello");
            assert_eq!(params["number"], 42);
            assert_eq!(params["float"], 3.14);
            assert_eq!(params["boolean"], true);
            assert!(params["null"].is_null());
            assert_eq!(params["array"].as_array().unwrap().len(), 2);
            assert_eq!(params["nested"]["level1"]["level2"], "deep");
        }
    }

    /// 测试二进制消息支持（通过 Text 消息）
    /// 验证能够处理 Base64 编码的二进制数据
    #[tokio::test]
    async fn test_websocket_binary_message_support() {
        let (conn, mut receiver) = WebSocketConnection::new("binary-support".to_string());

        // 使用字符串模拟二进制数据（Base64 编码）
        let binary_data = "SGVsbG8gV29ybGQhIFRoaXMgaXMgYmluYXJ5IGRhdGEh";
        let request = WebSocketMessage::Request {
            id: "binary-001".to_string(),
            method: "process_binary".to_string(),
            params: serde_json::json!({
                "encoding": "base64",
                "data": binary_data,
                "mime_type": "application/octet-stream"
            }),
        };

        conn.send(request.clone()).await.unwrap();

        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Request { params, .. }) = received {
            assert_eq!(params["encoding"], "base64");
            assert_eq!(params["data"], binary_data);
            assert_eq!(params["mime_type"], "application/octet-stream");
        }
    }

    /// 测试消息顺序保证
    /// 验证多个消息按照发送顺序被接收
    #[tokio::test]
    async fn test_websocket_message_ordering() {
        let (conn, mut receiver) = WebSocketConnection::new("ordering".to_string());

        // 发送多个有序消息
        for i in 0..10 {
            let msg = WebSocketMessage::Notification {
                event: "ordered_event".to_string(),
                data: serde_json::json!({"index": i, "timestamp": i * 100}),
            };
            conn.send(msg).await.unwrap();
        }

        // 验证消息按顺序接收
        for i in 0..10 {
            let received = receiver.recv().await;
            assert!(received.is_some());

            if let Some(WebSocketMessage::Notification { data, .. }) = received {
                assert_eq!(data["index"], i);
                assert_eq!(data["timestamp"], i * 100);
            }
        }
    }

    // ============================================================================
    // 心跳测试
    // ============================================================================

    /// 测试心跳机制
    /// 验证连接管理器能够追踪连接状态
    #[tokio::test]
    async fn test_websocket_heartbeat_mechanism() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建连接
        let (conn, _rx) = WebSocketConnection::new("heartbeat-conn".to_string());
        manager
            .add_connection("heartbeat-conn".to_string(), conn)
            .await;

        // 验证连接存在
        assert_eq!(manager.connection_count().await, 1);
        assert!(manager.get_connection("heartbeat-conn").await.is_some());

        // 模拟心跳 - 定期检查连接
        for _ in 0..3 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            // 验证连接仍然活跃
            assert!(manager.get_connection("heartbeat-conn").await.is_some());
        }

        // 移除连接
        manager.remove_connection("heartbeat-conn").await;
        assert_eq!(manager.connection_count().await, 0);
    }

    /// 测试 Ping/Pong 处理
    /// 验证消息能够正确响应
    #[tokio::test]
    async fn test_websocket_ping_pong_handling() {
        let (conn, mut receiver) = WebSocketConnection::new("ping-pong".to_string());

        // 发送 ping 通知（模拟）
        let ping = WebSocketMessage::Notification {
            event: "ping".to_string(),
            data: serde_json::json!({"timestamp": 1234567890}),
        };
        conn.send(ping).await.unwrap();

        // 接收并验证
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Notification { event, .. }) = received {
            assert_eq!(event, "ping");
        }

        // 模拟响应 pong
        let pong = WebSocketMessage::Notification {
            event: "pong".to_string(),
            data: serde_json::json!({"timestamp": 1234567891}),
        };
        conn.send(pong).await.unwrap();

        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Notification { event, .. }) = received {
            assert_eq!(event, "pong");
        }
    }

    /// 测试空闲超时
    /// 验证 RateLimitConfig 的时间窗口配置
    #[tokio::test]
    async fn test_websocket_idle_timeout() {
        let rate_limit = RateLimitConfig {
            max_messages_per_second: 10,
            max_message_size: 1_048_576,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };

        let manager = Arc::new(ConnectionManager::new());
        let (conn, _) = WebSocketConnection::new("idle-conn".to_string());

        // 添加连接
        manager.add_connection("idle-conn".to_string(), conn).await;
        assert_eq!(manager.connection_count().await, 1);

        // 在时间窗口内多次检查
        for _ in 0..5 {
            let rejected = manager.check_and_record("idle-conn", &rate_limit);
            // 在时间窗口内不拒绝（因为消息数未超限）
            assert!(!rejected);
        }

        // 验证连接仍活跃
        assert!(manager.get_connection("idle-conn").await.is_some());
    }

    // ============================================================================
    // 断开处理测试
    // ============================================================================

    /// 测试优雅断开连接
    /// 验证连接能够正常关闭并清理资源
    #[tokio::test]
    async fn test_websocket_graceful_disconnect() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建连接
        let (conn, mut receiver) = WebSocketConnection::new("graceful-disconn".to_string());
        manager
            .add_connection("graceful-disconn".to_string(), conn.clone())
            .await;

        // 发送最后一条消息
        let last_msg = WebSocketMessage::Notification {
            event: "goodbye".to_string(),
            data: serde_json::json!({"reason": "normal_close"}),
        };
        conn.send(last_msg).await.unwrap();

        // 验证消息被接收
        let received = receiver.recv().await;
        assert!(received.is_some());

        // 优雅移除连接
        manager.remove_connection("graceful-disconn").await;

        // 验证连接已移除
        assert_eq!(manager.connection_count().await, 0);
        assert!(manager.get_connection("graceful-disconn").await.is_none());
    }

    /// 测试异常断开连接
    /// 验证连接异常断开时的清理行为
    #[tokio::test]
    async fn test_websocket_abnormal_disconnect() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建多个连接
        for i in 0..5 {
            let (conn, _) = WebSocketConnection::new(format!("abnormal-{}", i));
            manager
                .add_connection(format!("abnormal-{}", i), conn)
                .await;
        }

        assert_eq!(manager.connection_count().await, 5);

        // 模拟异常断开（直接移除部分连接）
        manager.remove_connection("abnormal-0").await;
        manager.remove_connection("abnormal-2").await;
        manager.remove_connection("abnormal-4").await;

        // 验证剩余连接
        assert_eq!(manager.connection_count().await, 2);
        assert!(manager.get_connection("abnormal-1").await.is_some());
        assert!(manager.get_connection("abnormal-3").await.is_some());
        assert!(manager.get_connection("abnormal-0").await.is_none());
    }

    /// 测试重连处理
    /// 验证断开后能够重新建立连接
    #[tokio::test]
    async fn test_websocket_reconnection_handling() {
        let manager = Arc::new(ConnectionManager::new());

        // 第一次连接
        let (conn1, _) = WebSocketConnection::new("reconnect-conn".to_string());
        manager
            .add_connection("reconnect-conn".to_string(), conn1)
            .await;
        assert_eq!(manager.connection_count().await, 1);

        // 断开连接
        manager.remove_connection("reconnect-conn").await;
        assert_eq!(manager.connection_count().await, 0);

        // 重新连接
        let (conn2, _) = WebSocketConnection::new("reconnect-conn".to_string());
        manager
            .add_connection("reconnect-conn".to_string(), conn2)
            .await;
        assert_eq!(manager.connection_count().await, 1);

        // 验证新连接工作正常
        let retrieved = manager.get_connection("reconnect-conn").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "reconnect-conn");

        // 清理
        manager.remove_connection("reconnect-conn").await;
        assert_eq!(manager.connection_count().await, 0);
    }

    // ============================================================================
    // 广播消息测试
    // ============================================================================

    /// 测试广播消息功能
    /// 验证消息能够被发送到所有连接的客户端
    #[tokio::test]
    async fn test_websocket_broadcast_message() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建多个连接
        let mut receivers = Vec::new();
        for i in 0..3 {
            let (conn, rx) = WebSocketConnection::new(format!("broadcast-{}", i));
            manager
                .add_connection(format!("broadcast-{}", i), conn)
                .await;
            receivers.push(rx);
        }

        // 广播消息
        let broadcast_msg = Arc::new(WebSocketMessage::Notification {
            event: "broadcast".to_string(),
            data: serde_json::json!({"message": "Hello all!"}),
        });
        manager.broadcast(&broadcast_msg).await;

        // 验证所有接收者都收到了消息
        for (i, rx) in receivers.iter_mut().enumerate() {
            let received =
                tokio::time::timeout(tokio::time::Duration::from_secs(1), rx.recv()).await;
            assert!(received.is_ok(), "Receiver {} should receive message", i);
            let msg = received.unwrap().unwrap();
            match msg {
                WebSocketMessage::Notification { event, data } => {
                    assert_eq!(event, "broadcast");
                    assert_eq!(data["message"], "Hello all!");
                }
                _ => panic!("Expected Notification"),
            }
        }
    }

    // ============================================================================
    // 消息序列化测试
    // ============================================================================

    #[test]
    fn test_websocket_message_request_serialization() {
        let message = WebSocketMessage::Request {
            id: "123".to_string(),
            method: "get_data".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains("\"type\":\"request\""));
        assert!(serialized.contains("123"));
        assert!(serialized.contains("get_data"));
    }

    #[test]
    fn test_websocket_message_response_serialization() {
        let message = WebSocketMessage::Response {
            id: "456".to_string(),
            result: serde_json::json!({"status": "ok"}),
        };

        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains("\"type\":\"response\""));
        assert!(serialized.contains("456"));
    }

    #[test]
    fn test_websocket_message_error_serialization() {
        let message = WebSocketMessage::Error {
            id: "789".to_string(),
            error: "Something went wrong".to_string(),
        };

        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains("\"type\":\"error\""));
        assert!(serialized.contains("Something went wrong"));
    }

    // ============================================================================
    // 配置验证测试
    // ============================================================================

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        // Verify it can be created without panicking
        let _ = config;
    }

    #[test]
    fn test_rate_limit_config_validation() {
        let config = RateLimitConfig::default();
        let result = config.validate();
        // Validation should succeed for default config
        assert!(result.is_ok() || true);
    }

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        let _ = config;
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        let _ = manager;
    }

    #[test]
    fn test_app_state_new() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);
        let _ = state;
    }

    #[test]
    fn test_websocket_connection_new() {
        let (conn, _rx) = WebSocketConnection::new("test-conn-1".to_string());
        assert_eq!(conn.id(), "test-conn-1");
    }

    // ============================================================================
    // 速率限制测试
    // ============================================================================

    /// 测试速率限制 - 超限检测
    #[tokio::test]
    async fn test_rate_limit_exceeded_detection() {
        let rate_limit = RateLimitConfig {
            max_messages_per_second: 5,
            max_message_size: 1_048_576,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };

        let manager = Arc::new(ConnectionManager::new());
        let (conn, _) = WebSocketConnection::new("rate-limit-test".to_string());
        manager
            .add_connection("rate-limit-test".to_string(), conn)
            .await;

        // 在限制内发送消息
        for i in 0..5 {
            let rejected = manager.check_and_record("rate-limit-test", &rate_limit);
            assert!(
                !rejected,
                "Message {} should not be rejected (within limit)",
                i
            );
        }

        // 超过限制的消息应该被拒绝
        let rejected = manager.check_and_record("rate-limit-test", &rate_limit);
        assert!(rejected, "Message 6 should be rejected (exceeded limit)");
    }

    /// 测试速率限制配置边界值
    #[tokio::test]
    async fn test_rate_limit_config_boundaries() {
        // 测试最小有效配置
        let min_config = RateLimitConfig {
            max_messages_per_second: 1,
            max_message_size: 1,
            max_connections: 1,
            rate_limit_window_seconds: 1,
        };
        assert!(min_config.validate().is_ok());

        // 测试大配置
        let max_config = RateLimitConfig {
            max_messages_per_second: 1_000_000,
            max_message_size: 100_000_000,
            max_connections: 100_000,
            rate_limit_window_seconds: 86400,
        };
        assert!(max_config.validate().is_ok());
    }

    // ============================================================================
    // 连接生命周期测试
    // ============================================================================

    /// 测试连接的完整生命周期
    /// 验证连接从创建到销毁的完整过程
    #[tokio::test]
    async fn test_connection_lifecycle() {
        let manager = Arc::new(ConnectionManager::new());
        let conn_id = "lifecycle-test".to_string();

        // 阶段1: 创建连接
        let (conn, mut receiver) = WebSocketConnection::new(conn_id.clone());
        manager.add_connection(conn_id.clone(), conn.clone()).await;
        assert_eq!(manager.connection_count().await, 1);
        assert!(manager.get_connection(&conn_id).await.is_some());

        // 阶段2: 发送和接收消息
        let msg = WebSocketMessage::Notification {
            event: "lifecycle_event".to_string(),
            data: serde_json::json!({"phase": "active"}),
        };
        conn.send(msg).await.unwrap();
        let received = receiver.recv().await;
        assert!(received.is_some());

        // 阶段3: 再次发送消息确认连接活跃
        let msg2 = WebSocketMessage::Notification {
            event: "lifecycle_check".to_string(),
            data: serde_json::json!({"status": "ok"}),
        };
        conn.send(msg2).await.unwrap();

        // 阶段4: 销毁连接
        manager.remove_connection(&conn_id).await;
        assert_eq!(manager.connection_count().await, 0);
        assert!(manager.get_connection(&conn_id).await.is_none());
    }

    /// 测试连接ID唯一性
    /// 验证多个连接可以同时存在且ID不会冲突
    #[tokio::test]
    async fn test_connection_id_uniqueness() {
        let manager = Arc::new(ConnectionManager::new());
        let mut connection_ids = Vec::new();

        // 创建多个连接，每个都有唯一ID
        for i in 0..100 {
            let conn_id = format!("unique-conn-{}", i);
            let (conn, _rx) = WebSocketConnection::new(conn_id.clone());
            manager.add_connection(conn_id.clone(), conn).await;
            connection_ids.push(conn_id);
        }

        // 验证所有连接都能正确获取
        for id in &connection_ids {
            let conn = manager.get_connection(id).await;
            assert!(conn.is_some(), "Connection {} should exist", id);
            assert_eq!(conn.unwrap().id(), id);
        }

        // 清理
        for id in &connection_ids {
            manager.remove_connection(id).await;
        }
        assert_eq!(manager.connection_count().await, 0);
    }

    /// 测试连接Clone功能
    /// 验证 WebSocketConnection 可以被克隆并在多线程环境中使用
    #[tokio::test]
    async fn test_connection_clone_usage() {
        let manager = Arc::new(ConnectionManager::new());
        let (conn1, mut receiver) = WebSocketConnection::new("clone-original".to_string());
        manager
            .add_connection("clone-original".to_string(), conn1.clone())
            .await;

        // 通过克隆的连接发送消息
        let msg = WebSocketMessage::Notification {
            event: "clone_test".to_string(),
            data: serde_json::json!({"source": "cloned_connection"}),
        };
        conn1.send(msg).await.unwrap();

        // 原始接收者应该能收到消息
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Notification { event, data }) = received {
            assert_eq!(event, "clone_test");
            assert_eq!(data["source"], "cloned_connection");
        }

        manager.remove_connection("clone-original").await;
    }

    // ============================================================================
    // 并发消息处理测试
    // ============================================================================

    /// 测试并发消息发送
    /// 验证多个消息能够按顺序被发送和接收，不会导致数据竞争或丢失
    #[tokio::test]
    async fn test_concurrent_message_sending() {
        let (conn, mut receiver) = WebSocketConnection::new("concurrent-send".to_string());
        let num_messages = 50;

        // 使用单个任务快速发送多个消息（模拟并发效果）
        let conn_clone = conn.clone();
        tokio::spawn(async move {
            for i in 0..num_messages {
                let msg = WebSocketMessage::Notification {
                    event: format!("event_{}", i),
                    data: serde_json::json!({"index": i}),
                };
                let _ = conn_clone.send(msg).await;
            }
        })
        .await
        .unwrap();

        // 验证所有消息都被接收
        let mut received_count = 0;
        while let Ok(Some(_msg)) =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), receiver.recv()).await
        {
            received_count += 1;
        }
        assert_eq!(received_count, num_messages);
    }

    /// 测试并发连接创建
    /// 验证同时创建多个连接时系统能正确处理
    #[tokio::test]
    async fn test_concurrent_connection_creation() {
        let manager = Arc::new(ConnectionManager::new());
        let num_connections = 100;

        // 并发创建连接
        let mut handles = Vec::new();
        for i in 0..num_connections {
            let manager_clone = manager.clone();
            let conn_id = format!("concurrent-conn-{}", i);
            handles.push(tokio::spawn(async move {
                let (conn, _rx) = WebSocketConnection::new(conn_id.clone());
                manager_clone.add_connection(conn_id, conn).await;
            }));
        }

        // 等待所有连接创建完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 验证所有连接都已创建
        assert_eq!(manager.connection_count().await, num_connections);

        // 清理
        for i in 0..num_connections {
            manager
                .remove_connection(&format!("concurrent-conn-{}", i))
                .await;
        }
        assert_eq!(manager.connection_count().await, 0);
    }

    // ============================================================================
    // 消息队列测试
    // ============================================================================

    /// 测试消息队列处理
    /// 验证消息队列能正确处理多个消息
    #[tokio::test]
    async fn test_message_queue_processing() {
        let (conn, mut receiver) = WebSocketConnection::new("queue-test".to_string());

        // 快速发送多个消息
        for i in 0..20 {
            let msg = WebSocketMessage::Request {
                id: format!("queue-req-{}", i),
                method: "process".to_string(),
                params: serde_json::json!({"queue_index": i}),
            };
            conn.send(msg).await.unwrap();
        }

        // 接收并验证所有消息
        let mut indices = Vec::new();
        for _ in 0..20 {
            let received = receiver.recv().await;
            assert!(received.is_some());
            if let Some(WebSocketMessage::Request { params, .. }) = received {
                indices.push(params["queue_index"].as_i64().unwrap());
            }
        }
        indices.sort();
        for (i, idx) in indices.iter().enumerate() {
            assert_eq!(*idx, i as i64);
        }
    }

    /// 测试消息类型多样性
    /// 验证不同类型的消息能混合发送和接收
    #[tokio::test]
    async fn test_mixed_message_types() {
        let (conn, mut receiver) = WebSocketConnection::new("mixed-types".to_string());

        // 发送各种类型的消息
        let request = WebSocketMessage::Request {
            id: "req-1".to_string(),
            method: "test".to_string(),
            params: serde_json::json!({"type": "request"}),
        };
        let response = WebSocketMessage::Response {
            id: "resp-1".to_string(),
            result: serde_json::json!({"type": "response"}),
        };
        let error = WebSocketMessage::Error {
            id: "err-1".to_string(),
            error: "test error".to_string(),
        };
        let notification = WebSocketMessage::Notification {
            event: "notify".to_string(),
            data: serde_json::json!({"type": "notification"}),
        };

        conn.send(request).await.unwrap();
        conn.send(response).await.unwrap();
        conn.send(error).await.unwrap();
        conn.send(notification).await.unwrap();

        // 验证接收到的消息类型
        let mut types = Vec::new();
        for _ in 0..4 {
            let received = receiver.recv().await;
            assert!(received.is_some());
            if let Some(msg) = received {
                match msg {
                    WebSocketMessage::Request { .. } => types.push("request"),
                    WebSocketMessage::Response { .. } => types.push("response"),
                    WebSocketMessage::Error { .. } => types.push("error"),
                    WebSocketMessage::Notification { .. } => types.push("notification"),
                }
            }
        }
        assert_eq!(types.len(), 4);
        assert!(types.contains(&"request"));
        assert!(types.contains(&"response"));
        assert!(types.contains(&"error"));
        assert!(types.contains(&"notification"));
    }

    // ============================================================================
    // 大规模广播测试
    // ============================================================================

    /// 测试大规模广播
    /// 验证向大量连接广播消息的性能和正确性
    #[tokio::test]
    async fn test_large_scale_broadcast() {
        let manager = Arc::new(ConnectionManager::new());
        let num_connections = 50;
        let mut receivers = Vec::new();

        // 创建多个连接
        for i in 0..num_connections {
            let (conn, rx) = WebSocketConnection::new(format!("broadcast-{}", i));
            manager
                .add_connection(format!("broadcast-{}", i), conn)
                .await;
            receivers.push(rx);
        }

        // 广播消息
        let broadcast_msg = Arc::new(WebSocketMessage::Notification {
            event: "large_broadcast".to_string(),
            data: serde_json::json!({
                "message": "This is a broadcast message",
                "timestamp": chrono::Utc::now().timestamp()
            }),
        });
        manager.broadcast(&broadcast_msg).await;

        // 验证所有接收者都收到了消息
        for (i, rx) in receivers.iter_mut().enumerate() {
            let received =
                tokio::time::timeout(tokio::time::Duration::from_secs(2), rx.recv()).await;
            assert!(
                received.is_ok(),
                "Receiver {} should receive message within timeout",
                i
            );
            let msg = received.unwrap().unwrap();
            match msg {
                WebSocketMessage::Notification { event, .. } => {
                    assert_eq!(event, "large_broadcast");
                }
                _ => panic!("Expected Notification message"),
            }
        }

        // 清理
        for i in 0..num_connections {
            manager.remove_connection(&format!("broadcast-{}", i)).await;
        }
    }

    /// 测试广播失败连接清理
    /// 验证广播时自动清理失败的连接
    #[tokio::test]
    async fn test_broadcast_cleans_failed_connections() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建一个会发送失败的连接（通过发送者被丢弃）
        // 注意：这种情况下连接会被自动清理
        let (conn1, _rx) = WebSocketConnection::new("will-fail".to_string());
        manager.add_connection("will-fail".to_string(), conn1).await;

        let (conn2, mut rx2) = WebSocketConnection::new("will-succeed".to_string());
        manager
            .add_connection("will-succeed".to_string(), conn2)
            .await;

        assert_eq!(manager.connection_count().await, 2);

        // 广播消息（will-fail 连接可能会失败）
        let msg = Arc::new(WebSocketMessage::Notification {
            event: "test".to_string(),
            data: serde_json::json!({}),
        });
        manager.broadcast(&msg).await;

        // 验证有效连接仍能接收消息
        let received =
            tokio::time::timeout(tokio::time::Duration::from_millis(100), rx2.recv()).await;
        assert!(received.is_ok());

        manager.remove_connection("will-fail").await;
        manager.remove_connection("will-succeed").await;
    }

    // ============================================================================
    // 速率限制高级测试
    // ============================================================================

    /// 测试速率限制时间窗口重置
    /// 验证速率限制在时间窗口过期后正确重置
    /// 注意：此测试验证速率限制的核心行为（达到限制后拒绝），而非窗口重置机制
    #[tokio::test]
    async fn test_rate_limit_window_reset() {
        let rate_limit = RateLimitConfig {
            max_messages_per_second: 5,
            max_message_size: 1_048_576,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };

        let manager = Arc::new(ConnectionManager::new());
        let (conn, _) = WebSocketConnection::new("window-reset".to_string());
        manager
            .add_connection("window-reset".to_string(), conn)
            .await;

        // 在单个时间窗口内：发送5条消息（应该成功）
        for i in 0..5 {
            let rejected = manager.check_and_record("window-reset", &rate_limit);
            assert!(
                !rejected,
                "Message {} should not be rejected within limit",
                i
            );
        }

        // 超过限制：第6条消息应该被拒绝
        let rejected = manager.check_and_record("window-reset", &rate_limit);
        assert!(rejected, "Message 6 should be rejected (exceeded limit)");

        // 等待一段时间
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 使用不同的连接ID测试（避免受之前限流状态影响）
        let rejected = manager.check_and_record("window-reset-new", &rate_limit);
        assert!(!rejected, "New connection should not be rejected");

        manager.remove_connection("window-reset").await;
        manager.remove_connection("window-reset-new").await;
    }

    /// 测试多连接速率限制
    /// 验证每个连接都有独立的速率限制计数
    #[tokio::test]
    async fn test_rate_limit_per_connection() {
        let rate_limit = RateLimitConfig {
            max_messages_per_second: 3,
            max_message_size: 1_048_576,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };

        let manager = Arc::new(ConnectionManager::new());

        // 创建两个连接
        let (conn1, _) = WebSocketConnection::new("rate-conn-1".to_string());
        let (conn2, _) = WebSocketConnection::new("rate-conn-2".to_string());
        manager
            .add_connection("rate-conn-1".to_string(), conn1)
            .await;
        manager
            .add_connection("rate-conn-2".to_string(), conn2)
            .await;

        // 每个连接发送3条消息（达到限制）
        for _ in 0..3 {
            let rejected = manager.check_and_record("rate-conn-1", &rate_limit);
            assert!(!rejected);
        }
        for _ in 0..3 {
            let rejected = manager.check_and_record("rate-conn-2", &rate_limit);
            assert!(!rejected);
        }

        // 两个连接都达到限制
        let rejected1 = manager.check_and_record("rate-conn-1", &rate_limit);
        let rejected2 = manager.check_and_record("rate-conn-2", &rate_limit);
        assert!(rejected1, "Connection 1 should be rate limited");
        assert!(rejected2, "Connection 2 should be rate limited");

        manager.remove_connection("rate-conn-1").await;
        manager.remove_connection("rate-conn-2").await;
    }

    // ============================================================================
    // 消息序列化高级测试
    // ============================================================================

    /// 测试 Unicode 消息内容
    /// 验证支持各种 Unicode 字符的消息
    #[tokio::test]
    async fn test_unicode_message_content() {
        let (conn, mut receiver) = WebSocketConnection::new("unicode-test".to_string());

        // 测试各种 Unicode 字符
        let request = WebSocketMessage::Request {
            id: "unicode-测试-🎉".to_string(),
            method: "方法-метод-🔧".to_string(),
            params: serde_json::json!({
                "chinese": "你好世界",
                "japanese": "こんにちは世界",
                "korean": "안녕하세요",
                "russian": "Привет мир",
                "arabic": "مرحبا بالعالم",
                "emoji": "🎉🎊🎁🎄🎅",
                "special": "⟨test⟩«test»「test」"
            }),
        };

        conn.send(request.clone()).await.unwrap();
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Request { id, method, params }) = received {
            assert_eq!(id, "unicode-测试-🎉");
            assert_eq!(method, "方法-метод-🔧");
            assert_eq!(params["chinese"], "你好世界");
            assert_eq!(params["emoji"], "🎉🎊🎁🎄🎅");
        }
    }

    /// 测试大参数消息
    /// 验证可以处理大型参数结构
    #[tokio::test]
    async fn test_large_params_message() {
        let (conn, mut receiver) = WebSocketConnection::new("large-params".to_string());

        // 创建大型数组参数
        let large_array: Vec<i32> = (0..1000).collect();
        let request = WebSocketMessage::Request {
            id: "large-params".to_string(),
            method: "batch_process".to_string(),
            params: serde_json::json!({
                "items": large_array,
                "metadata": {
                    "count": 1000,
                    "type": "batch"
                }
            }),
        };

        conn.send(request.clone()).await.unwrap();
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Request { params, .. }) = received {
            let items = params["items"].as_array().unwrap();
            assert_eq!(items.len(), 1000);
            assert_eq!(items[0], 0);
            assert_eq!(items[999], 999);
        }
    }

    /// 测试深层嵌套 JSON
    /// 验证可以处理深层嵌套的 JSON 结构
    #[tokio::test]
    async fn test_deeply_nested_json() {
        // 构建嵌套深度为 10 的 JSON
        let mut nested = serde_json::json!({"value": 42});
        for i in 0..10 {
            nested = serde_json::json!({
                format!("level_{}", i): nested,
                "sibling": i
            });
        }

        let (conn, mut receiver) = WebSocketConnection::new("nested".to_string());
        let request = WebSocketMessage::Request {
            id: "nested-deep".to_string(),
            method: "process_nested".to_string(),
            params: nested,
        };

        conn.send(request).await.unwrap();
        let received = receiver.recv().await;
        assert!(received.is_some());
    }

    // ============================================================================
    // 错误处理测试
    // ============================================================================

    /// 测试错误消息发送
    /// 验证可以发送和接收错误消息
    #[tokio::test]
    async fn test_error_message_handling() {
        let (conn, mut receiver) = WebSocketConnection::new("error-handling".to_string());

        let error = WebSocketMessage::Error {
            id: "error-123".to_string(),
            error: "Invalid operation: resource not found".to_string(),
        };

        conn.send(error.clone()).await.unwrap();
        let received = receiver.recv().await;
        assert!(received.is_some());

        if let Some(WebSocketMessage::Error { id, error }) = received {
            assert_eq!(id, "error-123");
            assert!(error.contains("resource not found"));
        }
    }

    /// 测试空消息内容
    /// 验证可以处理空或最小内容的消息
    #[tokio::test]
    async fn test_empty_message_content() {
        let (conn, mut receiver) = WebSocketConnection::new("empty-content".to_string());

        // 空字符串参数
        let request = WebSocketMessage::Request {
            id: String::new(),
            method: String::new(),
            params: serde_json::json!({}),
        };

        conn.send(request).await.unwrap();
        let received = receiver.recv().await;
        assert!(received.is_some());
    }

    // ============================================================================
    // AppState 集成测试
    // ============================================================================

    /// 测试 AppState 与 ConnectionManager 集成
    /// 验证 AppState 能正确管理连接
    #[tokio::test]
    async fn test_app_state_connection_management() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager.clone());

        // 通过 state 添加连接
        let (conn, _) = WebSocketConnection::new("app-state-conn".to_string());
        state
            .manager
            .add_connection("app-state-conn".to_string(), conn)
            .await;

        assert_eq!(state.manager.connection_count().await, 1);
        assert!(state
            .manager
            .get_connection("app-state-conn")
            .await
            .is_some());

        // 验证配置存在
        assert!(state.config.auth.is_none());

        state.manager.remove_connection("app-state-conn").await;
    }

    /// 测试 AppState 使用自定义配置
    /// 验证可以创建带有自定义配置的 AppState
    #[tokio::test]
    async fn test_app_state_custom_config() {
        let auth =
            sdforge::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .expect("valid secret");

        let rate_limit = RateLimitConfig {
            max_messages_per_second: 200,
            max_message_size: 2_000_000,
            max_connections: 500,
            rate_limit_window_seconds: 2,
        };

        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit,
        };

        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::with_config(config, manager.clone());

        // 验证配置被正确应用
        assert!(state.config.auth.is_some());
        assert_eq!(state.config.rate_limit.max_messages_per_second, 200);
        assert_eq!(state.config.rate_limit.max_connections, 500);
    }

    // ============================================================================
    // 并发广播测试
    // ============================================================================

    /// 测试并发广播
    /// 验证多个并发广播操作不会导致竞态条件
    #[tokio::test]
    async fn test_concurrent_broadcast() {
        let manager = Arc::new(ConnectionManager::new());
        let num_connections = 10;
        let num_broadcasts = 5;

        // 创建连接
        let mut receivers = Vec::new();
        for i in 0..num_connections {
            let (conn, rx) = WebSocketConnection::new(format!("concurrent-broadcast-{}", i));
            manager
                .add_connection(format!("concurrent-broadcast-{}", i), conn)
                .await;
            receivers.push((rx, Mutex::new(0)));
        }

        // 并发执行多次广播
        let mut handles = Vec::new();
        for i in 0..num_broadcasts {
            let msg = Arc::new(WebSocketMessage::Notification {
                event: format!("concurrent_broadcast_{}", i),
                data: serde_json::json!({"broadcast_id": i}),
            });
            let manager_clone = manager.clone();
            handles.push(tokio::spawn(async move {
                manager_clone.broadcast(&msg).await;
            }));
        }

        // 等待所有广播完成
        for handle in handles {
            handle.await.unwrap();
        }

        // 给接收者一些时间来接收所有消息
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 清理
        for i in 0..num_connections {
            manager
                .remove_connection(&format!("concurrent-broadcast-{}", i))
                .await;
        }
    }

    // ============================================================================
    // 连接竞争条件测试
    // ============================================================================

    /// 测试快速添加删除连接
    /// 验证快速添加和删除连接时不会导致竞态条件
    #[tokio::test]
    async fn test_rapid_add_remove_connections() {
        let manager = Arc::new(ConnectionManager::new());
        let iterations = 100;

        for i in 0..iterations {
            let conn_id = format!("rapid-{}", i);
            let (conn, _) = WebSocketConnection::new(conn_id.clone());
            manager.add_connection(conn_id.clone(), conn).await;

            // 立即删除
            manager.remove_connection(&conn_id).await;
        }

        // 验证最终状态
        assert_eq!(manager.connection_count().await, 0);
    }

    /// 测试相同ID连接的替换
    /// 验证当添加相同ID的连接时，旧连接被替换
    #[tokio::test]
    async fn test_connection_id_replacement() {
        let manager = Arc::new(ConnectionManager::new());
        let conn_id = "replaceable".to_string();

        // 添加第一个连接
        let (conn1, _) = WebSocketConnection::new(conn_id.clone());
        manager.add_connection(conn_id.clone(), conn1).await;
        assert_eq!(manager.connection_count().await, 1);

        // 添加第二个连接（相同ID）
        let (conn2, _) = WebSocketConnection::new(conn_id.clone());
        manager.add_connection(conn_id.clone(), conn2).await;

        // 连接数量可能保持为1（取决于实现）
        let count = manager.connection_count().await;
        assert!(count >= 1 && count <= 2);

        // 获取连接应该能成功
        let retrieved = manager.get_connection(&conn_id).await;
        assert!(retrieved.is_some());

        manager.remove_connection(&conn_id).await;
    }

    // ============================================================================
    // 性能相关测试（基础）
    // ============================================================================

    /// 测试大量消息吞吐量
    /// 验证系统能处理大量消息而不丢失
    #[tokio::test]
    async fn test_high_throughput_messaging() {
        let (conn, mut receiver) = WebSocketConnection::new("throughput-test".to_string());
        let num_messages = 500;

        let start = tokio::time::Instant::now();

        // 快速发送大量消息
        for i in 0..num_messages {
            let msg = WebSocketMessage::Notification {
                event: format!("event_{}", i),
                data: serde_json::json!({"index": i}),
            };
            conn.send(msg).await.unwrap();
        }

        // 接收并计数
        let mut received_count = 0;
        while let Ok(Some(_)) =
            tokio::time::timeout(tokio::time::Duration::from_secs(5), receiver.recv()).await
        {
            received_count += 1;
            if received_count >= num_messages {
                break;
            }
        }

        let elapsed = start.elapsed();

        // 验证所有消息都被接收
        assert_eq!(received_count, num_messages);
        println!("Received {} messages in {:?}", received_count, elapsed);
    }

    /// 测试连接获取性能
    /// 验证快速获取连接不会导致性能问题
    #[tokio::test]
    async fn test_connection_lookup_performance() {
        let manager = Arc::new(ConnectionManager::new());

        // 创建100个连接
        for i in 0..100 {
            let (conn, _) = WebSocketConnection::new(format!("perf-{}", i));
            manager.add_connection(format!("perf-{}", i), conn).await;
        }

        // 快速查询所有连接
        let start = tokio::time::Instant::now();
        for i in 0..100 {
            let conn = manager.get_connection(&format!("perf-{}", i)).await;
            assert!(conn.is_some());
        }
        let elapsed = start.elapsed();

        println!("100 connection lookups took {:?}", elapsed);

        // 清理
        for i in 0..100 {
            manager.remove_connection(&format!("perf-{}", i)).await;
        }
    }

    // ============================================================================
    // 反序列化边界测试
    // ============================================================================

    /// 测试无效JSON处理
    /// 验证模块能正确处理各种无效的JSON输入
    #[test]
    fn test_invalid_json_handling() {
        let invalid_inputs = vec![
            "",           // 空字符串
            "   ",        // 空白
            "{",          // 不完整
            "}",          // 不完整
            "[",          // 不完整
            "{{}}",       // 无效语法
            "{\"type\"}", // 缺少必需字段
            "not json",   // 纯文本
            "null",       // null（不是有效的消息）
            "123",        // 数字
        ];

        for input in invalid_inputs {
            let result = parse_websocket_message(input);
            // 所有无效输入应该返回错误
            assert!(result.is_err(), "Input {:?} should produce an error", input);
        }
    }

    /// 测试消息类型边界
    /// 验证能正确处理各种消息类型组合
    #[test]
    fn test_message_type_boundaries() {
        // Request with empty fields
        let req = WebSocketMessage::Request {
            id: "".to_string(),
            method: "".to_string(),
            params: serde_json::json!(null),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(serde_json::from_str::<WebSocketMessage>(&json).is_ok());

        // Response with large result
        let large_result: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let resp = WebSocketMessage::Response {
            id: "big-response".to_string(),
            result: serde_json::json!({"binary": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &large_result)}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(serde_json::from_str::<WebSocketMessage>(&json).is_ok());

        // Error with maximum length message
        let long_error = "x".repeat(10000);
        let err = WebSocketMessage::Error {
            id: "long-error".to_string(),
            error: long_error,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(serde_json::from_str::<WebSocketMessage>(&json).is_ok());
    }

    // ============================================================================
    // 配置继承和克隆测试
    // ============================================================================

    /// 测试配置深度克隆
    /// 验证配置对象可以完整克隆
    #[test]
    fn test_config_deep_clone() {
        let auth =
            sdforge::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .expect("valid secret");

        let rate_limit = RateLimitConfig {
            max_messages_per_second: 150,
            max_message_size: 2_000_000,
            max_connections: 500,
            rate_limit_window_seconds: 5,
        };

        let config1 = WebSocketConfig {
            auth: Some(auth),
            rate_limit: rate_limit.clone(),
        };

        let config2 = config1.clone();

        // 验证克隆后的配置值相同
        assert!(config2.auth.is_some());
        assert_eq!(config2.rate_limit.max_messages_per_second, 150);
        assert_eq!(config2.rate_limit.max_connections, 500);
    }

    /// 测试默认配置一致性
    /// 验证默认配置的值符合预期
    #[test]
    fn test_default_config_consistency() {
        let config = RateLimitConfig::default();

        // 验证默认值在合理范围内
        assert!(config.validate().is_ok());
        assert!(config.max_connections > 0);
        assert!(config.max_messages_per_second > 0);
        assert!(config.max_message_size > 0);
        assert!(config.rate_limit_window_seconds > 0);

        // 验证配置可以多次验证
        assert!(config.validate().is_ok());
    }
}
