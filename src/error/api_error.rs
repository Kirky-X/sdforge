// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use thiserror::Error;

use crate::core::response::ServiceError;
use crate::error::context::{ErrorCategory, ErrorContext};

#[cfg(feature = "ratelimit")]
use crate::security::ratelimit::RateLimitError;

/// Framework errors
///
/// Represents various error conditions that can occur during request processing.
/// Each variant includes appropriate metadata for error reporting and handling.
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ApiError {
    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound {
        /// The type of resource that was not found
        resource: String,
        /// The specific resource identifier that was not found
        resource_id: Option<String>,
    },

    /// Invalid input
    #[error("Invalid input: {message}")]
    InvalidInput {
        /// The error message describing what was invalid
        message: String,
        /// The field that had invalid input
        field: Option<String>,
        /// The invalid value that was provided
        value: Option<serde_json::Value>,
    },

    /// Authentication failed
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed {
        /// The reason authentication failed
        reason: String,
    },

    /// Access denied
    #[error("Access denied: {permission}")]
    AccessDenied {
        /// The permission that was denied
        permission: String,
        /// The user ID that was denied access
        user_id: Option<String>,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded {
        /// The maximum number of requests allowed in the window
        limit: u32,
        /// The duration of the rate limit window in seconds
        window_seconds: u32,
    },

    /// Internal server error
    /// Security: message is sanitized to not leak internal implementation details
    #[error("Internal server error")]
    Internal {
        /// Sanitized error message (never contains sensitive data like paths, stack traces, or internal error details)
        message: String,
        /// A unique identifier for this error (for debugging)
        error_id: String,
        /// Optional source error for error chaining
        #[source]
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
        /// Optional context information
        context: Option<ErrorContext>,
    },

    /// Service unavailable
    #[error("Service unavailable: {service}")]
    ServiceUnavailable {
        /// The service that is unavailable
        service: String,
        /// Seconds to wait before retrying
        retry_after: Option<u64>,
        /// Optional source error for error chaining
        #[source]
        #[serde(skip)]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// Validation error
    #[error("Validation failed: {field}")]
    ValidationError {
        /// The field that failed validation
        field: String,
        /// The constraint that was not satisfied
        constraint: String,
    },
}

impl ApiError {
    /// Create a validation error
    pub fn validation_error(_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field: None,
            value: None,
        }
    }

    /// Get the error category for this error
    ///
    /// This allows proper error handling and classification for monitoring,
    /// alerting, and user-facing error messages.
    pub fn category(&self) -> ErrorCategory {
        match self {
            ApiError::NotFound { .. } => ErrorCategory::ClientError,
            ApiError::InvalidInput { .. } => ErrorCategory::ClientError,
            ApiError::AuthenticationFailed { .. } => ErrorCategory::AuthError,
            ApiError::AccessDenied { .. } => ErrorCategory::AuthError,
            ApiError::RateLimitExceeded { .. } => ErrorCategory::RateLimitError,
            ApiError::Internal { .. } => ErrorCategory::ServerError,
            ApiError::ServiceUnavailable { .. } => ErrorCategory::ServerError,
            ApiError::ValidationError { .. } => ErrorCategory::ValidationError,
        }
    }

    /// Get the underlying source error if available
    ///
    /// This allows error chaining for debugging purposes.
    /// The source is typically None for client-facing errors,
    /// but may contain the original error for Internal or ServiceUnavailable errors.
    pub fn source(&self) -> Option<&(dyn std::error::Error + Send + Sync + 'static)> {
        match self {
            ApiError::Internal { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn std::error::Error + Send + Sync + 'static)),
            ApiError::ServiceUnavailable { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &(dyn std::error::Error + Send + Sync + 'static)),
            _ => None,
        }
    }

    /// Create a new Internal error (backwards compatible)
    ///
    /// This is the recommended way to create Internal errors without source.
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    pub fn internal_error(message: impl Into<String>, error_id: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: None,
            context: None,
        }
    }

    /// Create a new Internal error with source error
    ///
    /// This is the recommended way to create Internal errors from other errors.
    /// The source error is stored for debugging purposes.
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    /// * `source` - The underlying error that caused this error
    pub fn internal_with_source<E: StdError + Send + Sync + 'static>(
        message: impl Into<String>,
        error_id: impl Into<String>,
        source: E,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: Some(Box::new(source)),
            context: None,
        }
    }

    /// Create a new Internal error with context
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    /// * `context` - The context information where the error occurred
    pub fn internal_with_context(
        message: impl Into<String>,
        error_id: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: None,
            context: Some(context),
        }
    }

    /// Create a new Internal error with both source and context
    ///
    /// # Arguments
    ///
    /// * `message` - A sanitized error message for the user
    /// * `error_id` - A unique identifier for debugging
    /// * `source` - The underlying error that caused this error
    /// * `context` - The context information where the error occurred
    pub fn internal_with_source_and_context<E: StdError + Send + Sync + 'static>(
        message: impl Into<String>,
        error_id: impl Into<String>,
        source: E,
        context: ErrorContext,
    ) -> Self {
        Self::Internal {
            message: message.into(),
            error_id: error_id.into(),
            source: Some(Box::new(source)),
            context: Some(context),
        }
    }

    /// Create an Internal error from a standard error
    ///
    /// This is a convenience method for converting standard library errors
    /// into ApiError::Internal with automatic message sanitization.
    ///
    /// # Arguments
    ///
    /// * `error` - Any error that implements StdError
    pub fn from_std_error<E: StdError + Send + Sync + 'static>(error: E) -> Self {
        // Generate a simple error_id without rand dependency.
        //
        // Use `unwrap_or_default()` instead of `unwrap()`: if the system clock
        // is before UNIX_EPOCH (e.g. skewed clock in containers), `unwrap()`
        // would panic inside an error-handling path, masking the original
        // error. `unwrap_or_default()` returns a zero Duration, producing the
        // error_id "0000000000000000" — a degenerate but non-fatal value.
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let error_id = format!("{:016x}", timestamp);
        Self::Internal {
            message: "An internal error occurred. Please try again later.".to_string(),
            error_id,
            source: Some(Box::new(error)),
            context: None,
        }
    }

    /// Create a ServiceUnavailable error (backwards compatible)
    pub fn service_unavailable(service: impl Into<String>, retry_after: Option<u64>) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
            retry_after,
            source: None,
        }
    }

    /// Create a ServiceUnavailable error with source
    ///
    /// # Arguments
    ///
    /// * `service` - The service that is unavailable
    /// * `retry_after` - Seconds to wait before retrying
    /// * `source` - The underlying error that caused the unavailability
    pub fn service_unavailable_with_source<E: StdError + Send + Sync + 'static>(
        service: impl Into<String>,
        retry_after: Option<u64>,
        source: E,
    ) -> Self {
        Self::ServiceUnavailable {
            service: service.into(),
            retry_after,
            source: Some(Box::new(source)),
        }
    }

    /// Create a NotFound error
    pub fn not_found(resource: impl Into<String>, resource_id: Option<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            resource_id,
        }
    }

    /// Create an InvalidInput error
    pub fn invalid_input(
        message: impl Into<String>,
        field: Option<String>,
        value: Option<serde_json::Value>,
    ) -> Self {
        Self::InvalidInput {
            message: message.into(),
            field,
            value,
        }
    }

    /// Create an AuthenticationFailed error
    pub fn authentication_failed(reason: impl Into<String>) -> Self {
        Self::AuthenticationFailed {
            reason: reason.into(),
        }
    }

    /// Create an AccessDenied error
    pub fn access_denied(permission: impl Into<String>, user_id: Option<String>) -> Self {
        Self::AccessDenied {
            permission: permission.into(),
            user_id,
        }
    }

    /// Create a RateLimitExceeded error
    pub fn rate_limit_exceeded(limit: u32, window_seconds: u32) -> Self {
        Self::RateLimitExceeded {
            limit,
            window_seconds,
        }
    }

    /// Create a ValidationError
    pub fn validation(field: impl Into<String>, constraint: impl Into<String>) -> Self {
        Self::ValidationError {
            field: field.into(),
            constraint: constraint.into(),
        }
    }

    /// Get a sanitized error message for external display
    ///
    /// This strips any sensitive information that should not be exposed to clients.
    pub fn sanitized_message(&self) -> String {
        match self {
            ApiError::Internal { .. } => {
                "An internal error occurred. Please try again later.".into()
            }
            ApiError::ServiceUnavailable { .. } => {
                "The service is temporarily unavailable. Please try again later.".into()
            }
            other => other.to_string(),
        }
    }

    /// Format error as MCP-compatible JSON string
    pub fn to_mcp_json(&self) -> String {
        let (code, message) = match self {
            ApiError::NotFound { resource, .. } => {
                ("NOT_FOUND", format!("Resource not found: {}", resource))
            }
            ApiError::InvalidInput { message, .. } => ("INVALID_INPUT", message.clone()),
            ApiError::AuthenticationFailed { reason } => (
                "AUTHENTICATION_FAILED",
                format!("Authentication failed: {}", reason),
            ),
            ApiError::AccessDenied { permission, .. } => {
                ("ACCESS_DENIED", format!("Access denied: {}", permission))
            }
            ApiError::RateLimitExceeded { .. } => {
                ("RATE_LIMIT_EXCEEDED", "Rate limit exceeded".to_string())
            }
            ApiError::Internal { message, .. } => ("INTERNAL_ERROR", message.clone()),
            ApiError::ServiceUnavailable { service, .. } => (
                "SERVICE_UNAVAILABLE",
                format!("Service unavailable: {}", service),
            ),
            ApiError::ValidationError { field, constraint } => (
                "VALIDATION_ERROR",
                format!("Validation failed for {}: {}", field, constraint),
            ),
        };

        serde_json::to_string(&serde_json::json!({
            "success": false,
            "error": { "code": code, "message": message }
        }))
        .unwrap_or_else(|_| {
            format!(r#"{{"success":false,"error":{{"code":"{code}","message":"{message}"}}}}"#)
        })
    }

    /// Convert this `ApiError` into a `ServiceError` for HTTP responses.
    ///
    /// This is the single source of truth for `ApiError` → `ServiceError`
    /// conversion. Both `SdForgeError::to_service_error` and the
    /// `From<ApiError> for ServiceError` impl delegate here to avoid
    /// duplicating the ~80-line match (which previously diverged: the
    /// `From` impl added a `timestamp` under the `timestamp` feature while
    /// `to_service_error` did not).
    pub fn to_service_error(&self) -> ServiceError {
        match self {
            ApiError::NotFound {
                resource,
                resource_id,
            } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource, "resource_id": resource_id }),
                404,
            ),
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => ServiceError::with_details(
                "INVALID_INPUT",
                message.clone(),
                serde_json::json!({ "field": field, "value": value }),
                400,
            ),
            ApiError::AuthenticationFailed { reason } => ServiceError::with_details(
                "AUTHENTICATION_FAILED",
                format!("Authentication failed: {}", reason),
                serde_json::json!({ "reason": reason }),
                401,
            ),
            ApiError::AccessDenied {
                permission,
                user_id,
            } => ServiceError::with_details(
                "ACCESS_DENIED",
                format!("Access denied: {}", permission),
                serde_json::json!({ "permission": permission, "user_id": user_id }),
                403,
            ),
            ApiError::RateLimitExceeded {
                limit,
                window_seconds,
            } => ServiceError::with_details(
                "RATE_LIMIT_EXCEEDED",
                "Rate limit exceeded".to_string(),
                serde_json::json!({ "limit": limit, "window_seconds": window_seconds }),
                429,
            ),
            ApiError::Internal {
                message,
                error_id,
                source: _,
                context,
            } => {
                let mut details = serde_json::json!({ "error_id": error_id });
                #[cfg(feature = "timestamp")]
                {
                    details["timestamp"] = serde_json::json!(chrono::Utc::now().timestamp());
                }
                if let Some(ctx) = context {
                    details["context"] = serde_json::to_value(ctx).unwrap_or(serde_json::json!({}));
                }
                ServiceError::with_details("INTERNAL_ERROR", message.clone(), details, 500)
            }
            ApiError::ServiceUnavailable {
                service,
                retry_after,
                source: _,
            } => ServiceError::with_details(
                "SERVICE_UNAVAILABLE",
                format!("Service unavailable: {}", service),
                serde_json::json!({ "service": service, "retry_after": retry_after }),
                503,
            ),
            ApiError::ValidationError { field, constraint } => ServiceError::with_details(
                "VALIDATION_ERROR",
                format!("Validation failed: {}", field),
                serde_json::json!({ "field": field, "constraint": constraint }),
                422,
            ),
        }
    }
}

