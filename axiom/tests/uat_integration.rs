//! User Acceptance Tests (UAT)
//!
//! These tests verify end-to-end user scenarios based on real-world usage patterns.

#[cfg(all(test, feature = "http"))]
mod uat_tests {
    use axiom::config::{CorsConfig, ServerConfig};
    use axiom::prelude::*;
    use axiom::security::{ApiKeyAuth, RateLimitConfig, RateLimiter};
    use serde_json::json;
    use std::time::Duration;

    /// UAT-001: Basic API Service Setup
    #[tokio::test]
    async fn test_uat_basic_api_service_setup() {
        // User wants to create a basic API service
        let config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            tls: None,
            cors: None,
        };

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
    }

    /// UAT-002: API with Error Handling
    #[tokio::test]
    async fn test_uat_api_error_handling() {
        // User creates API that handles various error types
        let not_found = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };

        let validation = ApiError::ValidationError {
            field: "email".to_string(),
            constraint: "must be valid email".to_string(),
        };

        let auth_error = ApiError::AuthenticationFailed {
            reason: "Invalid token".to_string(),
        };

        // Verify all error types work
        assert!(not_found.to_mcp_json().contains("NOT_FOUND"));
        assert!(validation.to_mcp_json().contains("VALIDATION_ERROR"));
        assert!(auth_error.to_mcp_json().contains("AUTHENTICATION_FAILED"));
    }

    /// UAT-003: API with Rate Limiting
    #[tokio::test]
    async fn test_uat_rate_limiting() {
        // User configures rate limiting for their API
        let config = RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // First 100 requests from the same user should succeed
        for _ in 0..100 {
            assert!(limiter.check("user-same").is_ok());
        }

        // 101st request should fail
        assert!(limiter.check("user-same").is_err());
    }

    /// UAT-004: API with Authentication
    #[tokio::test]
    async fn test_uat_api_authentication() {
        // User configures API key authentication
        let auth = ApiKeyAuth::new();
        auth.add_key("key-123", vec!["read".to_string(), "write".to_string()]);
        auth.add_key("key-456", vec!["read".to_string()]);

        assert!(auth.validate_key("key-123").is_some());
        assert!(auth.validate_key("key-456").is_some());
        assert!(auth.validate_key("key-789").is_none());
    }

    /// UAT-005: Service Response Types
    #[tokio::test]
    async fn test_uat_service_response_types() {
        // User creates various response types
        let success = ServiceResponse::success(json!({"id": 1, "name": "Test"}));
        assert!(success.success);
        assert!(success.data.is_some());

        let error_response: ServiceResponse<()> =
            ServiceResponse::error(ServiceError::new("ERR", "Error occurred", 500));
        assert!(!error_response.success);
        assert!(error_response.error.is_some());
    }

    /// UAT-006: CORS Configuration
    #[tokio::test]
    async fn test_uat_cors_configuration() {
        // User configures CORS for their API
        let cors = CorsConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            allow_credentials: true,
            max_age: Some(86400),
        };

        assert!(cors
            .allowed_origins
            .iter()
            .any(|s| s == "https://example.com"));
        assert!(cors.allow_credentials);
        assert_eq!(cors.max_age, Some(86400));
    }

    /// UAT-007: API Metadata
    #[tokio::test]
    async fn test_uat_api_metadata() {
        // User defines API metadata
        let metadata = ApiMetadata {
            name: "User Management API".to_string(),
            version: "v1.0.0".to_string(),
            description: "API for managing users".to_string(),
            cache_ttl: Some(300),
            is_streaming: false,
        };

        assert_eq!(metadata.name, "User Management API");
        assert_eq!(metadata.version, "v1.0.0");
    }

    /// UAT-008: JSON Serialization
    #[tokio::test]
    async fn test_uat_json_serialization() {
        // User serializes and deserializes data
        let original = ApiError::Internal {
            message: "Database error".to_string(),
            error_id: "ERR-001".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ApiError = serde_json::from_str(&json).unwrap();

        match deserialized {
            ApiError::Internal { message, error_id } => {
                assert_eq!(message, "Database error");
                assert_eq!(error_id, "ERR-001");
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// UAT-009: Rate Limiter Boundary Conditions
    #[tokio::test]
    async fn test_uat_rate_limiter_boundaries() {
        // User tests rate limiter with extreme values
        let strict_config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(1),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(strict_config));

        assert!(limiter.check("strict-user").is_ok());
        assert!(limiter.check("strict-user").is_err());
    }

    /// UAT-010: Auth Context with Permissions
    #[tokio::test]
    async fn test_uat_auth_context_permissions() {
        // User creates auth context with multiple permissions
        use axiom::security::AuthContext;

        let context = AuthContext {
            user_id: Some("user-123".to_string()),
            permissions: vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "admin:access".to_string(),
            ],
            metadata: Default::default(),
        };

        assert!(context.permissions.contains(&"users:read".to_string()));
        assert!(context.permissions.contains(&"admin:access".to_string()));
    }

    /// UAT-011: Service Error with Details
    #[tokio::test]
    async fn test_uat_service_error_with_details() {
        // User creates service error with additional details
        let details = serde_json::json!({
            "field": "email",
            "rejected_value": "invalid"
        });

        let error = ServiceError::with_details(
            "VALIDATION_FAILED",
            "Validation error occurred",
            details,
            422,
        );

        assert_eq!(error.code, "VALIDATION_FAILED");
        assert_eq!(error.http_status, 422);
        assert!(error.details.is_some());
    }

    /// UAT-012: API Error Variants
    #[tokio::test]
    async fn test_uat_api_error_variants() {
        // User tests all API error variants
        let errors: Vec<(ApiError, &str)> = vec![
            (
                ApiError::NotFound {
                    resource: "X".to_string(),
                    resource_id: None,
                },
                "NOT_FOUND",
            ),
            (
                ApiError::InvalidInput {
                    message: "X".to_string(),
                    field: None,
                    value: None,
                },
                "INVALID_INPUT",
            ),
            (
                ApiError::AuthenticationFailed {
                    reason: "X".to_string(),
                },
                "AUTHENTICATION_FAILED",
            ),
            (
                ApiError::AccessDenied {
                    permission: "X".to_string(),
                    user_id: None,
                },
                "ACCESS_DENIED",
            ),
            (
                ApiError::RateLimitExceeded {
                    limit: 100,
                    window_seconds: 60,
                },
                "RATE_LIMIT_EXCEEDED",
            ),
            (
                ApiError::Internal {
                    message: "X".to_string(),
                    error_id: "X".to_string(),
                },
                "INTERNAL_ERROR",
            ),
            (
                ApiError::ServiceUnavailable {
                    service: "X".to_string(),
                    retry_after: None,
                },
                "SERVICE_UNAVAILABLE",
            ),
            (
                ApiError::ValidationError {
                    field: "X".to_string(),
                    constraint: "X".to_string(),
                },
                "VALIDATION_ERROR",
            ),
        ];

        for (error, expected_code) in errors {
            let json = error.to_mcp_json();
            assert!(
                json.contains(expected_code),
                "Expected {} in: {}",
                expected_code,
                json
            );
        }
    }

    /// UAT-013: Empty and Default Configurations
    #[tokio::test]
    async fn test_uat_empty_configurations() {
        // User tests with minimal/empty configurations
        let empty_metadata = ApiMetadata {
            name: "".to_string(),
            version: "".to_string(),
            description: "".to_string(),
            cache_ttl: None,
            is_streaming: false,
        };

        assert!(empty_metadata.name.is_empty());

        let no_cors = CorsConfig {
            allowed_origins: vec![],
            allowed_methods: vec![],
            allowed_headers: vec![],
            allow_credentials: false,
            max_age: None,
        };

        assert!(no_cors.allowed_origins.is_empty());
    }

    /// UAT-014: Service Response Serialization Roundtrip
    #[tokio::test]
    async fn test_uat_response_serialization_roundtrip() {
        // User serializes and deserializes responses
        let original: ServiceResponse<serde_json::Value> =
            ServiceResponse::success(json!({"user": "test", "active": true}));

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ServiceResponse<serde_json::Value> = serde_json::from_str(&json).unwrap();

        assert!(deserialized.success);
        assert!(deserialized.data.is_some());
    }

    /// UAT-015: Multiple Rate Limiters Independent
    #[tokio::test]
    async fn test_uat_multiple_rate_limiters_independent() {
        // User creates multiple rate limiters that operate independently
        let config1 = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let config2 = RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter1 = RateLimiter::new(Some(config1));
        let limiter2 = RateLimiter::new(Some(config2));

        // Use all 5 slots in limiter1
        for _ in 0..5 {
            assert!(limiter1.check("shared-key").is_ok());
        }
        assert!(limiter1.check("shared-key").is_err());

        // limiter2 still has 10 slots available
        for _ in 0..10 {
            assert!(limiter2.check("shared-key").is_ok());
        }
        assert!(limiter2.check("shared-key").is_err());
    }
}
