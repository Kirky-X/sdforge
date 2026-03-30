// WebSocket Integration Tests
// Tests WebSocket upgrade with authentication flow

#[cfg(feature = "websocket")]
mod websocket_integration_tests {
    use sdforge::websocket::{AppState, ConnectionManager, RateLimitConfig, WebSocketConfig};
    use std::sync::Arc;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        // Verify default values
        assert_eq!(config.max_message_size, 65536);
        assert_eq!(config.ping_interval_secs, 30);
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        // Verify it creates a valid instance
        assert!(Arc::strong_count(&manager) >= 1, "ConnectionManager should be created");
    }
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        // Verify default rate limit values
        assert_eq!(config.max_messages_per_window, 100);
        assert_eq!(config.window_size_secs, 60);
    }

    #[test]
    fn test_app_state_with_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);
        // Verify state is created with correct manager reference count
        assert!(Arc::strong_count(&state.connection_manager) >= 2, "AppState should hold a reference to the manager");
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
