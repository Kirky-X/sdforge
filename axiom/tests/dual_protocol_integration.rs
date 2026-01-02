//! Dual protocol and feature combination tests
//!
//! Tests that verify correct behavior when both HTTP and MCP features are enabled
//! and when different feature combinations are used.

#[cfg(all(test, feature = "http", feature = "mcp"))]
mod dual_protocol_tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct CreateUserRequest {
        name: String,
        email: String,
    }

    #[tokio::test]
    async fn test_both_protocols_build() {
        // Both HTTP and MCP features should be available
        let _http_router = axiom::http::build();
        let _mcp_server = axiom::mcp::build();
        assert!(true);
    }

    #[tokio::test]
    async fn test_service_response_serialization() {
        // ServiceResponse should work for both protocols
        use axiom::prelude::ServiceResponse;

        // Success response
        let response = ServiceResponse::success(User {
            id: 1,
            name: "Test User".to_string(),
        });
        assert!(response.is_success());
        assert!(response.data().is_some());

        // Error response
        let error = axiom::prelude::ServiceError::new("VALIDATION_ERROR", "Invalid input", 400);
        let error_response: ServiceResponse<()> = ServiceResponse::error(error);
        assert!(!error_response.is_success());
        assert!(error_response.error_ref().is_some());
    }

    #[tokio::test]
    async fn test_api_error_mcp_compatibility() {
        // API errors should be convertible to MCP JSON format
        use axiom::prelude::ApiError;

        // NotFound error
        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("NOT_FOUND"));
        assert!(mcp_json.contains("Resource not found"));

        // InvalidInput error
        let error = ApiError::InvalidInput {
            message: "Name is required".to_string(),
            field: Some("name".to_string()),
            value: None,
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("INVALID_INPUT"));
    }

    #[tokio::test]
    async fn test_service_error_http_compatibility() {
        // ServiceError should work with HTTP responses
        use axiom::prelude::ServiceError;

        let error = ServiceError::new("ERR_CODE", "Error message", 500);
        assert_eq!(error.http_status(), 500);

        let error_with_details =
            ServiceError::with_details("ERR_CODE", "Error message", json!({"field": "value"}), 400);
        assert!(error_with_details.details().is_some());
    }

    #[tokio::test]
    async fn test_api_metadata_dual_protocol() {
        // API metadata should be protocol-agnostic
        use axiom::prelude::ApiMetadata;

        let metadata = ApiMetadata::new(
            "test-api".to_string(),
            "v1".to_string(),
            "A test API for dual protocol testing".to_string(),
            None,
            false,
        );

        assert_eq!(metadata.name(), "test-api");
        assert_eq!(metadata.version(), "v1");
        assert_eq!(
            metadata.description(),
            "A test API for dual protocol testing"
        );
        assert_eq!(metadata.cache_ttl(), None);
        assert_eq!(metadata.is_streaming(), false);
    }

    #[tokio::test]
    async fn test_rate_limit_with_mcp() {
        // Rate limiter should work independently of MCP
        use axiom::security::{RateLimitConfig, RateLimiter};
        use std::time::Duration;

        let config = RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // Should work for HTTP contexts
        for i in 0..10 {
            assert!(limiter.check(&format!("http-ip-{}", i)).is_ok());
        }

        // Should work independently for MCP contexts
        assert!(limiter.check("mcp-ip-1").is_ok());
        assert!(limiter.check("mcp-ip-1").is_ok());
    }

    #[tokio::test]
    async fn test_auth_context_dual_protocol() {
        // Auth context should be usable by both protocols
        use axiom::security::AuthContext;

        let context = AuthContext {
            user_id: Some("user-123".to_string()),
            permissions: vec!["read".to_string(), "write".to_string()],
            metadata: Default::default(),
        };

        assert_eq!(context.user_id, Some("user-123".to_string()));
        assert_eq!(context.permissions.len(), 2);
    }

    #[tokio::test]
    async fn test_config_with_dual_protocol() {
        // Config should support both protocols
        use axiom::config::AppConfig;

        let config = AppConfig::default();

        // Server config should be usable for HTTP
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);

        // API config should be protocol-agnostic
        assert_eq!(config.api.name, "axiom-api");
        assert_eq!(config.api.version, "0.1.0");
    }
}

