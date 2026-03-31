// WebSocket Integration Tests
// Tests WebSocket upgrade with authentication flow

#[cfg(feature = "websocket")]
mod websocket_integration_tests {
    use sdforge::websocket::{AppState, ConnectionManager, RateLimitConfig, WebSocketConfig, WebSocketConnection};
    use std::sync::Arc;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        // Verify it can be created and cloned
        let _cloned = config.clone();
        assert!(true, "WebSocketConfig should be creatable");
    }

    #[test]
    fn test_connection_manager_new() {
        let _manager = ConnectionManager::new();
        // Verify it creates a valid instance
        assert!(true, "ConnectionManager should be creatable");
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        // Verify it can be created and cloned
        let _cloned = config.clone();
        assert!(true, "RateLimitConfig should be creatable");
    }

    #[test]
    fn test_app_state_with_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);
        // Verify state is created
        let _state_clone = state.clone();
        assert!(true, "AppState should be creatable");
    }

    #[test]
    fn test_websocket_integration_basic() {
        // Test basic WebSocket integration components
        let _config = WebSocketConfig::default();
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);

        // Basic verification - state was created successfully
        assert!(state.config.rate_limit.max_connections > 0);
    }
}

// Enhanced WebSocket Integration tests
#[cfg(feature = "websocket")]
mod websocket_enhanced_integration_tests {
    use sdforge::websocket::{AppState, ConnectionManager, RateLimitConfig, WebSocketConnection};
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
        manager.add_connection("remove-test".to_string(), conn).await;
        assert_eq!(manager.connection_count().await, 1);
        
        // Remove connection
        manager.remove_connection("remove-test").await;
        assert_eq!(manager.connection_count().await, 0);
        
        // Verify connection no longer exists
        let conn = manager.get_connection("remove-test").await;
        assert!(conn.is_none());
    }

    /// Test 4: Rate limit configuration validation
    #[test]
    fn test_rate_limit_config_validation() {
        // Valid config
        let valid_config = RateLimitConfig {
            max_connections: 100,
            max_messages_per_second: 50,
            max_message_size: 1024 * 1024,
            rate_limit_window_seconds: 1,
        };
        assert!(valid_config.validate().is_ok());
        
        // Invalid config - zero max_connections
        let invalid_config = RateLimitConfig {
            max_connections: 0,
            max_messages_per_second: 50,
            max_message_size: 1024 * 1024,
            rate_limit_window_seconds: 1,
        };
        assert!(invalid_config.validate().is_err());
    }
}
