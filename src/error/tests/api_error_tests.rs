// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::core::response::ServiceError;
use crate::error::*;
use std::error::Error as StdError;

/// Test ApiError::NotFound variant
#[test]
fn test_api_error_not_found() {
    let error = ApiError::NotFound {
        resource: "user".to_string(),
        resource_id: Some("123".to_string()),
    };
    assert!(error.to_string().contains("Resource not found"));
    assert!(error.to_string().contains("user"));
    assert_eq!(error.category(), ErrorCategory::ClientError);
}

/// Test ApiError::InvalidInput variant
#[test]
fn test_api_error_invalid_input() {
    let error = ApiError::InvalidInput {
        message: "Invalid email format".to_string(),
        field: Some("email".to_string()),
        value: Some(serde_json::json!("invalid@")),
    };
    assert!(error.to_string().contains("Invalid input"));
    assert_eq!(error.category(), ErrorCategory::ClientError);
}

/// Test ApiError::AuthenticationFailed variant
#[test]
fn test_api_error_authentication_failed() {
    let error = ApiError::AuthenticationFailed {
        reason: "Invalid token".to_string(),
    };
    assert!(error.to_string().contains("Authentication failed"));
    assert!(error.to_string().contains("Invalid token"));
    assert_eq!(error.category(), ErrorCategory::AuthError);
}

/// Test ApiError::AccessDenied variant
#[test]
fn test_api_error_access_denied() {
    let error = ApiError::AccessDenied {
        permission: "admin.write".to_string(),
        user_id: Some("user123".to_string()),
    };
    assert!(error.to_string().contains("Access denied"));
    assert!(error.to_string().contains("admin.write"));
    assert_eq!(error.category(), ErrorCategory::AuthError);
}

/// Test ApiError::RateLimitExceeded variant
#[test]
fn test_api_error_rate_limit_exceeded() {
    let error = ApiError::RateLimitExceeded {
        limit: 100,
        window_seconds: 60,
    };
    assert!(error.to_string().contains("Rate limit exceeded"));
    assert_eq!(error.category(), ErrorCategory::RateLimitError);
}

/// Test ApiError::Internal variant
#[test]
fn test_api_error_internal() {
    let error = ApiError::Internal {
        message: "Database connection failed".to_string(),
        error_id: "abc123".to_string(),
        source: None,
        context: None,
    };
    assert!(error.to_string().contains("Internal server error"));
    // Message should be sanitized (internal details not leaked)
    assert_eq!(error.category(), ErrorCategory::ServerError);
}

/// Test ApiError::ServiceUnavailable variant
#[test]
fn test_api_error_service_unavailable() {
    let error = ApiError::ServiceUnavailable {
        service: "external_service".to_string(),
        retry_after: Some(30),
        source: None,
    };
    assert!(error.to_string().contains("Service unavailable"));
    assert!(error.to_string().contains("external_service"));
    assert_eq!(error.category(), ErrorCategory::ServerError);
}

/// Test ApiError::ValidationError variant
#[test]
fn test_api_error_validation() {
    let error = ApiError::ValidationError {
        field: "email".to_string(),
        constraint: "must be valid email".to_string(),
    };
    assert!(error.to_string().contains("Validation failed"));
    assert!(error.to_string().contains("email"));
    assert_eq!(error.category(), ErrorCategory::ValidationError);
}

/// Test ErrorCategory for all error types
#[test]
fn test_error_category_all_variants() {
    let client_errors = vec![
        ApiError::NotFound {
            resource: "x".into(),
            resource_id: None,
        },
        ApiError::InvalidInput {
            message: "x".into(),
            field: None,
            value: None,
        },
    ];
    for err in client_errors {
        assert_eq!(err.category(), ErrorCategory::ClientError);
    }

    let auth_errors = vec![
        ApiError::AuthenticationFailed { reason: "x".into() },
        ApiError::AccessDenied {
            permission: "x".into(),
            user_id: None,
        },
    ];
    for err in auth_errors {
        assert_eq!(err.category(), ErrorCategory::AuthError);
    }

    let server_errors = vec![
        ApiError::Internal {
            message: "x".into(),
            error_id: "x".into(),
            source: None,
            context: None,
        },
        ApiError::ServiceUnavailable {
            service: "x".into(),
            retry_after: None,
            source: None,
        },
    ];
    for err in server_errors {
        assert_eq!(err.category(), ErrorCategory::ServerError);
    }

    assert_eq!(
        ApiError::RateLimitExceeded {
            limit: 0,
            window_seconds: 0
        }
        .category(),
        ErrorCategory::RateLimitError
    );
    assert_eq!(
        ApiError::ValidationError {
            field: "x".into(),
            constraint: "x".into()
        }
        .category(),
        ErrorCategory::ValidationError
    );
}

