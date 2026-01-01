//! Integration tests for WebSocket feature

#[cfg(feature = "websocket")]
mod websocket_tests {
    use axiom::websocket::{
        BoxFuture, ConnectionManager, WebSocketConnection, WebSocketHandler, WebSocketMessage,
    };

    /// Test WebSocket message serialization/deserialization
    #[test]
    fn test_websocket_message_serialization() {
        let request = WebSocketMessage::Request {
            id: "test-id".to_string(),
            method: "test_method".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketMessage::Request { id, method, params } => {
                assert_eq!(id, "test-id");
                assert_eq!(method, "test_method");
                assert_eq!(params, serde_json::json!({"key": "value"}));
            }
            _ => panic!("Expected Request message"),
        }
    }

    /// Test WebSocket message types
    #[test]
    fn test_websocket_message_types() {
        // Request message
        let request = WebSocketMessage::Request {
            id: "1".to_string(),
            method: "test".to_string(),
            params: serde_json::json!({}),
        };
        assert!(matches!(request, WebSocketMessage::Request { .. }));

        // Response message
        let response = WebSocketMessage::Response {
            id: "1".to_string(),
            result: serde_json::json!({"status": "ok"}),
        };
        assert!(matches!(response, WebSocketMessage::Response { .. }));

        // Error message
        let error = WebSocketMessage::Error {
            id: "1".to_string(),
            error: "Test error".to_string(),
        };
        assert!(matches!(error, WebSocketMessage::Error { .. }));

        // Notification message
        let notification = WebSocketMessage::Notification {
            event: "test_event".to_string(),
            data: serde_json::json!({"key": "value"}),
        };
        assert!(matches!(
            notification,
            WebSocketMessage::Notification { .. }
        ));
    }

    /// Test WebSocket connection creation
    #[tokio::test]
    async fn test_websocket_connection_creation() {
        let (conn, mut receiver) = WebSocketConnection::new("test-connection-id".to_string());

        assert_eq!(conn.id(), "test-connection-id");

        // Test sending a message
        let message = WebSocketMessage::Notification {
            event: "test".to_string(),
            data: serde_json::json!({}),
        };
        let result = conn.send(message).await;
        assert!(result.is_ok());

        // Test receiving a message
        let received = receiver.recv().await;
        assert!(received.is_some());
    }

    /// Test ConnectionManager
    #[tokio::test]
    async fn test_connection_manager() {
        let manager = ConnectionManager::new();

        assert_eq!(manager.connection_count().await, 0);

        // Add a connection
        let (conn, _receiver) = WebSocketConnection::new("conn-1".to_string());
        manager.add_connection("conn-1".to_string(), conn).await;

        assert_eq!(manager.connection_count().await, 1);

        // Get a connection
        let retrieved = manager.get_connection("conn-1").await;
        assert!(retrieved.is_some());

        // Remove a connection
        manager.remove_connection("conn-1").await;
        assert_eq!(manager.connection_count().await, 0);
    }

    /// Test ConnectionManager broadcast
    #[tokio::test]
    async fn test_connection_manager_broadcast() {
        let manager = ConnectionManager::new();

        // Add multiple connections
        let (conn1, mut receiver1) = WebSocketConnection::new("conn-1".to_string());
        let (conn2, mut receiver2) = WebSocketConnection::new("conn-2".to_string());

        manager.add_connection("conn-1".to_string(), conn1).await;
        manager.add_connection("conn-2".to_string(), conn2).await;

        // Broadcast a message
        let message = WebSocketMessage::Notification {
            event: "broadcast_test".to_string(),
            data: serde_json::json!({"value": 42}),
        };
        manager.broadcast(message).await;

        // Verify both connections received the message
        let msg1 = receiver1.recv().await;
        let msg2 = receiver2.recv().await;

        assert!(msg1.is_some());
        assert!(msg2.is_some());
    }

    /// Test custom WebSocketHandler
    #[tokio::test]
    async fn test_custom_websocket_handler() {
        struct TestHandler;

        impl WebSocketHandler for TestHandler {
            fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
                Box::pin(async move {
                    match message {
                        WebSocketMessage::Request { id, method, .. } => {
                            WebSocketMessage::Response {
                                id,
                                result: serde_json::json!({"method": method}),
                            }
                        }
                        _ => message,
                    }
                })
            }
        }

        let handler = TestHandler;
        let request = WebSocketMessage::Request {
            id: "test-id".to_string(),
            method: "test_method".to_string(),
            params: serde_json::json!({}),
        };

        let response = handler.handle(request).await;
        assert!(matches!(response, WebSocketMessage::Response { .. }));
    }

    /// Test WebSocket route registration
    #[test]
    fn test_websocket_route() {
        use axiom::websocket::WebSocketRoute;
        use std::sync::Arc;

        struct TestHandler;

        impl WebSocketHandler for TestHandler {
            fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
                Box::pin(async move { message })
            }
        }

        let route = WebSocketRoute {
            path: "/ws/test".to_string(),
            handler: Arc::new(TestHandler),
        };

        assert_eq!(route.path, "/ws/test");
    }

    /// Test WebSocket message with complex JSON
    #[test]
    fn test_websocket_message_complex_json() {
        let complex_data = serde_json::json!({
            "nested": {
                "array": [1, 2, 3],
                "object": {"key": "value"}
            },
            "string": "test",
            "number": 42,
            "boolean": true,
            "null": null
        });

        let message = WebSocketMessage::Notification {
            event: "complex".to_string(),
            data: complex_data.clone(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

        match deserialized {
            WebSocketMessage::Notification { event, data } => {
                assert_eq!(event, "complex");
                assert_eq!(data, complex_data);
            }
            _ => panic!("Expected Notification message"),
        }
    }
}
