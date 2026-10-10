// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// WebSocket Integration Tests
// Tests WebSocket upgrade with authentication flow

#[cfg(feature = "websocket")]
mod websocket_integration_tests {
    use sdforge::websocket::{ConnectionManager, SdForgeState, WebSocketConfig};
    use std::sync::Arc;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        // Verify it can be created and cloned
        let _cloned = config.clone();
        // `max_message_size` is now a top-level field (default 1 MiB).
        assert_eq!(config.max_message_size, 1_048_576);
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        // Verify it creates a valid instance (construction without panic)
        let _ = manager;
    }

    #[test]
    fn test_app_state_with_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = SdForgeState::new(manager);
        // Verify state is created
        let _state_clone = state.clone();
        assert_eq!(state.config.max_message_size, 1_048_576);
    }

    #[test]
    fn test_websocket_integration_basic() {
        // Test basic WebSocket integration components
        let _config = WebSocketConfig::default();
        let manager = Arc::new(ConnectionManager::new());
        let state = SdForgeState::new(manager);

        // Basic verification - state was created successfully
        assert_eq!(state.config.max_message_size, 1_048_576);
    }
}

// Enhanced WebSocket Integration tests
#[cfg(feature = "websocket")]
mod websocket_enhanced_integration_tests {
    use sdforge::websocket::{ConnectionManager, WebSocketConnection};
    use std::sync::Arc;

    /// Test 1: WebSocket connection creation and basic operations
    #[tokio::test]
    async fn test_websocket_connection_creation() {
        let (conn, _rx) = WebSocketConnection::new("test-conn-1".to_string());

        assert_eq!(conn.id(), "test-conn-1");
    }

    /// Test 2: Connection manager with multiple connections
    #[tokio::test]
    async fn test_connection_manager_multiple_connections() {
        let manager = Arc::new(ConnectionManager::new());

        for i in 0..3 {
            let conn_id = format!("integration-conn-{}", i);
            let (conn, _rx) = WebSocketConnection::new(conn_id.clone());
            manager.add_connection(conn_id, conn).await;
        }

        // Verify connection count
        assert_eq!(manager.connection_count().await, 3);

        // Verify we can retrieve connections
        let conn = manager.get_connection("integration-conn-1").await;
        assert!(conn.is_some());
        assert_eq!(conn.unwrap().id(), "integration-conn-1");
    }

    /// Test 3: Connection removal
    #[tokio::test]
    async fn test_connection_removal() {
        let manager = Arc::new(ConnectionManager::new());

        // Add connection
        let (conn, _rx) = WebSocketConnection::new("remove-test".to_string());
        manager
            .add_connection("remove-test".to_string(), conn)
            .await;
        assert_eq!(manager.connection_count().await, 1);

        // Remove connection
        manager.remove_connection("remove-test").await;
        assert_eq!(manager.connection_count().await, 0);

        // Verify connection no longer exists
        let conn = manager.get_connection("remove-test").await;
        assert!(conn.is_none());
    }
}

/// `#[forge(ws_path = "...")]` 回归：声明式挂载路径必须由宏注册进
/// `WebSocketRoute` 并被 `websocket::build()` 消费（此前 ws_path 被解析后
/// 丢弃，路由只挂在 `/{name}` 上）。
#[cfg(feature = "websocket")]
mod websocket_ws_path_tests {
    use sdforge::websocket::{BoxFuture, WebSocketHandler, WebSocketMessage};
    use std::sync::Arc;

    struct Echo;
    impl WebSocketHandler for Echo {
        fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
            Box::pin(async move { message })
        }
    }

    #[sdforge::forge(name = "it_ws_internal", version = "v1", ws_path = "/ws/it-declared")]
    fn it_ws_internal() -> Arc<dyn WebSocketHandler> {
        Arc::new(Echo)
    }

    #[test]
    fn registered_ws_route_exposes_declared_path() {
        let routes = sdforge::get_websocket_routes();
        let route = routes
            .iter()
            .find(|r| r.name == "it_ws_internal")
            .expect("ws route registered via #[forge(ws_path)]");
        assert_eq!(route.path, "/ws/it-declared");
        assert_eq!(route.mount_path(), "/ws/it-declared");
    }

    /// 端到端：`build()` 后的路由必须在声明的 ws_path 上完成 101 升级。
    #[tokio::test]
    async fn build_serves_declared_ws_path() {
        let app = sdforge::websocket::build();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = sdforge::axum::serve(listener, app).await;
        });

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = format!(
            "GET /ws/it-declared HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        );
        sock.write_all(req.as_bytes()).await.unwrap();
        let mut buf = vec![0u8; 1024];
        let n = sock.read(&mut buf).await.unwrap();
        let head = String::from_utf8_lossy(&buf[..n]).to_string();
        assert!(
            head.starts_with("HTTP/1.1 101"),
            "declared ws_path must serve the upgrade: {head:?}"
        );
    }
}
