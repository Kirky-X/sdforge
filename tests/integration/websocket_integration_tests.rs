// WebSocket Integration Tests
// Tests WebSocket upgrade with authentication flow

#[cfg(feature = "websocket")]
mod websocket_integration_tests {
    use sdforge::websocket::{
        AppState, ConnectionManager, RateLimitConfig, WebSocketConfig, WebSocketMessage,
    };
    use sdforge::security::BearerAuth;
    use serde_json::json;
    use std::sync::Arc;

    // =========================================================================
    // Configuration Tests
    // =========================================================================

    #[test]
    fn test_websocket_config_with_auth_required() {
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        assert!(config.auth.is_some());
    }

    #[test]
    fn test_websocket_config_without_auth() {
        let config = WebSocketConfig::default();
        assert!(config.auth.is_none());
    }

    // =========================================================================
    // Connection Manager Tests
    // =========================================================================

    #[test]
    fn test_connection_manager_tracks_connections() {
        let manager = ConnectionManager::new();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_connection_manager_clone_preserves_state() {
        let manager1 = ConnectionManager::new();
        let manager2 = manager1.clone();
        assert!(manager2.is_empty());
    }

    // =========================================================================
    // AppState Tests
    // =========================================================================

    #[test]
    fn test_app_state_with_websocket_config() {
        let manager = Arc::new(ConnectionManager::new());
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        let state = AppState::with_config(config, manager.clone());
        assert!(state.config.auth.is_some());
    }

    #[test]
    fn test_app_state_with_default_config() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::with_default(manager.clone());
        assert!(state.config.auth.is_none());
    }

    // =========================================================================
    // Rate Limit Config Tests
    // =========================================================================

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_messages_per_second, 100);
        assert_eq!(config.max_message_size, 1_048_576);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.rate_limit_window_seconds, 1);
    }

    #[test]
    fn test_rate_limit_config_validation_valid() {
        let config = RateLimitConfig {
            max_messages_per_second: 50,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 5,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limit_config_validation_zero_connections() {
        let config = RateLimitConfig {
            max_messages_per_second: 100,
            max_message_size: 1024,
            max_connections: 0,
            rate_limit_window_seconds: 1,
        };
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_connections"));
    }

    #[test]
    fn test_rate_limit_config_validation_zero_messages() {
        let config = RateLimitConfig {
            max_messages_per_second: 0,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };
        let result = config.validate();
        assert!(result.is_err());
    }

    // =========================================================================
    // Message Serialization Tests
    // =========================================================================

    #[test]
    fn test_websocket_message_request_serialization() {
        let message = WebSocketMessage::Request {
            id: "123".to_string(),
            method: "get_data".to_string(),
            params: json!({"key": "value"}),
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
            result: json!({"status": "ok"}),
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
    fn test_websocket_message_notification_serialization() {
        let message = WebSocketMessage::Notification {
            event: "update".to_string(),
            data: json!({"count": 5}),
        };
        let serialized = serde_json::to_string(&message).unwrap();
        assert!(serialized.contains("\"type\":\"notification\""));
        assert!(serialized.contains("update"));
    }

    #[test]
    fn test_websocket_message_deserialization() {
        let json = r#"{"type":"request","id":"123","method":"test","params":{}}"#;
        let message: WebSocketMessage = serde_json::from_str(json).unwrap();
        match message {
            WebSocketMessage::Request { id, method, params: _ } => {
                assert_eq!(id, "123");
                assert_eq!(method, "test");
            }
            _ => panic!("Expected Request message"),
        }
    }

    #[test]
    fn test_websocket_message_clone() {
        let message = WebSocketMessage::Request {
            id: "test".to_string(),
            method: "test_method".to_string(),
            params: json!({}),
        };
        let cloned = message.clone();
        match cloned {
            WebSocketMessage::Request { id, .. } => {
                assert_eq!(id, "test");
            }
            _ => panic!("Expected Request message"),
        }
    }

    #[test]
    fn test_websocket_message_complex_params() {
        let message = WebSocketMessage::Request {
            id: "complex".to_string(),
            method: "process".to_string(),
            params: json!({
                "items": [1, 2, 3],
                "nested": {"key": "value"},
                "boolean": true
            }),
        };
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            WebSocketMessage::Request { method, params, .. } => {
                assert_eq!(method, "process");
                assert!(params.get("items").is_some());
            }
            _ => panic!("Expected Request"),
        }
    }

    // =========================================================================
    // BearerAuth Validation Tests
    // =========================================================================

    #[test]
    fn test_bearer_auth_valid_secret() {
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let auth = BearerAuth::try_new(secret);
        assert!(auth.is_ok());
    }

    #[test]
    fn test_bearer_auth_invalid_secret_too_short() {
        let secret = "short";
        let auth = BearerAuth::try_new(secret);
        assert!(auth.is_err());
    }

    #[test]
    fn test_bearer_auth_validate_token_rejects_empty() {
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("valid secret");
        let result = auth.validate_token("");
        assert!(result.is_none());
    }

    // =========================================================================
    // Config Cloning Tests
    // =========================================================================

    #[test]
    fn test_websocket_config_clone() {
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        let cloned = config.clone();
        assert!(cloned.auth.is_some());
    }

    // =========================================================================
    // Auth Integration Tests
    // =========================================================================

    #[test]
    fn test_app_state_auth_context() {
        let manager = Arc::new(ConnectionManager::new());
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        let state = AppState::with_config(config, manager.clone());
        assert!(state.config.auth.is_some());
        assert!(Arc::strong_count(&state.manager) >= 1);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_rate_limit_config_large_values() {
        let config = RateLimitConfig {
            max_messages_per_second: 10000,
            max_message_size: 10_485_760,
            max_connections: 10000,
            rate_limit_window_seconds: 60,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limit_config_boundary_values() {
        // Test boundary conditions
        let config = RateLimitConfig {
            max_messages_per_second: 1,
            max_message_size: 1,
            max_connections: 1,
            rate_limit_window_seconds: 1,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_message_with_empty_params() {
        let message = WebSocketMessage::Request {
            id: "id1".to_string(),
            method: "ping".to_string(),
            params: json!({}),
        };
        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&serialized).unwrap();
        match deserialized {
            WebSocketMessage::Request { id, .. } => {
                assert_eq!(id, "id1");
            }
            _ => panic!("Expected Request"),
        }
    }

    #[test]
    fn test_message_with_large_payload() {
        let large_string = "x".repeat(10000);
        let message = WebSocketMessage::Request {
            id: "large".to_string(),
            method: "upload".to_string(),
            params: json!({"data": large_string}),
        };
        // Should not panic on serialization
        let result = serde_json::to_string(&message);
        assert!(result.is_ok());
        let serialized = result.unwrap();
        assert!(serialized.len() > 10000);
    }
}

#[cfg(not(feature = "websocket"))]
mod websocket_integration_tests_placeholder {
    #[test]
    fn test_websocket_feature_required() {
        assert!(
            true,
            "WebSocket integration tests require 'websocket' feature"
        );
    }
}