/// Test ErrorCategory serialization
#[test]
fn test_error_category_serialization() {
    let categories = vec![
        ErrorCategory::ClientError,
        ErrorCategory::AuthError,
        ErrorCategory::ServerError,
        ErrorCategory::RateLimitError,
        ErrorCategory::ValidationError,
    ];
    for cat in categories {
        let json = serde_json::to_string(&cat).unwrap();
        let deserialized: ErrorCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(cat, deserialized);
    }
}

/// Test ApiError::validation_error constructor
#[test]
fn test_validation_error_constructor() {
    let error = ApiError::validation_error("VALIDATION_001", "Invalid input");
    match error {
        ApiError::InvalidInput {
            message,
            field,
            value,
        } => {
            assert_eq!(message, "Invalid input");
            assert!(field.is_none());
            assert!(value.is_none());
        }
        _ => unreachable!("Unexpected variant in ApiError::InvalidInput test"),
    }
}

/// Test to_mcp_json for all variants
#[test]
fn test_to_mcp_json() {
    let not_found = ApiError::NotFound {
        resource: "test".to_string(),
        resource_id: None,
    };
    let json = not_found.to_mcp_json();
    assert!(json.contains("NOT_FOUND"));
    assert!(json.contains("success\":false"));

    let auth_failed = ApiError::AuthenticationFailed {
        reason: "bad token".to_string(),
    };
    let json = auth_failed.to_mcp_json();
    assert!(json.contains("AUTHENTICATION_FAILED"));

    let validation = ApiError::ValidationError {
        field: "name".to_string(),
        constraint: "required".to_string(),
    };
    let json = validation.to_mcp_json();
    assert!(json.contains("VALIDATION_ERROR"));
}

/// Test ApiError serialization
#[test]
fn test_api_error_serialization() {
    let error = ApiError::NotFound {
        resource: "file".to_string(),
        resource_id: Some("123".to_string()),
    };
    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("\"type\":\"NotFound\""));
    assert!(json.contains("\"resource\":\"file\""));
}

/// Test ApiError deserialization
#[test]
fn test_api_error_deserialization() {
    let json = r#"{"type":"NotFound","resource":"user","resource_id":"456"}"#;
    let error: ApiError = serde_json::from_str(json).unwrap();

    assert!(
        matches!(error, ApiError::NotFound { ref resource, resource_id: Some(ref id) }
            if resource == "user" && id == "456"),
        "Expected NotFound variant with correct values, got {:?}",
        error
    );
}

/// Test ApiError constructor methods
#[test]
fn test_api_error_constructors() {
    let not_found = ApiError::not_found("user", Some("123".into()));
    assert!(matches!(not_found, ApiError::NotFound { .. }));

    let invalid = ApiError::invalid_input("bad data", Some("email".into()), None);
    assert!(matches!(invalid, ApiError::InvalidInput { .. }));

    let auth_failed = ApiError::authentication_failed("Invalid token");
    assert!(matches!(auth_failed, ApiError::AuthenticationFailed { .. }));

    let access_denied = ApiError::access_denied("admin", Some("user1".into()));
    assert!(matches!(access_denied, ApiError::AccessDenied { .. }));

    let rate_limit = ApiError::rate_limit_exceeded(100, 60);
    assert!(matches!(rate_limit, ApiError::RateLimitExceeded { .. }));

    let internal = ApiError::internal_error("Something went wrong", "ERR123");
    assert!(matches!(internal, ApiError::Internal { .. }));

    let unavailable = ApiError::service_unavailable("database", Some(30));
    assert!(matches!(unavailable, ApiError::ServiceUnavailable { .. }));

    let validation = ApiError::validation("email", "must be valid");
    assert!(matches!(validation, ApiError::ValidationError { .. }));
}

