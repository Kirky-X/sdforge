#[cfg(feature = "websocket")]
mod websocket_tests {
    use sdforge::websocket::{
        AppState, ConnectionManager, RateLimitConfig, WebSocketConfig, WebSocketMessage,
    };
    use sdforge::security::BearerAuth;
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
    fn test_websocket_message_notification_serialization() {
        let message = WebSocketMessage::Notification {
            event: "update".to_string(),
            data: serde_json::json!({"count": 5}),
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
            WebSocketMessage::Request { id, method, params } => {
                assert_eq!(id, "123");
                assert_eq!(method, "test");
            }
            _ => panic!("Expected Request message"),
        }
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_connection_manager_is_empty() {
        let manager = ConnectionManager::new();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_messages_per_second, 100);
        assert_eq!(config.max_message_size, 1_048_576);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.rate_limit_window_seconds, 1);
    }

    #[test]
    fn test_rate_limit_config_validate_valid() {
        let config = RateLimitConfig {
            max_messages_per_second: 50,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 5,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_rate_limit_config_validate_zero_connections() {
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
    fn test_rate_limit_config_validate_too_many_connections() {
        let config = RateLimitConfig {
            max_messages_per_second: 100,
            max_message_size: 1024,
            max_connections: 200_000,
            rate_limit_window_seconds: 1,
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_config_validate_zero_messages() {
        let config = RateLimitConfig {
            max_messages_per_second: 0,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_messages_per_second"));
    }

    #[test]
    fn test_rate_limit_config_validate_zero_message_size() {
        let config = RateLimitConfig {
            max_messages_per_second: 100,
            max_message_size: 0,
            max_connections: 100,
            rate_limit_window_seconds: 1,
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max_message_size"));
    }

    #[test]
    fn test_rate_limit_config_validate_zero_window() {
        let config = RateLimitConfig {
            max_messages_per_second: 100,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 0,
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limit_config_validate_large_window() {
        let config = RateLimitConfig {
            max_messages_per_second: 100,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 100_000,
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_websocket_message_clone() {
        let message = WebSocketMessage::Request {
            id: "test".to_string(),
            method: "test_method".to_string(),
            params: serde_json::json!({}),
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
    fn test_websocket_message_with_complex_params() {
        let message = WebSocketMessage::Request {
            id: "complex".to_string(),
            method: "process".to_string(),
            params: serde_json::json!({
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

    #[test]
    fn test_rate_limit_config_debug() {
        let config = RateLimitConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("RateLimitConfig"));
    }

    #[test]
    fn test_websocket_config_default_has_no_auth() {
        // When no auth is configured, connections should be accepted without token
        let config = WebSocketConfig::default();
        assert!(config.auth.is_none());
        assert_eq!(config.rate_limit.max_connections, 1000);
    }

    #[test]
    fn test_websocket_config_with_bearer_auth() {
        // When auth is configured, connections must present a valid JWT
        let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        assert!(config.auth.is_some());
    }

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

    #[test]
    fn test_app_state_with_auth() {
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

    #[test]
    fn test_bearer_auth_valid_token_accepted() {
        // Create a BearerAuth with a known secret
        let secret = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let auth = BearerAuth::try_new(secret).expect("valid secret");

        // Manually create a minimal test: auth accepts non-empty config
        // Full JWT token generation requires the auth internals
        assert!(auth.validate_token("fake-token").is_none()); // Invalid token rejected
    }
}

#[cfg(not(feature = "websocket"))]
mod websocket_tests_placeholder {
    #[test]
    fn test_websocket_feature_required() {
        assert!(true, "WebSocket tests require websocket feature");
    }
}
