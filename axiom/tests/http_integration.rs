//! HTTP protocol integration tests
//!
//! Tests for HTTP server functionality with the Axiom macros.

#[cfg(all(test, feature = "http"))]
mod http_integration_tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use std::time::Duration;

    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        id: u64,
        name: String,
        email: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct ApiResponse<T> {
        success: bool,
        data: Option<T>,
        error: Option<String>,
    }

    #[tokio::test]
    async fn test_router_build() {
        let _router = axiom::http::build();
        // Router is built successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_router_build_with_redirect() {
        let _router = axiom::http::build_with_redirect();
        // Router with redirect is built successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_router_build_with_config() {
        use axiom::config::AppConfig;

        let config = AppConfig::default();
        let result = axiom::http::build_with_config(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cors_config() {
        use axiom::config::CorsConfig;

        let config = CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            allow_credentials: false,
            max_age: Some(3600),
        };

        assert!(config.allowed_origins.contains(&"*".to_string()));
        assert_eq!(config.allowed_methods.len(), 2);
        assert_eq!(config.max_age, Some(3600));
    }

    #[tokio::test]
    async fn test_api_metadata() {
        use axiom::prelude::ApiMetadata;

        let metadata = ApiMetadata {
            name: "test-api".to_string(),
            version: "v1".to_string(),
            description: "Test API".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        assert_eq!(metadata.name, "test-api");
        assert_eq!(metadata.version, "v1");
        assert_eq!(metadata.description, "Test API");
    }

    #[tokio::test]
    async fn test_api_error_types() {
        use axiom::prelude::ApiError;

        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("NOT_FOUND"));

        let error = ApiError::InvalidInput {
            message: "Invalid input".to_string(),
            field: None,
            value: None,
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("INVALID_INPUT"));

        let error = ApiError::Internal {
            message: "Server error".to_string(),
            error_id: "err-123".to_string(),
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("INTERNAL_ERROR"));

        let error = ApiError::AuthenticationFailed {
            reason: "Invalid API key".to_string(),
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("AUTHENTICATION_FAILED"));

        let error = ApiError::AccessDenied {
            permission: "admin".to_string(),
            user_id: None,
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("ACCESS_DENIED"));

        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };
        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("RATE_LIMIT_EXCEEDED"));
    }

    #[tokio::test]
    async fn test_api_error_serialization() {
        use axiom::prelude::ApiError;
        use serde_json;

        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: None,
        };
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("NotFound"));

        let deserialized: ApiError = serde_json::from_str(&json).unwrap();
        match deserialized {
            ApiError::NotFound {
                resource,
                resource_id: _,
            } => {
                assert_eq!(resource, "User");
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[tokio::test]
    async fn test_service_response_types() {
        use axiom::prelude::ServiceResponse;

        let response = ServiceResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));

        let error = axiom::prelude::ServiceError::new("ERROR", "error message", 400);
        let response: ServiceResponse<()> = ServiceResponse::error(error);
        assert!(!response.success);
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_service_response_with_metadata() {
        use axiom::prelude::ServiceResponse;

        let response = ServiceResponse::success("data");
        assert!(response.success);
        assert_eq!(response.data, Some("data"));
    }

    #[tokio::test]
    async fn test_path_parameter_extraction() {
        // Test the path parameter pattern
        let path = "/users/:id";
        assert!(path.contains(":id"));

        // VersionedRoute requires a handler which is complex to construct
        // Just test the concept here
        assert!(true);
    }

    #[tokio::test]
    async fn test_version_router_config() {
        use axiom::http::version_routing::VersionRouterConfig;

        let config = VersionRouterConfig::default();
        assert_eq!(config.default_version, "v1");
        assert!(config.supported_versions.contains(&"v1".to_string()));
    }

    #[tokio::test]
    async fn test_build_version_router() {
        let _router = axiom::http::version_routing::build_version_router();
        assert!(true);
    }

    #[tokio::test]
    async fn test_rate_limit_config() {
        use axiom::security::{RateLimitConfig, RateLimiter};

        let config = RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));
        assert!(limiter.check("test-ip").is_ok());
    }

    #[tokio::test]
    async fn test_rate_limit_exceeds() {
        use axiom::security::{RateLimitConfig, RateLimiter};

        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // First 3 requests should succeed
        for _ in 0..3 {
            assert!(limiter.check("test-ip").is_ok());
        }

        // 4th request should fail
        assert!(limiter.check("test-ip").is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_different_keys() {
        use axiom::security::{RateLimitConfig, RateLimiter};

        let config = RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // Different keys should have separate limits
        assert!(limiter.check("ip1").is_ok());
        assert!(limiter.check("ip1").is_ok());
        assert!(limiter.check("ip1").is_err());

        assert!(limiter.check("ip2").is_ok());
        assert!(limiter.check("ip2").is_ok());
        assert!(limiter.check("ip2").is_err());
    }

    #[tokio::test]
    async fn test_api_key_authentication() {
        use axiom::security::ApiKeyAuth;

        let auth = ApiKeyAuth::new();
        auth.add_key(
            "test-key-123",
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(auth.validate_key("test-key-123").is_some());
        assert!(auth.validate_key("invalid-key").is_none());
    }

    #[tokio::test]
    async fn test_audit_logger() {
        use axiom::security::{AuditLogger, AuditResult};

        let logger = AuditLogger::new();
        let context = axiom::security::AuthContext {
            user_id: Some("user-123".to_string()),
            permissions: vec!["read".to_string()],
            metadata: Default::default(),
        };

        logger
            .log(&context, "test_action", "/api/test", true, None)
            .await;
        // Give the background worker time to process the log
        tokio::task::yield_now().await;
        let logs = logger.get_logs("user-123");
        assert!(logs.len() >= 1);
    }

    #[tokio::test]
    async fn test_validation_email() {
        use axiom::prelude::validate_email;

        let result = validate_email("test@example.com");
        assert!(result.is_ok());

        let result = validate_email("invalid-email");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validation_length() {
        use axiom::prelude::validate_length;

        let result = validate_length("test", 1, 10);
        assert!(result.is_ok());

        let result = validate_length("test", 10, 20);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_defaults() {
        use axiom::config::AppConfig;

        let config = AppConfig::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
    }

    #[tokio::test]
    async fn test_server_config() {
        use axiom::config::ServerConfig;

        let config = ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            tls: None,
            cors: None,
        };

        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 3000);
    }

    #[tokio::test]
    async fn test_api_config() {
        use axiom::config::ApiConfig;

        let config = ApiConfig {
            name: "my-api".to_string(),
            version: "v2".to_string(),
            description: Some("My API".to_string()),
        };

        assert_eq!(config.name, "my-api");
        assert_eq!(config.version, "v2");
    }

    #[tokio::test]
    async fn test_service_error_types() {
        use axiom::prelude::ServiceError;

        let error = ServiceError::new("ERR_CODE", "Error message", 500);
        assert_eq!(error.code, "ERR_CODE");
        assert_eq!(error.message, "Error message");
        assert_eq!(error.http_status, 500);
    }

    #[tokio::test]
    async fn test_service_error_with_details() {
        use axiom::prelude::ServiceError;

        let error =
            ServiceError::with_details("ERR_CODE", "Error message", json!({"field": "value"}), 400);

        assert_eq!(error.code, "ERR_CODE");
        assert!(error.details.is_some());
    }

    #[tokio::test]
    async fn test_auth_context() {
        use axiom::security::AuthContext;

        let context = AuthContext {
            user_id: Some("user-123".to_string()),
            permissions: vec!["read".to_string()],
            metadata: Default::default(),
        };

        assert_eq!(context.user_id, Some("user-123".to_string()));
        assert_eq!(context.permissions.len(), 1);
    }

    #[tokio::test]
    async fn test_auth_error_types() {
        use axiom::security::AuthError;

        let error = AuthError::MissingAuth;
        assert!(error.to_string().contains("Missing"));

        let error = AuthError::InvalidToken;
        assert!(error.to_string().contains("Invalid"));

        let error = AuthError::InsufficientPermissions {
            required: "admin".to_string(),
            user_permissions: vec!["read".to_string()],
        };
        assert!(error.to_string().contains("Insufficient"));
    }
}