/// Test sanitized_message for internal errors
#[test]
fn test_sanitized_message() {
    let internal = ApiError::internal_error(
        "Database connection failed: host=localhost port=5432",
        "ERR123",
    );
    let msg = internal.sanitized_message();
    assert!(msg.contains("internal error"));
    assert!(!msg.contains("localhost"));
    assert!(!msg.contains("5432"));

    let unavailable = ApiError::service_unavailable("Database connection failed", None);
    let msg = unavailable.sanitized_message();
    assert!(msg.contains("temporarily unavailable"));
    assert!(!msg.contains("Database"));

    let not_found = ApiError::not_found("user", Some("123".into()));
    let msg = not_found.sanitized_message();
    assert!(msg.contains("user"));

    let auth_failed = ApiError::authentication_failed("Invalid token");
    let msg = auth_failed.sanitized_message();
    assert!(msg.contains("Authentication failed"));
}

/// Test source() returns None for errors without source
#[test]
fn test_source_returns_none_for_errors_without_source() {
    let errors = vec![
        ApiError::not_found("test", None),
        ApiError::invalid_input("test", None, None),
        ApiError::authentication_failed("test"),
        ApiError::access_denied("test", None),
        ApiError::rate_limit_exceeded(1, 1),
        ApiError::internal_error("test", "test"),
        ApiError::service_unavailable("test", None),
        ApiError::validation("test", "test"),
    ];
    for err in errors {
        assert!(err.source().is_none());
    }
}

/// Test source() returns Some for errors with source
#[test]
fn test_source_returns_some_for_errors_with_source() {
    // Create a test error
    #[derive(Debug)]
    struct TestError(&'static str);
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl StdError for TestError {}
    unsafe impl Send for TestError {}
    unsafe impl Sync for TestError {}

    // Test Internal error with source
    let internal =
        ApiError::internal_with_source("test message", "ERR001", TestError("test error"));
    assert!(internal.source().is_some());
    assert!(
        internal
            .source()
            .unwrap()
            .to_string()
            .contains("test error")
    );

    // Test ServiceUnavailable error with source
    let unavailable = ApiError::service_unavailable_with_source(
        "test service",
        Some(30),
        TestError("service error"),
    );
    assert!(unavailable.source().is_some());
    assert!(
        unavailable
            .source()
            .unwrap()
            .to_string()
            .contains("service error")
    );
}

/// Test ErrorCategory derives
#[test]
fn test_error_category_derives() {
    let cat = ErrorCategory::ClientError;
    let copied = cat;
    assert_eq!(ErrorCategory::ClientError, copied);
}

/// Test ErrorContext::new() creates empty context
#[test]
fn test_error_context_new() {
    let ctx = ErrorContext::new();
    assert!(ctx.file.is_none());
    assert!(ctx.line.is_none());
    assert!(ctx.function.is_none());
    assert!(ctx.extra.is_empty());
}

/// Test ErrorContext::current() captures caller information
#[test]
fn test_error_context_current() {
    let ctx = ErrorContext::current();
    assert!(ctx.file.is_some());
    assert!(ctx.file.unwrap().contains("error"));
    assert!(ctx.line.is_some());
    assert!(ctx.line.unwrap() > 0);
    assert!(ctx.function.is_some());
    assert!(ctx.extra.is_empty());
}

/// Test ErrorContext::with_extra() adds extra information
#[test]
fn test_error_context_with_extra() {
    let ctx = ErrorContext::new()
        .with_extra("user_id".to_string(), "12345".to_string())
        .with_extra("action".to_string(), "delete".to_string());

    assert_eq!(ctx.extra.len(), 2);
    assert_eq!(ctx.extra.get("user_id"), Some(&"12345".to_string()));
    assert_eq!(ctx.extra.get("action"), Some(&"delete".to_string()));
}

/// Test ErrorContext serialization
#[test]
fn test_error_context_serialization() {
    let mut extra = std::collections::HashMap::new();
    extra.insert("key1".to_string(), "value1".to_string());
    extra.insert("key2".to_string(), "value2".to_string());

    let ctx = ErrorContext {
        file: Some("test.rs".to_string()),
        line: Some(42),
        function: Some("test_function".to_string()),
        extra,
    };

    let json = serde_json::to_string(&ctx).unwrap();
    assert!(json.contains("test.rs"));
    assert!(json.contains("42"));
    assert!(json.contains("test_function"));
    assert!(json.contains("key1"));
    assert!(json.contains("value1"));

    // Test deserialization
    let deserialized: ErrorContext = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.file, Some("test.rs".to_string()));
    assert_eq!(deserialized.line, Some(42));
    assert_eq!(deserialized.function, Some("test_function".to_string()));
    assert_eq!(deserialized.extra.len(), 2);
}

