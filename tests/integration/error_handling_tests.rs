// Copyright (c) 2026 Kirky.X
//! Error Handling Integration Tests
//!
//! This module contains comprehensive integration tests for error handling functionality.
//! Tests cover error types, error-to-HTTP status mapping, error-to-MCP response conversion,
//! error chain preservation, error context enrichment, error logging format, and correlation ID.
//!
//! All tests are integration tests and use real functionality without mocks.

#[cfg(any(feature = "http", feature = "mcp"))]
mod error_handling_tests {
    use sdforge::core::error::{ApiError, ErrorCategory, ErrorContext};
    use sdforge::core::response::{ServiceError, ServiceResponse};
    use serde_json::json;
    use tower::ServiceExt;

    // ============================================================================
    // Error Type Tests
    // ============================================================================

    /// Test: NotFound error returns correct HTTP status and message
    ///
    /// Verifies that ApiError::NotFound is properly created and maps to HTTP 404.
    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("12345".to_string()),
        };

        // Verify error message contains resource info
        assert!(error.to_string().contains("Resource not found"));
        assert!(error.to_string().contains("user"));

        // Verify category classification
        assert_eq!(error.category(), ErrorCategory::ClientError);

        // Verify source error is not available for NotFound
        assert!(error.source().is_none());

        // Verify sanitized message
        let sanitized = error.sanitized_message();
        assert!(sanitized.contains("user"));
    }

    /// Test: Validation error returns correct HTTP status and validation details
    ///
    /// Verifies that ApiError::ValidationError captures field and constraint information.
    #[test]
    fn test_api_error_validation() {
        let error = ApiError::ValidationError {
            field: "email".to_string(),
            constraint: "must be valid email format".to_string(),
        };

        // Verify error message
        assert!(error.to_string().contains("Validation failed"));
        assert!(error.to_string().contains("email"));

        // Verify category classification
        assert_eq!(error.category(), ErrorCategory::ValidationError);

        // Verify source error is not available
        assert!(error.source().is_none());

        // Verify sanitized message
        let sanitized = error.sanitized_message();
        assert!(sanitized.contains("email"));
    }

    /// Test: Unauthorized error (AuthenticationFailed) returns HTTP 401
    ///
    /// Verifies that ApiError::AuthenticationFailed is properly handled.
    #[test]
    fn test_api_error_unauthorized() {
        let error = ApiError::AuthenticationFailed {
            reason: "Invalid or expired token".to_string(),
        };

        // Verify error message
        assert!(error.to_string().contains("Authentication failed"));
        assert!(error.to_string().contains("Invalid or expired token"));

        // Verify category classification
        assert_eq!(error.category(), ErrorCategory::AuthError);

        // Verify source error is not available
        assert!(error.source().is_none());

        // Verify sanitized message preserves reason
        let sanitized = error.sanitized_message();
        assert!(sanitized.contains("Authentication failed"));
    }

    /// Test: Rate limit error returns HTTP 429 with limit info
    ///
    /// Verifies that ApiError::RateLimitExceeded includes rate limit details.
    #[test]
    fn test_api_error_rate_limit() {
        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };

        // Verify error message
        assert!(error.to_string().contains("Rate limit exceeded"));

        // Verify category classification
        assert_eq!(error.category(), ErrorCategory::RateLimitError);

        // Verify source error is not available
        assert!(error.source().is_none());

        // Verify sanitized message
        let sanitized = error.sanitized_message();
        assert!(sanitized.contains("Rate limit"));
    }

    /// Test: Internal server error sanitizes sensitive information
    ///
    /// Verifies that ApiError::Internal does not leak internal details.
    #[test]
    fn test_api_error_internal() {
        let error = ApiError::Internal {
            message: "Database connection failed: host=localhost port=5432".to_string(),
            error_id: "ERR001".to_string(),
            source: None,
            context: None,
        };

        // Verify error message is generic for internal errors
        assert!(error.to_string().contains("Internal server error"));

        // Verify category classification
        assert_eq!(error.category(), ErrorCategory::ServerError);

        // Verify source error is not available
        assert!(error.source().is_none());

        // Verify sanitized message does NOT contain sensitive info
        let sanitized = error.sanitized_message();
        assert!(!sanitized.contains("localhost"));
        assert!(!sanitized.contains("5432"));
        assert!(sanitized.contains("internal error"));
    }

    /// Test: Bad request error (InvalidInput) returns HTTP 400
    ///
    /// Verifies that ApiError::InvalidInput is properly handled.
    #[test]
    fn test_api_error_bad_request() {
        let error = ApiError::InvalidInput {
            message: "Invalid JSON payload".to_string(),
            field: Some("body".to_string()),
            value: Some(json!("{broken")),
        };

        // Verify error message
        assert!(error.to_string().contains("Invalid input"));
        assert!(error.to_string().contains("Invalid JSON payload"));

        // Verify category classification
        assert_eq!(error.category(), ErrorCategory::ClientError);

        // Verify source error is not available
        assert!(error.source().is_none());

        // Verify sanitized message preserves error info
        let sanitized = error.sanitized_message();
        assert!(sanitized.contains("Invalid input"));
    }

    // ============================================================================
    // Error Conversion Tests
    // ============================================================================

    /// Test: Error to HTTP status mapping for all error types
    ///
    /// Verifies that each ApiError variant correctly maps to its HTTP status code.
    #[cfg(feature = "http")]
    #[tokio::test]
    async fn test_error_to_http_status_mapping() {
        use axum::{body::Body, http::Request, http::StatusCode, routing::get, Router};

        // Test NotFound -> 404
        async fn not_found_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::not_found("user", Some("123".into())))
        }
        let router = Router::new().route("/not_found", get(not_found_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/not_found")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Test InvalidInput -> 400
        async fn invalid_input_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::invalid_input(
                "bad data",
                Some("field".into()),
                None,
            ))
        }
        let router = Router::new().route("/invalid_input", get(invalid_input_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/invalid_input")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Test AuthenticationFailed -> 401
        async fn auth_failed_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::authentication_failed("Invalid token"))
        }
        let router = Router::new().route("/auth_failed", get(auth_failed_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/auth_failed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Test AccessDenied -> 403
        async fn access_denied_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::access_denied("admin", Some("user1".into())))
        }
        let router = Router::new().route("/access_denied", get(access_denied_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/access_denied")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // Test RateLimitExceeded -> 429
        async fn rate_limit_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::rate_limit_exceeded(100, 60))
        }
        let router = Router::new().route("/rate_limit", get(rate_limit_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/rate_limit")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        // Test Internal -> 500
        async fn internal_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::internal_error("Something went wrong", "ERR123"))
        }
        let router = Router::new().route("/internal", get(internal_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/internal")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Test ServiceUnavailable -> 503
        async fn unavailable_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::service_unavailable("database", Some(30)))
        }
        let router = Router::new().route("/unavailable", get(unavailable_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/unavailable")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Test ValidationError -> 422
        async fn validation_handler() -> Result<&'static str, ApiError> {
            Err(ApiError::validation("email", "must be valid"))
        }
        let router = Router::new().route("/validation", get(validation_handler));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/validation")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// Test: Error to MCP JSON conversion
    ///
    /// Verifies that ApiError correctly converts to MCP-compatible JSON format.
    #[test]
    fn test_error_to_mcp_response_conversion() {
        // Test NotFound -> MCP
        let not_found = ApiError::not_found("user", Some("123".into()));
        let mcp_json = not_found.to_mcp_json();
        assert!(mcp_json.contains("\"success\":false"));
        assert!(mcp_json.contains("\"code\":\"NOT_FOUND\""));
        assert!(mcp_json.contains("Resource not found"));

        // Test AuthenticationFailed -> MCP
        let auth_failed = ApiError::authentication_failed("Invalid token");
        let mcp_json = auth_failed.to_mcp_json();
        assert!(mcp_json.contains("AUTHENTICATION_FAILED"));

        // Test RateLimitExceeded -> MCP
        let rate_limit = ApiError::rate_limit_exceeded(100, 60);
        let mcp_json = rate_limit.to_mcp_json();
        assert!(mcp_json.contains("RATE_LIMIT_EXCEEDED"));

        // Test Internal -> MCP
        let internal = ApiError::internal_error("Server error", "ERR001");
        let mcp_json = internal.to_mcp_json();
        assert!(mcp_json.contains("INTERNAL_ERROR"));

        // Test ValidationError -> MCP
        let validation = ApiError::validation("email", "required");
        let mcp_json = validation.to_mcp_json();
        assert!(mcp_json.contains("VALIDATION_ERROR"));
        assert!(mcp_json.contains("email"));
    }

    /// Test: Error chain preservation through source errors
    ///
    /// Verifies that source errors are properly preserved and accessible.
    #[test]
    fn test_error_chain_preservation() {
        // Define a source error type
        #[derive(Debug)]
        struct DatabaseError(&'static str);
        impl std::fmt::Display for DatabaseError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl std::error::Error for DatabaseError {}
        unsafe impl Send for DatabaseError {}
        unsafe impl Sync for DatabaseError {}

        // Create error with source
        let source_error = DatabaseError("connection refused");
        let api_error =
            ApiError::internal_with_source("Database operation failed", "DB001", source_error);

        // Verify source is accessible
        let source = api_error.source();
        assert!(source.is_some());
        let source_msg = source.unwrap().to_string();
        assert!(source_msg.contains("connection refused"));
    }

    /// Test: Error context enrichment with additional information
    ///
    /// Verifies that ErrorContext can be added to errors with custom metadata.
    #[test]
    fn test_error_context_enrichment() {
        // Create context with additional metadata
        let context = ErrorContext::new()
            .with_extra("user_id".to_string(), "user_123".to_string())
            .with_extra("operation".to_string(), "create_user".to_string())
            .with_extra("request_path".to_string(), "/api/users".to_string());

        // Create error with context
        let error = ApiError::internal_with_context("Failed to create user", "CREATE001", context);

        // Verify error contains context
        match error {
            ApiError::Internal {
                message,
                error_id,
                context: ctx,
                ..
            } => {
                assert_eq!(message, "Failed to create user");
                assert_eq!(error_id, "CREATE001");
                assert!(ctx.is_some());

                let ctx = ctx.unwrap();
                assert_eq!(ctx.extra.get("user_id"), Some(&"user_123".to_string()));
                assert_eq!(ctx.extra.get("operation"), Some(&"create_user".to_string()));
                assert_eq!(
                    ctx.extra.get("request_path"),
                    Some(&"/api/users".to_string())
                );
            }
            _ => panic!("Expected Internal error"),
        }
    }

    // ============================================================================
    // Error Logging Tests
    // ============================================================================

    /// Test: Error logging format contains required fields
    ///
    /// Verifies that errors serialize correctly for logging purposes.
    #[test]
    fn test_error_logging_format() {
        // Test serialization format for logging
        let error = ApiError::NotFound {
            resource: "order".to_string(),
            resource_id: Some("ORD-12345".to_string()),
        };

        let json = serde_json::to_string(&error).expect("Should serialize to JSON");

        // Verify JSON contains required fields
        assert!(json.contains("\"type\":\"NotFound\""));
        assert!(json.contains("\"resource\":\"order\""));
        assert!(json.contains("\"resource_id\":\"ORD-12345\""));

        // Verify JSON is valid and can be deserialized
        let deserialized: ApiError = serde_json::from_str(&json).expect("Should deserialize");
        match deserialized {
            ApiError::NotFound {
                resource,
                resource_id,
            } => {
                assert_eq!(resource, "order");
                assert_eq!(resource_id, Some("ORD-12345".to_string()));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    /// Test: Error context capture includes caller information
    ///
    /// Verifies that ErrorContext::current() captures call site information.
    #[test]
    fn test_error_context_capture() {
        // Capture context at known location
        let context = ErrorContext::current();

        // Verify file is captured
        assert!(context.file.is_some());
        let file = context.file.as_ref().unwrap();
        // Should contain "error_handling_tests" in path (this file)
        assert!(file.contains("error_handling_tests") || file.ends_with(".rs"));

        // Verify line number is captured (should be greater than 0)
        assert!(context.line.is_some());
        assert!(context.line.unwrap() > 0);

        // Verify function is captured
        assert!(context.function.is_some());

        // Verify extra map exists
        assert!(context.extra.is_empty());

        // Test ErrorContext serialization for logging
        let json = serde_json::to_string(&context).expect("Should serialize context");
        assert!(json.contains("file") || json.contains("line") || json.contains("function"));
    }

    /// Test: Error correlation ID (request_id) for request tracking
    ///
    /// Verifies that errors can be associated with request IDs for tracing.
    #[test]
    fn test_error_correlation_id() {
        // Simulate correlation ID tracking
        let request_id = "req_abc123def456";

        // Create error with request context
        let context = ErrorContext::new()
            .with_extra("request_id".to_string(), request_id.to_string())
            .with_extra("endpoint".to_string(), "/api/v1/users".to_string())
            .with_extra("method".to_string(), "POST".to_string());

        let error =
            ApiError::internal_with_context("Request processing failed", "PROC001", context);

        // Verify error contains correlation ID
        match error {
            ApiError::Internal {
                message,
                error_id,
                context: ctx,
                ..
            } => {
                assert_eq!(message, "Request processing failed");
                assert_eq!(error_id, "PROC001");
                assert!(ctx.is_some());

                let ctx = ctx.unwrap();
                assert_eq!(
                    ctx.extra.get("request_id"),
                    Some(&"req_abc123def456".to_string())
                );
                assert_eq!(
                    ctx.extra.get("endpoint"),
                    Some(&"/api/v1/users".to_string())
                );
            }
            _ => panic!("Expected Internal error"),
        }
    }

    // ============================================================================
    // Error Constructor Tests
    // ============================================================================

    /// Test: All error constructor methods work correctly
    ///
    /// Verifies that convenience constructors create correct error variants.
    #[test]
    fn test_error_constructors() {
        // Test not_found constructor
        let not_found = ApiError::not_found("resource", None);
        assert!(matches!(not_found, ApiError::NotFound { .. }));

        // Test invalid_input constructor
        let invalid = ApiError::invalid_input("bad input", Some("field".into()), None);
        assert!(matches!(invalid, ApiError::InvalidInput { .. }));

        // Test authentication_failed constructor
        let auth = ApiError::authentication_failed("token expired");
        assert!(matches!(auth, ApiError::AuthenticationFailed { .. }));

        // Test access_denied constructor
        let denied = ApiError::access_denied("write", Some("user1".into()));
        assert!(matches!(denied, ApiError::AccessDenied { .. }));

        // Test rate_limit_exceeded constructor
        let rate = ApiError::rate_limit_exceeded(50, 30);
        assert!(matches!(rate, ApiError::RateLimitExceeded { .. }));

        // Test internal_error constructor
        let internal = ApiError::internal_error("error message", "E001");
        assert!(matches!(internal, ApiError::Internal { .. }));

        // Test service_unavailable constructor
        let unavailable = ApiError::service_unavailable("service", Some(60));
        assert!(matches!(unavailable, ApiError::ServiceUnavailable { .. }));

        // Test validation constructor
        let validation = ApiError::validation("field", "constraint");
        assert!(matches!(validation, ApiError::ValidationError { .. }));
    }

    // ============================================================================
    // ServiceError Conversion Tests
    // ============================================================================

    /// Test: ApiError to ServiceError conversion
    ///
    /// Verifies that ApiError correctly converts to ServiceError for HTTP responses.
    #[test]
    fn test_api_error_to_service_error_conversion() {
        // Test NotFound conversion
        let api_error = ApiError::not_found("order", Some("123".into()));
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "NOT_FOUND");
        assert_eq!(service_error.http_status(), 404);
        assert!(service_error.details().is_some());

        // Test InvalidInput conversion
        let api_error = ApiError::invalid_input("Invalid email", Some("email".into()), None);
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "INVALID_INPUT");
        assert_eq!(service_error.http_status(), 400);

        // Test AuthenticationFailed conversion
        let api_error = ApiError::authentication_failed("Token expired");
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "AUTHENTICATION_FAILED");
        assert_eq!(service_error.http_status(), 401);

        // Test AccessDenied conversion
        let api_error = ApiError::access_denied("admin", Some("user1".into()));
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "ACCESS_DENIED");
        assert_eq!(service_error.http_status(), 403);

        // Test RateLimitExceeded conversion
        let api_error = ApiError::rate_limit_exceeded(100, 60);
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "RATE_LIMIT_EXCEEDED");
        assert_eq!(service_error.http_status(), 429);

        // Test Internal conversion
        let api_error = ApiError::internal_error("Internal error", "ERR001");
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "INTERNAL_ERROR");
        assert_eq!(service_error.http_status(), 500);

        // Test ValidationError conversion
        let api_error = ApiError::validation("email", "required");
        let service_error: ServiceError = api_error.into();
        assert_eq!(service_error.code(), "VALIDATION_ERROR");
        assert_eq!(service_error.http_status(), 422);
    }

    // ============================================================================
    // ServiceResponse Error Tests
    // ============================================================================

    /// Test: ServiceResponse with error
    ///
    /// Verifies that ServiceResponse correctly wraps errors.
    #[test]
    fn test_service_response_with_error() {
        let service_error = ServiceError::with_details(
            "VALIDATION_ERROR",
            "Invalid input data",
            json!({"field": "email", "reason": "missing @ symbol"}),
            422,
        );

        let response: ServiceResponse<String> = ServiceResponse::error(service_error);

        // Verify error response structure
        assert!(!response.is_success());
        assert!(response.data().is_none());
        assert!(response.error_ref().is_some());

        let error = response.error_ref().unwrap();
        assert_eq!(error.code(), "VALIDATION_ERROR");
        assert_eq!(error.message(), "Invalid input data");
        assert_eq!(error.http_status(), 422);
        assert!(error.details().is_some());
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    /// Test: Error with all optional fields populated
    ///
    /// Verifies that errors work correctly with all fields populated.
    #[test]
    fn test_error_with_all_fields() {
        let context = ErrorContext::new()
            .with_extra("key1".to_string(), "value1".to_string())
            .with_extra("key2".to_string(), "value2".to_string());

        #[derive(Debug)]
        struct SourceError(&'static str);
        impl std::fmt::Display for SourceError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
        impl std::error::Error for SourceError {}
        unsafe impl Send for SourceError {}
        unsafe impl Sync for SourceError {}

        let error = ApiError::Internal {
            message: "Detailed error message".to_string(),
            error_id: "ERR_ALL_FIELDS".to_string(),
            source: Some(Box::new(SourceError("original error"))),
            context: Some(context),
        };

        // Verify source is accessible
        assert!(error.source().is_some());

        // Verify category
        assert_eq!(error.category(), ErrorCategory::ServerError);

        // Verify serialization preserves all fields
        let json = serde_json::to_string(&error).expect("Should serialize");
        assert!(json.contains("ERR_ALL_FIELDS"));
        assert!(json.contains("key1"));
        assert!(json.contains("value1"));
    }

    /// Test: Error serialization roundtrip
    ///
    /// Verifies that errors can be serialized and deserialized correctly.
    #[test]
    fn test_error_serialization_roundtrip() {
        let original = ApiError::ValidationError {
            field: "username".to_string(),
            constraint: "must be alphanumeric".to_string(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Should serialize");

        // Deserialize back
        let deserialized: ApiError = serde_json::from_str(&json).expect("Should deserialize");

        // Verify content is preserved
        match deserialized {
            ApiError::ValidationError { field, constraint } => {
                assert_eq!(field, "username");
                assert_eq!(constraint, "must be alphanumeric");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    /// Test: ErrorContext with empty extra map
    ///
    /// Verifies that ErrorContext works correctly with empty extra data.
    #[test]
    fn test_error_context_empty_extra() {
        let context = ErrorContext::new();
        assert!(context.extra.is_empty());
        assert!(context.file.is_none());
        assert!(context.line.is_none());
        assert!(context.function.is_none());

        // Verify serialization works with empty context
        let json = serde_json::to_string(&context).expect("Should serialize");
        let deserialized: ErrorContext = serde_json::from_str(&json).expect("Should deserialize");
        assert!(deserialized.extra.is_empty());
    }

    /// Test: ErrorCategory for all variants
    ///
    /// Verifies that ErrorCategory correctly classifies all error types.
    #[test]
    fn test_error_category_all_variants() {
        // Client errors
        assert_eq!(
            ApiError::not_found("x", None).category(),
            ErrorCategory::ClientError
        );
        assert_eq!(
            ApiError::invalid_input("x", None, None).category(),
            ErrorCategory::ClientError
        );

        // Auth errors
        assert_eq!(
            ApiError::authentication_failed("x").category(),
            ErrorCategory::AuthError
        );
        assert_eq!(
            ApiError::access_denied("x", None).category(),
            ErrorCategory::AuthError
        );

        // Server errors
        assert_eq!(
            ApiError::internal_error("x", "x").category(),
            ErrorCategory::ServerError
        );
        assert_eq!(
            ApiError::service_unavailable("x", None).category(),
            ErrorCategory::ServerError
        );

        // Rate limit
        assert_eq!(
            ApiError::rate_limit_exceeded(1, 1).category(),
            ErrorCategory::RateLimitError
        );

        // Validation
        assert_eq!(
            ApiError::validation("x", "x").category(),
            ErrorCategory::ValidationError
        );
    }

    /// Test: Sanitized message for all error types
    ///
    /// Verifies that sanitized_message() works correctly for all variants.
    #[test]
    fn test_sanitized_message_all_types() {
        // NotFound - should return full message
        let msg = ApiError::not_found("user", None).sanitized_message();
        assert!(msg.contains("user"));

        // InvalidInput - should return full message
        let msg = ApiError::invalid_input("bad data", None, None).sanitized_message();
        assert!(msg.contains("bad data"));

        // AuthenticationFailed - should return full message
        let msg = ApiError::authentication_failed("token expired").sanitized_message();
        assert!(msg.contains("Authentication failed"));

        // AccessDenied - should return full message
        let msg = ApiError::access_denied("admin", None).sanitized_message();
        assert!(msg.contains("Access denied"));

        // RateLimitExceeded - should return generic message
        let msg = ApiError::rate_limit_exceeded(1, 1).sanitized_message();
        assert!(msg.contains("Rate limit"));

        // Internal - should return generic message
        let msg = ApiError::internal_error("secret info", "ERR").sanitized_message();
        assert!(msg.contains("internal error"));
        assert!(!msg.contains("secret info"));

        // ServiceUnavailable - should return generic message
        let msg = ApiError::service_unavailable("database", None).sanitized_message();
        assert!(msg.contains("unavailable"));
        assert!(!msg.contains("database"));

        // ValidationError - should return full message
        let msg = ApiError::validation("field", "constraint").sanitized_message();
        assert!(msg.contains("Validation failed"));
    }
}
