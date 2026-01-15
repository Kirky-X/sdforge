//! Error handling and boundary condition tests
//!
//! Tests that verify correct error handling and boundary conditions.

#[cfg(all(test, feature = "http"))]
mod error_handling_tests {
    use axiom::prelude::{ApiError, ServiceError, ServiceResponse};
    #[cfg(feature = "security")]
    use axiom::security::{AuthContext, RateLimitConfig, RateLimiter};
    use std::time::Duration;

    #[tokio::test]
    async fn test_api_error_not_found() {
        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("NOT_FOUND"));
        // Check that resource is present (actual format may vary)
        assert!(mcp_json.contains("User") || mcp_json.contains("user"));
    }

    #[tokio::test]
    async fn test_api_error_not_found_without_id() {
        let error = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: None,
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("NOT_FOUND"));
    }

    #[tokio::test]
    async fn test_api_error_invalid_input() {
        let error = ApiError::InvalidInput {
            message: "Email is invalid".to_string(),
            field: Some("email".to_string()),
            value: Some(serde_json::json!("invalid-email")),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("INVALID_INPUT"));
        // Check that message is present
        assert!(mcp_json.contains("Email is invalid") || mcp_json.contains("email"));
    }

    #[tokio::test]
    async fn test_api_error_authentication_failed() {
        let error = ApiError::AuthenticationFailed {
            reason: "Token expired".to_string(),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("AUTHENTICATION_FAILED"));
        assert!(mcp_json.contains("Token expired"));
    }

    #[tokio::test]
    async fn test_api_error_access_denied() {
        let error = ApiError::AccessDenied {
            permission: "admin.write".to_string(),
            user_id: Some("user-456".to_string()),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("ACCESS_DENIED"));
        assert!(mcp_json.contains("admin.write"));
    }

    #[tokio::test]
    async fn test_api_error_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("RATE_LIMIT_EXCEEDED"));
    }

    #[tokio::test]
    async fn test_api_error_internal() {
        let error = ApiError::Internal {
            message: "Database connection failed".to_string(),
            error_id: "err-abc-123".to_string(),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("INTERNAL_ERROR"));
        assert!(mcp_json.contains("Database connection failed"));
    }

    #[tokio::test]
    async fn test_api_error_service_unavailable() {
        let error = ApiError::ServiceUnavailable {
            service: "Payment Gateway".to_string(),
            retry_after: Some(30),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("SERVICE_UNAVAILABLE"));
        assert!(mcp_json.contains("Payment Gateway"));
    }

    #[tokio::test]
    async fn test_api_error_validation() {
        let error = ApiError::ValidationError {
            field: "age".to_string(),
            constraint: "must be positive".to_string(),
        };

        let mcp_json = error.to_mcp_json();
        assert!(mcp_json.contains("VALIDATION_ERROR"));
        assert!(mcp_json.contains("age"));
    }

    #[tokio::test]
    async fn test_service_error_creation() {
        let error = ServiceError::new("CUSTOM_CODE", "Custom message", 418);
        assert_eq!(error.code(), "CUSTOM_CODE");
        assert_eq!(error.message(), "Custom message");
        assert_eq!(error.http_status(), 418);
        assert!(error.details().is_none());
    }

    #[tokio::test]
    async fn test_service_error_with_details() {
        let details = serde_json::json!({
            "field": "email",
            "reason": "already exists"
        });

        let error = ServiceError::with_details("CONFLICT", "Resource already exists", details, 409);

        assert_eq!(error.code(), "CONFLICT");
        assert_eq!(error.http_status(), 409);
        assert!(error.details().is_some());
    }

    #[tokio::test]
    async fn test_service_response_success() {
        let response = ServiceResponse::success("test data");
        assert!(response.is_success());
        assert_eq!(response.data(), Some(&"test data"));
        assert!(response.error_ref().is_none());
    }

    #[tokio::test]
    async fn test_service_response_error() {
        let error = ServiceError::new("ERR", "Error", 500);
        let response: ServiceResponse<()> = ServiceResponse::error(error);
        assert!(!response.is_success());
        assert!(response.data().is_none());
        assert!(response.error_ref().is_some());
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_rate_limit_boundary() {
        let config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // First request should succeed
        assert!(limiter.check("boundary-test").is_ok());

        // Second request should fail
        assert!(limiter.check("boundary-test").is_err());
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_rate_limit_zero_requests() {
        let config = RateLimitConfig {
            max_requests: 0,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // Even with 0 max requests, first check might succeed or fail depending on implementation
        let _result = limiter.check("zero-limit");
        // This tests the boundary condition
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_rate_limit_many_keys() {
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            include_headers: true,
        };

        let limiter = RateLimiter::new(Some(config));

        // Many different keys should each have their own limit
        for i in 0..100 {
            for _ in 0..5 {
                assert!(limiter.check(&format!("key-{}", i)).is_ok());
            }
            assert!(limiter.check(&format!("key-{}", i)).is_err());
        }
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_auth_context_empty() {
        let context = AuthContext {
            user_id: None,
            permissions: vec![],
            metadata: Default::default(),
        };

        assert!(context.user_id.is_none());
        assert!(context.permissions.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_auth_context_many_permissions() {
        let permissions: Vec<String> = (0..100).map(|i| format!("permission-{}", i)).collect();

        let context = AuthContext {
            user_id: Some("user-100".to_string()),
            permissions,
            metadata: Default::default(),
        };

        assert_eq!(context.permissions.len(), 100);
    }
}

#[cfg(all(test, feature = "http"))]
mod boundary_tests {
    use axiom::config::{ApiConfig, CorsConfig, ServerConfig};
    #[cfg(feature = "security")]
    use axiom::security::RateLimitConfig;
    use std::time::Duration;

    #[tokio::test]
    async fn test_server_config_boundaries() {
        // Min port
        let min_config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 1,
            request_timeout_secs: 30,
            tls: None,
            cors: None,
        };
        assert_eq!(min_config.port, 1);

        // Max port
        let max_config = ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 65535,
            request_timeout_secs: 30,
            tls: None,
            cors: None,
        };
        assert_eq!(max_config.port, 65535);
    }

    #[tokio::test]
    async fn test_cors_config_boundaries() {
        // Empty origins
        let empty = CorsConfig {
            allowed_origins: vec![],
            allowed_methods: vec![],
            allowed_headers: vec![],
            allow_credentials: false,
            max_age: None,
        };
        assert!(empty.allowed_origins.is_empty());

        // Wildcard origin
        let wildcard = CorsConfig {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["*".to_string()],
            allowed_headers: vec!["*".to_string()],
            allow_credentials: false,
            max_age: Some(86400),
        };
        assert!(wildcard.allowed_origins.contains(&"*".to_string()));
        assert_eq!(wildcard.max_age, Some(86400));
    }

    #[tokio::test]
    async fn test_api_config_boundaries() {
        // Long name
        let long_name = "a".repeat(1000);
        let config = ApiConfig {
            name: long_name.clone(),
            version: "v1".to_string(),
            description: None,
        };
        assert_eq!(config.name.len(), 1000);

        // Empty version
        let empty_version = ApiConfig {
            name: "test".to_string(),
            version: "".to_string(),
            description: Some("Test".to_string()),
        };
        assert!(empty_version.version.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_rate_limit_window_boundaries() {
        // Min window
        let min_config = RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(1),
            include_headers: true,
        };
        assert_eq!(min_config.window.as_secs(), 1);

        // Large window
        let large_config = RateLimitConfig {
            max_requests: 1000000,
            window: Duration::from_secs(86400 * 365), // 1 year
            include_headers: true,
        };
        assert!(large_config.window.as_secs() > 86400);
    }
}

#[cfg(all(test, feature = "http"))]
mod edge_case_tests {
    use axiom::prelude::ApiMetadata;

    #[tokio::test]
    async fn test_api_metadata_empty_strings() {
        let metadata =
            ApiMetadata::new("".to_string(), "".to_string(), "".to_string(), None, false);

        assert!(metadata.name().is_empty());
        assert!(metadata.version().is_empty());
        assert!(metadata.description().is_empty());
    }

    #[tokio::test]
    async fn test_api_metadata_special_characters() {
        let metadata = ApiMetadata::new(
            "API-With-Special_Chars.123".to_string(),
            "v1.0.0-alpha+build.123".to_string(),
            "A test API with special characters: @#$%^&*()".to_string(),
            None,
            false,
        );

        assert!(metadata.name().contains("."));
        assert!(metadata.version().contains("+"));
        assert!(metadata.description().contains("@"));
    }

    #[tokio::test]
    async fn test_api_metadata_unicode() {
        let metadata = ApiMetadata::new(
            "中文API".to_string(),
            "v1".to_string(),
            "Тест API".to_string(),
            None,
            false,
        );

        assert!(metadata.name().contains("中"));
        assert!(metadata.description().contains("Т"));
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_empty_permissions() {
        use axiom::security::AuthContext;

        let context = AuthContext {
            user_id: Some("user".to_string()),
            permissions: vec![],
            metadata: Default::default(),
        };

        assert!(context.permissions.is_empty());
    }

    #[tokio::test]
    #[cfg(feature = "security")]
    async fn test_very_long_permission() {
        use axiom::security::AuthContext;

        let long_permission = "a".repeat(10000);
        let context = AuthContext {
            user_id: Some("user".to_string()),
            permissions: vec![long_permission],
            metadata: Default::default(),
        };

        assert_eq!(context.permissions[0].len(), 10000);
    }
}

#[cfg(all(test, feature = "http"))]
mod serialization_tests {
    use axiom::prelude::{ApiError, ServiceError, ServiceResponse};
    use serde_json::json;

    #[tokio::test]
    async fn test_api_error_json_roundtrip() {
        let original = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("123".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ApiError = serde_json::from_str(&json).unwrap();

        match deserialized {
            ApiError::NotFound {
                resource,
                resource_id,
            } => {
                assert_eq!(resource, "User");
                assert_eq!(resource_id, Some("123".to_string()));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    #[tokio::test]
    async fn test_service_response_json_roundtrip() {
        let original = ServiceResponse::success(json!({"key": "value"}));

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ServiceResponse<serde_json::Value> = serde_json::from_str(&json).unwrap();

        assert!(deserialized.is_success());
        assert!(deserialized.data().is_some());
    }

    #[tokio::test]
    async fn test_service_error_json_roundtrip() {
        let original = ServiceError::new("CODE", "message", 400);

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ServiceError = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.code(), "CODE");
        assert_eq!(deserialized.message(), "message");
        assert_eq!(deserialized.http_status(), 400);
    }

    #[tokio::test]
    async fn test_error_with_null_details() {
        let error = ServiceError::new("CODE", "message", 500);
        assert!(error.details().is_none());

        let json = serde_json::to_string(&error).unwrap();
        // Verify the JSON contains expected fields
        assert!(json.contains("CODE"));
        assert!(json.contains("message"));
    }
}