/// Test internal_with_context() includes context
#[test]
fn test_internal_with_context() {
    let ctx =
        ErrorContext::current().with_extra("operation".to_string(), "database_query".to_string());

    let error = ApiError::internal_with_context("Database error", "DB001", ctx);

    match error {
        ApiError::Internal {
            message,
            error_id,
            context,
            ..
        } => {
            assert_eq!(message, "Database error");
            assert_eq!(error_id, "DB001");
            assert!(context.is_some());
            let ctx = context.unwrap();
            assert!(ctx.extra.contains_key("operation"));
            assert_eq!(
                ctx.extra.get("operation"),
                Some(&"database_query".to_string())
            );
        }
        _ => panic!("Expected Internal error"),
    }
}

/// Test internal_with_source_and_context() includes both
#[test]
fn test_internal_with_source_and_context() {
    #[derive(Debug)]
    struct TestError(&'static str);
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl StdError for TestError {}
    unsafe impl Send for TestError {}
    unsafe impl Sync for TestError {}

    let ctx = ErrorContext::current().with_extra("retry_count".to_string(), "3".to_string());

    let error = ApiError::internal_with_source_and_context(
        "Connection failed",
        "CONN001",
        TestError("connection timeout"),
        ctx,
    );

    match error {
        ApiError::Internal {
            message,
            error_id,
            source,
            context,
            ..
        } => {
            assert_eq!(message, "Connection failed");
            assert_eq!(error_id, "CONN001");
            assert!(source.is_some());
            assert!(context.is_some());

            let ctx = context.unwrap();
            assert_eq!(ctx.extra.get("retry_count"), Some(&"3".to_string()));
        }
        _ => panic!("Expected Internal error"),
    }
}

/// Test from_std_error() creates Internal error
#[test]
fn test_from_std_error() {
    #[derive(Debug)]
    struct StdErrorImpl(&'static str);
    impl std::fmt::Display for StdErrorImpl {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl StdError for StdErrorImpl {}
    unsafe impl Send for StdErrorImpl {}
    unsafe impl Sync for StdErrorImpl {}

    let std_error = StdErrorImpl("something went wrong");
    let api_error = ApiError::from_std_error(std_error);

    match api_error {
        ApiError::Internal {
            message,
            error_id,
            source,
            ..
        } => {
            assert_eq!(
                message,
                "An internal error occurred. Please try again later."
            );
            assert!(source.is_some());
            assert!(error_id.len() == 16); // hex format
            assert!(error_id.chars().all(|c| c.is_ascii_hexdigit()));
        }
        _ => panic!("Expected Internal error"),
    }
}

/// Test error chain propagation with multiple layers
#[test]
fn test_error_chain_propagation() {
    #[derive(Debug)]
    struct BottomError(&'static str);
    impl std::fmt::Display for BottomError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Bottom: {}", self.0)
        }
    }
    impl StdError for BottomError {}
    unsafe impl Send for BottomError {}
    unsafe impl Sync for BottomError {}

    let bottom =
        ApiError::internal_with_source("Base error", "BASE001", BottomError("database failure"));

    // The source should be accessible
    assert!(bottom.source().is_some());
    let source_msg = bottom.source().unwrap().to_string();
    assert!(source_msg.contains("Bottom"));
    assert!(source_msg.contains("database failure"));
}

/// Test ErrorContext Default implementation
#[test]
fn test_error_context_default() {
    let ctx = ErrorContext::default();
    assert!(ctx.file.is_none());
    assert!(ctx.line.is_none());
    assert!(ctx.function.is_none());
    assert!(ctx.extra.is_empty());
}

/// Test ServiceUnavailable with source
#[test]
fn test_service_unavailable_with_source() {
    #[derive(Debug)]
    struct ServiceError(&'static str);
    impl std::fmt::Display for ServiceError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    impl StdError for ServiceError {}
    unsafe impl Send for ServiceError {}
    unsafe impl Sync for ServiceError {}

    let error = ApiError::service_unavailable_with_source(
        "database",
        Some(60),
        ServiceError("connection pool exhausted"),
    );

    match error {
        ApiError::ServiceUnavailable {
            service,
            retry_after,
            source,
        } => {
            assert_eq!(service, "database");
            assert_eq!(retry_after, Some(60));
            assert!(source.is_some());
            let source_msg = source.unwrap().to_string();
            assert!(source_msg.contains("connection pool exhausted"));
        }
        _ => panic!("Expected ServiceUnavailable error"),
    }
}

/// Test backward compatibility - existing constructors still work
#[test]
fn test_backward_compatibility() {
    // Test all existing constructors still work
    let not_found = ApiError::not_found("user", Some("123".into()));
    assert!(matches!(not_found, ApiError::NotFound { .. }));

    let invalid = ApiError::invalid_input("bad data", Some("email".into()), None);
    assert!(matches!(invalid, ApiError::InvalidInput { .. }));

    let auth_failed = ApiError::authentication_failed("Invalid token");
    assert!(matches!(auth_failed, ApiError::AuthenticationFailed { .. }));

    let access_denied = ApiError::access_denied("admin", Some("user1".into()));
    assert!(matches!(access_denied, ApiError::AccessDenied { .. }));

    let rate_limit = ApiError::rate_limit_exceeded(100, 60);
    assert!(matches!(rate_limit, ApiError::RateLimitExceeded { .. }));

    let internal = ApiError::internal_error("Something went wrong", "ERR123");
    assert!(matches!(internal, ApiError::Internal { .. }));

    let unavailable = ApiError::service_unavailable("database", Some(30));
    assert!(matches!(unavailable, ApiError::ServiceUnavailable { .. }));

    let validation = ApiError::validation("email", "must be valid");
    assert!(matches!(validation, ApiError::ValidationError { .. }));
}

/// Test SdForgeError unified error type
#[test]
fn test_sdforge_error_api_variant() {
    let api_err = ApiError::not_found("user", Some("123".into()));
    let sdforge_err: SdForgeError = api_err.into();

    assert!(matches!(sdforge_err, SdForgeError::Api(_)));
    assert_eq!(sdforge_err.category(), ErrorCategory::ClientError);
}

/// Test SdForgeError internal constructor
#[test]
fn test_sdforge_error_internal() {
    let err = SdForgeError::internal("test error");

    match &err {
        SdForgeError::Internal(msg) => {
            assert_eq!(msg, &"test error");
        }
        _ => panic!("Expected Internal variant"),
    }
    assert_eq!(err.category(), ErrorCategory::ServerError);
}

/// Test SdForgeError sanitized_message
#[test]
fn test_sdforge_error_sanitized_message() {
    let internal = SdForgeError::internal("Database connection failed: host=localhost");
    let msg = internal.sanitized_message();
    assert!(msg.contains("Database")); // Not sanitized for Internal variant

    let api_internal = ApiError::internal_error("DB failed", "ERR001");
    let sdforge_err: SdForgeError = api_internal.into();
    let msg = sdforge_err.sanitized_message();
    assert!(msg.contains("internal error")); // Sanitized
    assert!(!msg.contains("DB failed"));
}

/// Test SdForgeError to_service_error conversion
#[test]
fn test_sdforge_error_to_service_error() {
    let api_err = ApiError::not_found("resource", None);
    let sdforge_err: SdForgeError = api_err.into();
    let service_err = sdforge_err.to_service_error();

    // Should preserve the error details
    assert!(service_err.code == "NOT_FOUND" || service_err.code.contains("NOT_FOUND"));
}

// ========================================================================
// to_mcp_json() comprehensive tests for all 8 ApiError variants
// ========================================================================

#[test]
fn test_to_mcp_json_not_found() {
    let error = ApiError::NotFound {
        resource: "user".to_string(),
        resource_id: Some("123".to_string()),
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "NOT_FOUND");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("user")
    );
}

#[test]
fn test_to_mcp_json_invalid_input() {
    let error = ApiError::InvalidInput {
        message: "bad value".to_string(),
        field: Some("email".to_string()),
        value: None,
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "INVALID_INPUT");
    assert_eq!(parsed["error"]["message"], "bad value");
}

#[test]
fn test_to_mcp_json_authentication_failed() {
    let error = ApiError::AuthenticationFailed {
        reason: "bad token".to_string(),
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "AUTHENTICATION_FAILED");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("bad token")
    );
}

#[test]
fn test_to_mcp_json_access_denied() {
    let error = ApiError::AccessDenied {
        permission: "admin.write".to_string(),
        user_id: Some("user1".to_string()),
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "ACCESS_DENIED");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("admin.write")
    );
}