// Backward compatibility - keep existing From implementation
impl From<ApiError> for ServiceError {
    fn from(err: ApiError) -> Self {
        err.to_service_error()
    }
}

/// Convert a `RateLimitError` into an `ApiError`.
///
/// Mapping (see design.md D5):
/// - `Exceeded { limit, window_seconds }` → `RateLimitExceeded` (HTTP 429)
/// - `Banned { .. }` → `AccessDenied` (HTTP 403)
/// - `CircuitOpen` → `ServiceUnavailable` (HTTP 503)
/// - `QuotaExhausted { used, total }` → `RateLimitExceeded` (semantically
///   imperfect — `window_seconds` is reused to carry `used`; tracked as
///   technical debt in design.md D8)
/// - `Limiteron(e)` → `Internal` with source preserved for error chaining
///
/// `RateLimitError` uses `u64` for limit/window counters, while
/// `ApiError::RateLimitExceeded` uses `u32`. The conversion uses a
/// saturating cast (`u32::try_from(v).unwrap_or(u32::MAX)`) to avoid silent
/// truncation on overflow.
#[cfg(feature = "ratelimit")]
impl From<RateLimitError> for ApiError {
    fn from(e: RateLimitError) -> Self {
        // Saturating cast: u64 → u32, clamping at u32::MAX on overflow.
        fn cast_u32(v: u64) -> u32 {
            u32::try_from(v).unwrap_or(u32::MAX)
        }

        match e {
            RateLimitError::Exceeded {
                limit,
                window_seconds,
            } => ApiError::RateLimitExceeded {
                limit: cast_u32(limit),
                window_seconds: cast_u32(window_seconds),
            },
            RateLimitError::Banned { .. } => ApiError::AccessDenied {
                permission: "rate_limit".to_string(),
                user_id: None,
            },
            RateLimitError::CircuitOpen => ApiError::ServiceUnavailable {
                service: "circuit_breaker".to_string(),
                retry_after: None,
                source: None,
            },
            RateLimitError::QuotaExhausted { used, total } => ApiError::RateLimitExceeded {
                limit: cast_u32(total),
                window_seconds: cast_u32(used),
            },
            RateLimitError::Limiteron(e) => {
                let message = e.to_string();
                ApiError::internal_with_source(message, "ratelimit_error", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::response::ServiceError;
    use crate::error::context::{ErrorCategory, ErrorContext};

    /// A simple source error used to test error chaining.
    #[derive(Debug, thiserror::Error)]
    #[error("test source error: {0}")]
    struct TestSourceError(String);

    // =========================================================================
    // Constructor tests
    // =========================================================================

    /// Test validation_error() constructor produces InvalidInput with no field/value.
    #[test]
    fn test_validation_error_constructor() {
        let err = ApiError::validation_error("VALIDATION", "field is required");
        match err {
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => {
                assert_eq!(message, "field is required");
                assert!(field.is_none());
                assert!(value.is_none());
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    /// Test that `From<RateLimitError>` converts the `Limiteron` variant to
    /// `ApiError::Internal` with source (lines 532-534).
    #[cfg(feature = "ratelimit")]
    #[test]
    fn test_from_ratelimit_error_limiteron_variant() {
        use crate::security::ratelimit::RateLimitError;
        use limiteron::LimiteronError;

        let err =
            RateLimitError::Limiteron(LimiteronError::ConfigError("test config error".to_string()));
        let api_error: ApiError = err.into();

        match api_error {
            ApiError::Internal { .. } => {}
            _ => panic!("Expected Internal variant for Limiteron error"),
        }
    }

    /// Test not_found() constructor.
    #[test]
    fn test_not_found_constructor() {
        let err = ApiError::not_found("User", Some("42".to_string()));
        match err {
            ApiError::NotFound {
                resource,
                resource_id,
            } => {
                assert_eq!(resource, "User");
                assert_eq!(resource_id, Some("42".to_string()));
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    /// Test not_found() constructor without a resource_id.
    #[test]
    fn test_not_found_constructor_no_id() {
        let err = ApiError::not_found("Session", None);
        match err {
            ApiError::NotFound {
                resource,
                resource_id,
            } => {
                assert_eq!(resource, "Session");
                assert!(resource_id.is_none());
            }
            _ => panic!("Expected NotFound variant"),
        }
    }

    /// Test invalid_input() constructor with all fields.
    #[test]
    fn test_invalid_input_constructor_full() {
        let err = ApiError::invalid_input(
            "bad value",
            Some("email".to_string()),
            Some(serde_json::json!("not-an-email")),
        );
        match err {
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => {
                assert_eq!(message, "bad value");
                assert_eq!(field, Some("email".to_string()));
                assert_eq!(value, Some(serde_json::json!("not-an-email")));
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    /// Test invalid_input() constructor with minimal fields.
    #[test]
    fn test_invalid_input_constructor_minimal() {
        let err = ApiError::invalid_input("bad value", None, None);
        match err {
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => {
                assert_eq!(message, "bad value");
                assert!(field.is_none());
                assert!(value.is_none());
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    /// Test authentication_failed() constructor.
    #[test]
    fn test_authentication_failed_constructor() {
        let err = ApiError::authentication_failed("expired token");
        match err {
            ApiError::AuthenticationFailed { reason } => {
                assert_eq!(reason, "expired token");
            }
            _ => panic!("Expected AuthenticationFailed variant"),
        }
    }

    /// Test access_denied() constructor with user_id.
    #[test]
    fn test_access_denied_constructor_with_user() {
        let err = ApiError::access_denied("read", Some("user-1".to_string()));
        match err {
            ApiError::AccessDenied {
                permission,
                user_id,
            } => {
                assert_eq!(permission, "read");
                assert_eq!(user_id, Some("user-1".to_string()));
            }
            _ => panic!("Expected AccessDenied variant"),
        }
    }

    /// Test access_denied() constructor without user_id.
    #[test]
    fn test_access_denied_constructor_no_user() {
        let err = ApiError::access_denied("write", None);
        match err {
            ApiError::AccessDenied {
                permission,
                user_id,
            } => {
                assert_eq!(permission, "write");
                assert!(user_id.is_none());
            }
            _ => panic!("Expected AccessDenied variant"),
        }
    }

    /// Test rate_limit_exceeded() constructor.
    #[test]
    fn test_rate_limit_exceeded_constructor() {
        let err = ApiError::rate_limit_exceeded(100, 60);
        match err {
            ApiError::RateLimitExceeded {
                limit,
                window_seconds,
            } => {
                assert_eq!(limit, 100);
                assert_eq!(window_seconds, 60);
            }
            _ => panic!("Expected RateLimitExceeded variant"),
        }
    }

    /// Test validation() constructor.
    #[test]
    fn test_validation_constructor() {
        let err = ApiError::validation("email", "invalid format");
        match err {
            ApiError::ValidationError { field, constraint } => {
                assert_eq!(field, "email");
                assert_eq!(constraint, "invalid format");
            }
            _ => panic!("Expected ValidationError variant"),
        }
    }

    /// Test internal_error() constructor.
    #[test]
    fn test_internal_error_constructor() {
        let err = ApiError::internal_error("sanitized message", "err-123");
        match err {
            ApiError::Internal {
                message,
                error_id,
                source,
                context,
            } => {
                assert_eq!(message, "sanitized message");
                assert_eq!(error_id, "err-123");
                assert!(source.is_none());
                assert!(context.is_none());
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test internal_with_source() constructor preserves the source error.
    #[test]
    fn test_internal_with_source_constructor() {
        let source_err = TestSourceError("downstream failure".to_string());
        let err = ApiError::internal_with_source("sanitized", "err-456", source_err);
        match err {
            ApiError::Internal {
                ref message,
                ref error_id,
                ref source,
                ref context,
            } => {
                assert_eq!(message, "sanitized");
                assert_eq!(error_id, "err-456");
                assert!(source.is_some());
                assert!(context.is_none());
                // Verify source is accessible via source()
                let src = err.source().expect("source should be present");
                assert!(src.to_string().contains("downstream failure"));
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test internal_with_context() constructor stores the context.
    #[test]
    fn test_internal_with_context_constructor() {
        let ctx = ErrorContext::new().with_extra("request_id".to_string(), "req-789".to_string());
        let err = ApiError::internal_with_context("msg", "err-789", ctx);
        match err {
            ApiError::Internal {
                message,
                error_id,
                source,
                context,
            } => {
                assert_eq!(message, "msg");
                assert_eq!(error_id, "err-789");
                assert!(source.is_none());
                assert!(context.is_some());
                let ctx = context.unwrap();
                assert_eq!(ctx.extra.get("request_id"), Some(&"req-789".to_string()));
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test internal_with_source_and_context() constructor stores both.
    #[test]
    fn test_internal_with_source_and_context_constructor() {
        let source_err = TestSourceError("db down".to_string());
        let ctx = ErrorContext::current();
        let err =
            ApiError::internal_with_source_and_context("sanitized", "err-000", source_err, ctx);
        match err {
            ApiError::Internal {
                message,
                error_id,
                source,
                context,
            } => {
                assert_eq!(message, "sanitized");
                assert_eq!(error_id, "err-000");
                assert!(source.is_some());
                assert!(context.is_some());
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test from_std_error() constructor generates an error_id and sanitizes message.
    #[test]
    fn test_from_std_error_constructor() {
        let source_err = TestSourceError("raw failure".to_string());
        let err = ApiError::from_std_error(source_err);
        match err {
            ApiError::Internal {
                message,
                error_id,
                source,
                context,
            } => {
                // Message must be sanitized — raw failure details must NOT leak.
                assert!(!message.contains("raw failure"));
                assert!(message.contains("internal error"));
                // error_id should be a 16-char hex string.
                assert_eq!(error_id.len(), 16);
                assert!(source.is_some());
                assert!(context.is_none());
            }
            _ => panic!("Expected Internal variant"),
        }
    }

    /// Test service_unavailable() constructor.
    #[test]
    fn test_service_unavailable_constructor() {
        let err = ApiError::service_unavailable("database", Some(30));
        match err {
            ApiError::ServiceUnavailable {
                service,
                retry_after,
                source,
            } => {
                assert_eq!(service, "database");
                assert_eq!(retry_after, Some(30));
                assert!(source.is_none());
            }
            _ => panic!("Expected ServiceUnavailable variant"),
        }
    }

    /// Test service_unavailable() constructor without retry_after.
    #[test]
    fn test_service_unavailable_constructor_no_retry() {
        let err = ApiError::service_unavailable("cache", None);
        match err {
            ApiError::ServiceUnavailable {
                service,
                retry_after,
                source,
            } => {
                assert_eq!(service, "cache");
                assert!(retry_after.is_none());
                assert!(source.is_none());
            }
            _ => panic!("Expected ServiceUnavailable variant"),
        }
    }

    /// Test service_unavailable_with_source() constructor.
    #[test]
    fn test_service_unavailable_with_source_constructor() {
        let source_err = TestSourceError("connection refused".to_string());
        let err = ApiError::service_unavailable_with_source("redis", Some(5), source_err);
        match err {
            ApiError::ServiceUnavailable {
                ref service,
                retry_after,
                ref source,
            } => {
                assert_eq!(service, "redis");
                assert_eq!(retry_after, Some(5));
                assert!(source.is_some());
                let src = err.source().expect("source should be present");
                assert!(src.to_string().contains("connection refused"));
            }
            _ => panic!("Expected ServiceUnavailable variant"),
        }
    }

    // =========================================================================
    // category() tests
    // =========================================================================

    /// Test category() returns the correct ErrorCategory for each variant.
    #[test]
    fn test_category_all_variants() {
        assert_eq!(
            ApiError::not_found("X", None).category(),
            ErrorCategory::ClientError
        );
        assert_eq!(
            ApiError::invalid_input("msg", None, None).category(),
            ErrorCategory::ClientError
        );
        assert_eq!(
            ApiError::authentication_failed("reason").category(),
            ErrorCategory::AuthError
        );
        assert_eq!(
            ApiError::access_denied("perm", None).category(),
            ErrorCategory::AuthError
        );
        assert_eq!(
            ApiError::rate_limit_exceeded(10, 60).category(),
            ErrorCategory::RateLimitError
        );
        assert_eq!(
            ApiError::internal_error("msg", "id").category(),
            ErrorCategory::ServerError
        );
        assert_eq!(
            ApiError::service_unavailable("svc", None).category(),
            ErrorCategory::ServerError
        );
        assert_eq!(
            ApiError::validation("field", "constraint").category(),
            ErrorCategory::ValidationError
        );
    }

    // =========================================================================
    // source() tests
    // =========================================================================

    /// Test source() returns None for client-facing errors.
    #[test]
    fn test_source_none_for_client_errors() {
        assert!(ApiError::not_found("X", None).source().is_none());
        assert!(
            ApiError::invalid_input("msg", None, None)
                .source()
                .is_none()
        );
        assert!(ApiError::authentication_failed("r").source().is_none());
        assert!(ApiError::access_denied("p", None).source().is_none());
        assert!(ApiError::rate_limit_exceeded(1, 1).source().is_none());
        assert!(ApiError::validation("f", "c").source().is_none());
    }

    /// Test source() returns None for Internal without a source.
    #[test]
    fn test_source_none_for_internal_without_source() {
        let err = ApiError::internal_error("msg", "id");
        assert!(err.source().is_none());
    }

    /// Test source() returns the source for Internal with a source.
    #[test]
    fn test_source_some_for_internal_with_source() {
        let err =
            ApiError::internal_with_source("msg", "id", TestSourceError("chained".to_string()));
        let src = err.source().expect("source should be present");
        assert!(src.to_string().contains("chained"));
    }

    /// Test source() returns None for ServiceUnavailable without a source.
    #[test]
    fn test_source_none_for_service_unavailable_without_source() {
        let err = ApiError::service_unavailable("svc", None);
        assert!(err.source().is_none());
    }

    /// Test source() returns the source for ServiceUnavailable with a source.
    #[test]
    fn test_source_some_for_service_unavailable_with_source() {
        let err = ApiError::service_unavailable_with_source(
            "svc",
            None,
            TestSourceError("timeout".to_string()),
        );
        let src = err.source().expect("source should be present");
        assert!(src.to_string().contains("timeout"));
    }

    // =========================================================================
    // sanitized_message() tests
    // =========================================================================

    /// Test sanitized_message() for client-facing errors returns the full message.
    #[test]
    fn test_sanitized_message_client_errors() {
        assert!(
            ApiError::not_found("User", None)
                .sanitized_message()
                .contains("User")
        );
        assert!(
            ApiError::invalid_input("bad", None, None)
                .sanitized_message()
                .contains("bad")
        );
        assert!(
            ApiError::authentication_failed("expired")
                .sanitized_message()
                .contains("expired")
        );
        assert!(
            ApiError::access_denied("read", None)
                .sanitized_message()
                .contains("read")
        );
        assert!(
            ApiError::rate_limit_exceeded(1, 1)
                .sanitized_message()
                .contains("Rate limit")
        );
        assert!(
            ApiError::validation("email", "bad")
                .sanitized_message()
                .contains("email")
        );
    }

    /// Test sanitized_message() for Internal strips sensitive details.
    #[test]
    fn test_sanitized_message_internal_sanitized() {
        let err = ApiError::internal_error("secret stack trace", "id");
        let msg = err.sanitized_message();
        assert!(!msg.contains("secret stack trace"));
        assert!(msg.contains("internal error"));
    }

    /// Test sanitized_message() for ServiceUnavailable strips sensitive details.
    #[test]
    fn test_sanitized_message_service_unavailable_sanitized() {
        let err = ApiError::service_unavailable("internal-db-host", None);
        let msg = err.sanitized_message();
        // The service name must NOT leak in the sanitized message.
        assert!(!msg.contains("internal-db-host"));
        assert!(msg.contains("unavailable") || msg.contains("try again"));
    }

    // =========================================================================
    // to_mcp_json() tests
    // =========================================================================

    /// Test to_mcp_json() produces valid JSON with success=false for each variant.
    #[test]
    fn test_to_mcp_json_all_variants() {
        let errors: Vec<(&str, ApiError)> = vec![
            ("NOT_FOUND", ApiError::not_found("User", None)),
            ("INVALID_INPUT", ApiError::invalid_input("msg", None, None)),
            (
                "AUTHENTICATION_FAILED",
                ApiError::authentication_failed("r"),
            ),
            ("ACCESS_DENIED", ApiError::access_denied("p", None)),
            ("RATE_LIMIT_EXCEEDED", ApiError::rate_limit_exceeded(1, 1)),
            ("INTERNAL_ERROR", ApiError::internal_error("m", "id")),
            (
                "SERVICE_UNAVAILABLE",
                ApiError::service_unavailable("svc", None),
            ),
            ("VALIDATION_ERROR", ApiError::validation("f", "c")),
        ];

        for (expected_code, err) in errors {
            let json = err.to_mcp_json();
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap_or_else(|e| {
                panic!(
                    "to_mcp_json should produce valid JSON for {}: {}",
                    expected_code, e
                )
            });
            assert_eq!(
                parsed["success"], false,
                "success should be false for {}",
                expected_code
            );
            assert_eq!(
                parsed["error"]["code"], expected_code,
                "error code mismatch for {}",
                expected_code
            );
            assert!(
                parsed["error"]["message"].is_string(),
                "error message should be a string for {}",
                expected_code
            );
        }
    }

    /// Test to_mcp_json() for NotFound includes the resource in the message.
    #[test]
    fn test_to_mcp_json_not_found_includes_resource() {
        let err = ApiError::not_found("Document", None);
        let json = err.to_mcp_json();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let message = parsed["error"]["message"].as_str().unwrap();
        assert!(message.contains("Document"));
    }

    // =========================================================================
    // to_service_error() tests
    // =========================================================================

    /// Test to_service_error() produces the correct code and HTTP status for each variant.
    #[test]
    fn test_to_service_error_all_variants() {
        let cases: Vec<(&str, u16, ApiError)> = vec![
            ("NOT_FOUND", 404, ApiError::not_found("X", None)),
            (
                "INVALID_INPUT",
                400,
                ApiError::invalid_input("m", None, None),
            ),
            (
                "AUTHENTICATION_FAILED",
                401,
                ApiError::authentication_failed("r"),
            ),
            ("ACCESS_DENIED", 403, ApiError::access_denied("p", None)),
            (
                "RATE_LIMIT_EXCEEDED",
                429,
                ApiError::rate_limit_exceeded(1, 1),
            ),
            ("INTERNAL_ERROR", 500, ApiError::internal_error("m", "id")),
            (
                "SERVICE_UNAVAILABLE",
                503,
                ApiError::service_unavailable("svc", None),
            ),
            ("VALIDATION_ERROR", 422, ApiError::validation("f", "c")),
        ];

        for (expected_code, expected_status, err) in cases {
            let service_err = err.to_service_error();
            assert_eq!(
                service_err.code(),
                expected_code,
                "code mismatch for {}",
                expected_code
            );
            assert_eq!(
                service_err.http_status(),
                expected_status,
                "status mismatch for {}",
                expected_code
            );
        }
    }

    /// Test to_service_error() for NotFound includes resource and resource_id in details.
    #[test]
    fn test_to_service_error_not_found_details() {
        let err = ApiError::not_found("User", Some("42".to_string()));
        let service_err = err.to_service_error();
        let details = service_err.details().expect("details should be present");
        assert_eq!(details["resource"], "User");
        assert_eq!(details["resource_id"], "42");
    }

    /// Test to_service_error() for Internal includes error_id in details.
    #[test]
    fn test_to_service_error_internal_includes_error_id() {
        let err = ApiError::internal_error("msg", "err-abc");
        let service_err = err.to_service_error();
        let details = service_err.details().expect("details should be present");
        assert_eq!(details["error_id"], "err-abc");
    }

    /// Test to_service_error() for Internal with context includes context in details.
    #[test]
    fn test_to_service_error_internal_with_context_details() {
        let ctx = ErrorContext::new().with_extra("trace_id".to_string(), "trace-123".to_string());
        let err = ApiError::internal_with_context("msg", "id", ctx);
        let service_err = err.to_service_error();
        let details = service_err.details().expect("details should be present");
        assert!(
            details.get("context").is_some(),
            "context should be in details"
        );
    }

    /// Test to_service_error() for ServiceUnavailable includes service and retry_after.
    #[test]
    fn test_to_service_error_service_unavailable_details() {
        let err = ApiError::service_unavailable("redis", Some(30));
        let service_err = err.to_service_error();
        let details = service_err.details().expect("details should be present");
        assert_eq!(details["service"], "redis");
        assert_eq!(details["retry_after"], 30);
    }

    // =========================================================================
    // From<ApiError> for ServiceError tests
    // =========================================================================

    /// Test From<ApiError> for ServiceError delegates to to_service_error().
    #[test]
    fn test_from_api_error_for_service_error() {
        let api_err = ApiError::not_found("User", None);
        let service_err: ServiceError = api_err.into();
        assert_eq!(service_err.code(), "NOT_FOUND");
        assert_eq!(service_err.http_status(), 404);
    }

    // =========================================================================
    // Display / Debug tests
    // =========================================================================

    /// Test Display for each variant produces the expected message.
    #[test]
    fn test_display_all_variants() {
        assert!(
            ApiError::not_found("User", None)
                .to_string()
                .contains("User")
        );
        assert!(
            ApiError::invalid_input("bad", None, None)
                .to_string()
                .contains("bad")
        );
        assert!(
            ApiError::authentication_failed("reason")
                .to_string()
                .contains("reason")
        );
        assert!(
            ApiError::access_denied("perm", None)
                .to_string()
                .contains("perm")
        );
        assert!(
            ApiError::rate_limit_exceeded(1, 1)
                .to_string()
                .contains("Rate limit")
        );
        assert_eq!(
            ApiError::internal_error("m", "id").to_string(),
            "Internal server error"
        );
        assert!(
            ApiError::service_unavailable("svc", None)
                .to_string()
                .contains("svc")
        );
        assert!(
            ApiError::validation("field", "c")
                .to_string()
                .contains("field")
        );
    }

    /// Test Debug formatting produces a string containing the variant name.
    #[test]
    fn test_debug_format() {
        let err = ApiError::not_found("User", None);
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
        assert!(debug.contains("User"));
    }

    // =========================================================================
    // Serialize / Deserialize tests
    // =========================================================================

    /// Test ApiError can be serialized and deserialized (round-trip).
    #[test]
    fn test_serde_roundtrip_not_found() {
        let err = ApiError::not_found("User", Some("42".to_string()));
        let json = serde_json::to_string(&err).expect("serialization should succeed");
        let restored: ApiError =
            serde_json::from_str(&json).expect("deserialization should succeed");
        match restored {
            ApiError::NotFound {
                resource,
                resource_id,
            } => {
                assert_eq!(resource, "User");
                assert_eq!(resource_id, Some("42".to_string()));
            }
            _ => panic!("Expected NotFound variant after round-trip"),
        }
    }

    /// Test serialized ApiError includes the "type" tag.
    #[test]
    fn test_serde_includes_type_tag() {
        let err = ApiError::validation("email", "invalid");
        let json = serde_json::to_string(&err).expect("serialization should succeed");
        assert!(
            json.contains(r#""type":"ValidationError""#),
            "json should include type tag: {}",
            json
        );
    }

    /// Test that Internal and ServiceUnavailable variants skip the source field
    /// during serialization (source is #[serde(skip)]).
    #[test]
    fn test_serde_skips_source_field() {
        let err =
            ApiError::internal_with_source("msg", "id", TestSourceError("secret".to_string()));
        let json = serde_json::to_string(&err).expect("serialization should succeed");
        // The source error message must NOT appear in the serialized output.
        assert!(
            !json.contains("secret"),
            "source should be skipped in serialization"
        );
    }

    /// Test ApiError implements Send + Sync.
    #[test]
    fn test_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ApiError>();
    }

    // =========================================================================
    // From<RateLimitError> for ApiError conversion tests
    // =========================================================================

    /// `RateLimitError::Exceeded` maps to `ApiError::RateLimitExceeded`
    /// preserving `limit` and `window_seconds` (with u64 → u32 cast).
    #[cfg(feature = "ratelimit")]
    #[test]
    fn ratelimit_exceeded_maps_to_rate_limit_exceeded() {
        let err = ApiError::from(RateLimitError::Exceeded {
            limit: 100,
            window_seconds: 60,
        });
        match err {
            ApiError::RateLimitExceeded {
                limit,
                window_seconds,
            } => {
                assert_eq!(limit, 100);
                assert_eq!(window_seconds, 60);
            }
            _ => panic!("Expected RateLimitExceeded, got {err:?}"),
        }
    }

    /// `RateLimitError::Banned` maps to `ApiError::AccessDenied` with
    /// `permission = "rate_limit"` and `user_id = None`.
    #[cfg(feature = "ratelimit")]
    #[test]
    fn ratelimit_banned_maps_to_access_denied() {
        let err = ApiError::from(RateLimitError::Banned {
            reason: "abuse".to_string(),
        });
        match err {
            ApiError::AccessDenied {
                permission,
                user_id,
            } => {
                assert_eq!(permission, "rate_limit");
                assert!(user_id.is_none());
            }
            _ => panic!("Expected AccessDenied, got {err:?}"),
        }
    }

    /// `RateLimitError::CircuitOpen` maps to `ApiError::ServiceUnavailable`
    /// with `service = "circuit_breaker"`, no retry_after, no source.
    #[cfg(feature = "ratelimit")]
    #[test]
    fn ratelimit_circuit_open_maps_to_service_unavailable() {
        let err = ApiError::from(RateLimitError::CircuitOpen);
        match err {
            ApiError::ServiceUnavailable {
                service,
                retry_after,
                source,
            } => {
                assert_eq!(service, "circuit_breaker");
                assert!(retry_after.is_none());
                assert!(source.is_none());
            }
            _ => panic!("Expected ServiceUnavailable, got {err:?}"),
        }
    }

    /// `RateLimitError::QuotaExhausted { used, total }` maps to
    /// `ApiError::RateLimitExceeded { limit: total, window_seconds: used }`.
    /// Semantically imperfect (window_seconds carries `used`) — tracked as
    /// tech debt in design.md D8.
    #[cfg(feature = "ratelimit")]
    #[test]
    fn ratelimit_quota_exhausted_maps_to_rate_limit_exceeded() {
        let err = ApiError::from(RateLimitError::QuotaExhausted {
            used: 50,
            total: 100,
        });
        match err {
            ApiError::RateLimitExceeded {
                limit,
                window_seconds,
            } => {
                assert_eq!(limit, 100);
                assert_eq!(window_seconds, 50);
            }
            _ => panic!("Expected RateLimitExceeded, got {err:?}"),
        }
    }

    /// u64 → u32 saturating cast: values exceeding `u32::MAX` clamp to
    /// `u32::MAX` rather than silently truncating.
    #[cfg(feature = "ratelimit")]
    #[test]
    fn ratelimit_exceeded_u64_overflow_saturates_to_u32_max() {
        let err = ApiError::from(RateLimitError::Exceeded {
            limit: u64::MAX,
            window_seconds: u64::MAX,
        });
        match err {
            ApiError::RateLimitExceeded {
                limit,
                window_seconds,
            } => {
                assert_eq!(limit, u32::MAX);
                assert_eq!(window_seconds, u32::MAX);
            }
            _ => panic!("Expected RateLimitExceeded, got {err:?}"),
        }
    }
}