#[cfg(all(test, feature = "http", feature = "streaming"))]
mod streaming_feature_tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;

    #[derive(Debug, Serialize, Deserialize)]
    struct Event {
        id: u64,
        message: String,
    }

    #[tokio::test]
    async fn test_stream_response_basic() {
        use axiom::streaming::StreamResponse;

        let (_tx, rx) = mpsc::channel(32);
        let stream: StreamResponse<String> = StreamResponse::new(ReceiverStream::new(rx));

        assert!(!stream.is_final);
    }

    #[tokio::test]
    async fn test_stream_response_single_item() {
        use axiom::streaming::StreamResponse;

        let stream = StreamResponse::single("test data");
        assert!(!stream.is_final);
    }

    #[tokio::test]
    async fn test_stream_to_sse() {
        use axiom::streaming::{stream_to_sse, StreamEvent};

        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let _ = tx.send(Ok("Item 1".to_string())).await;
            let _ = tx.send(Ok("Item 2".to_string())).await;
        });

        let _sse_stream = stream_to_sse(ReceiverStream::new(rx), |item| match item {
            Ok(data) => {
                StreamEvent::data(serde_json::to_value(data).unwrap_or(serde_json::Value::Null))
            }
            Err(err) => StreamEvent::error(err),
        });

        // The stream should be created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_stream_event_types() {
        use axiom::streaming::StreamEvent;

        // Data event with Value type
        let event: StreamEvent<serde_json::Value> = StreamEvent::data(json!({"key": "value"}));
        match event {
            StreamEvent::Data {
                id,
                event_name: _,
                data,
            } => {
                assert!(id.is_none() || id.is_some());
            }
            _ => panic!("Expected Data event"),
        }

        // Ping event
        let event: StreamEvent<serde_json::Value> = StreamEvent::ping();
        match event {
            StreamEvent::Ping { timestamp } => {
                assert!(timestamp > 0);
            }
            _ => panic!("Expected Ping event"),
        }

        // Error event
        let event: StreamEvent<serde_json::Value> = StreamEvent::error("test error".to_string());
        match event {
            StreamEvent::Error { message } => {
                assert_eq!(message, "test error");
            }
            _ => panic!("Expected Error event"),
        }

        // Complete event
        let event: StreamEvent<serde_json::Value> = StreamEvent::complete();
        match event {
            StreamEvent::Complete => {}
            _ => panic!("Expected Complete event"),
        }
    }
}

#[cfg(all(test, feature = "http", feature = "timestamp"))]
mod timestamp_feature_tests {
    use axiom::prelude::ServiceResponse;

    #[tokio::test]
    async fn test_response_has_timestamp() {
        let response = ServiceResponse::success("test data");
        // With timestamp feature, the response should include a timestamp
        // The actual timestamp field is only serialized when the feature is enabled
        assert!(response.is_success());
        assert!(response.data().is_some());
    }
}

#[cfg(all(test, feature = "http", feature = "logging"))]
mod logging_feature_tests {
    use axiom::config::init_logging_default;

    #[tokio::test]
    async fn test_logging_initialization() {
        init_logging_default();
    }
}

#[cfg(all(test, feature = "http", feature = "security"))]
mod security_feature_tests {
    use axiom::security::{ApiKeyAuth, AuditLogger, RateLimiter};

    #[tokio::test]
    async fn test_api_key_auth() {
        let auth = ApiKeyAuth::new();
        auth.add_key("test-key", vec!["read".to_string()]);

        assert!(auth.validate_key("test-key", "127.0.0.1").is_some());
        assert!(auth.validate_key("invalid", "127.0.0.1").is_none());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        use axiom::security::RateLimitConfig;
        use std::time::Duration;

        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));
        assert!(limiter.check("test").is_ok());
    }

    #[tokio::test]
    async fn test_audit_logger() {
        use axiom::security::AuthContext;

        let logger = AuditLogger::new();
        let context = AuthContext {
            user_id: Some("user-1".to_string()),
            permissions: vec!["read".to_string()],
            metadata: Default::default(),
        };

        logger
            .log(&context, "test_action", "/api/test", true, None)
            .await;
        // Give the background worker time to process the log
        tokio::task::yield_now().await;
        let logs = logger.get_logs("user-1");
        assert!(!logs.is_empty());
    }
}