#[test]
fn test_to_mcp_json_rate_limit_exceeded() {
    let error = ApiError::RateLimitExceeded {
        limit: 100,
        window_seconds: 60,
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "RATE_LIMIT_EXCEEDED");
    assert_eq!(parsed["error"]["message"], "Rate limit exceeded");
}

#[test]
fn test_to_mcp_json_internal() {
    let error = ApiError::Internal {
        message: "db failure".to_string(),
        error_id: "ERR001".to_string(),
        source: None,
        context: None,
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "INTERNAL_ERROR");
    assert_eq!(parsed["error"]["message"], "db failure");
}

#[test]
fn test_to_mcp_json_service_unavailable() {
    let error = ApiError::ServiceUnavailable {
        service: "database".to_string(),
        retry_after: Some(30),
        source: None,
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "SERVICE_UNAVAILABLE");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("database")
    );
}

#[test]
fn test_to_mcp_json_validation_error() {
    let error = ApiError::ValidationError {
        field: "email".to_string(),
        constraint: "required".to_string(),
    };
    let json = error.to_mcp_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["success"], false);
    assert_eq!(parsed["error"]["code"], "VALIDATION_ERROR");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("email")
    );
    assert!(
        parsed["error"]["message"]
            .as_str()
            .unwrap()
            .contains("required")
    );
}

