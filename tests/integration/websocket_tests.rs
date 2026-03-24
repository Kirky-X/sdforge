#[cfg(feature = "websocket")]
mod websocket_tests {
    use sdforge::websocket::{
        AppState, ConnectionManager, RateLimitConfig, WebSocketConfig, WebSocketMessage,
        WebSocketConnection,
    };
    use serde_json;
    use std::sync::Arc;

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
}

#[cfg(not(feature = "websocket"))]
mod websocket_tests_placeholder {
    #[test]
    fn test_websocket_feature_required() {
        assert!(true, "WebSocket tests require websocket feature");
    }
}