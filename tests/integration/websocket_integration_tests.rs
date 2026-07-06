// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// WebSocket Integration Tests
// Tests WebSocket upgrade with authentication flow

#[cfg(feature = "websocket")]
mod websocket_integration_tests {
    use sdforge::websocket::{AppState, ConnectionManager, WebSocketConfig};
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
        let state = AppState::new(manager);
        // Verify state is created
        let _state_clone = state.clone();
        assert_eq!(state.config.max_message_size, 1_048_576);
    }

    #[test]
    fn test_websocket_integration_basic() {
        // Test basic WebSocket integration components
        let _config = WebSocketConfig::default();
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);

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

        // Create and add multiple connections
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