// ========================================================================
// SdForgeError category()/sanitized_message() tests for security/http variants
// ========================================================================

#[test]
#[cfg(feature = "security")]
fn test_sdforge_error_category_auth_variants() {
    let auth_err: SdForgeError = crate::security::AuthError::MissingAuth.into();
    assert_eq!(auth_err.category(), ErrorCategory::AuthError);

    let jwt_err: SdForgeError = crate::security::JwtError::InvalidFormat.into();
    assert_eq!(jwt_err.category(), ErrorCategory::AuthError);

    let auth_config_err: SdForgeError =
        crate::security::AuthConfigError::InvalidSecret("too short".to_string()).into();
    assert_eq!(auth_config_err.category(), ErrorCategory::AuthError);
}

#[test]
#[cfg(feature = "http")]
fn test_sdforge_error_category_config_variant() {
    let config_err: SdForgeError = crate::config::ConfigError::FileNotFound {
        path: "/missing.toml".to_string(),
    }
    .into();
    assert_eq!(config_err.category(), ErrorCategory::ClientError);
}

#[test]
#[cfg(feature = "security")]
fn test_sdforge_error_sanitized_message_security_variants() {
    let auth_err: SdForgeError = crate::security::AuthError::MissingAuth.into();
    let msg = auth_err.sanitized_message();
    // The `other` branch calls to_string() on the underlying error
    assert!(msg.contains("authorization header"));

    let jwt_err: SdForgeError = crate::security::JwtError::InvalidFormat.into();
    let msg = jwt_err.sanitized_message();
    assert!(!msg.is_empty());

    let auth_config_err: SdForgeError =
        crate::security::AuthConfigError::InvalidSecret("bad".to_string()).into();
    let msg = auth_config_err.sanitized_message();
    assert!(msg.contains("Invalid secret"));
}

#[test]
#[cfg(feature = "http")]
fn test_sdforge_error_sanitized_message_config_variant() {
    let config_err: SdForgeError = crate::config::ConfigError::FileNotFound {
        path: "/missing.toml".to_string(),
    }
    .into();
    let msg = config_err.sanitized_message();
    assert!(msg.contains("File not found"));
    assert!(msg.contains("/missing.toml"));
}

#[test]
fn test_sdforge_error_to_service_error_all_api_variants() {
    // Exercise to_service_error for each ApiError variant via SdForgeError
    let errors: Vec<ApiError> = vec![
        ApiError::not_found("user", Some("123".into())),
        ApiError::invalid_input("bad", Some("email".into()), None),
        ApiError::authentication_failed("bad token"),
        ApiError::access_denied("admin", Some("u1".into())),
        ApiError::rate_limit_exceeded(100, 60),
        ApiError::internal_error("db fail", "ERR001"),
        ApiError::service_unavailable("db", Some(30)),
        ApiError::validation("email", "required"),
    ];

    let expected_codes = [
        "NOT_FOUND",
        "INVALID_INPUT",
        "AUTHENTICATION_FAILED",
        "ACCESS_DENIED",
        "RATE_LIMIT_EXCEEDED",
        "INTERNAL_ERROR",
        "SERVICE_UNAVAILABLE",
        "VALIDATION_ERROR",
    ];

    for (err, expected_code) in errors.into_iter().zip(expected_codes.iter()) {
        let sdforge_err: SdForgeError = err.into();
        let service_err = sdforge_err.to_service_error();
        assert_eq!(
            service_err.code, *expected_code,
            "Expected code {} but got {}",
            expected_code, service_err.code
        );
    }
}

#[test]
fn test_sdforge_error_to_service_error_internal_string() {
    let err = SdForgeError::internal("custom internal failure");
    let service_err = err.to_service_error();
    assert_eq!(service_err.code, "INTERNAL_ERROR");
}

#[test]
fn test_api_error_to_service_error_from_all_variants() {
    // Exercise the From<ApiError> for ServiceError impl for all variants
    let not_found = ApiError::not_found("user", Some("123".into()));
    let svc: ServiceError = not_found.into();
    assert_eq!(svc.code, "NOT_FOUND");

    let invalid = ApiError::invalid_input("bad", Some("email".into()), None);
    let svc: ServiceError = invalid.into();
    assert_eq!(svc.code, "INVALID_INPUT");

    let auth = ApiError::authentication_failed("bad token");
    let svc: ServiceError = auth.into();
    assert_eq!(svc.code, "AUTHENTICATION_FAILED");

    let access = ApiError::access_denied("admin", Some("u1".into()));
    let svc: ServiceError = access.into();
    assert_eq!(svc.code, "ACCESS_DENIED");

    let rate = ApiError::rate_limit_exceeded(100, 60);
    let svc: ServiceError = rate.into();
    assert_eq!(svc.code, "RATE_LIMIT_EXCEEDED");

    let internal = ApiError::internal_error("db fail", "ERR001");
    let svc: ServiceError = internal.into();
    assert_eq!(svc.code, "INTERNAL_ERROR");

    let unavail = ApiError::service_unavailable("db", Some(30));
    let svc: ServiceError = unavail.into();
    assert_eq!(svc.code, "SERVICE_UNAVAILABLE");

    let validation = ApiError::validation("email", "required");
    let svc: ServiceError = validation.into();
    assert_eq!(svc.code, "VALIDATION_ERROR");
}
