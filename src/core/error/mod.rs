// Copyright (c) 2026 Kirky.X
//! Framework error types
//!
//! Provides comprehensive error types for the framework.

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    },

    /// Service unavailable
    #[error("Service unavailable: {service}")]
    ServiceUnavailable {
        /// The service that is unavailable
        service: String,
        /// Seconds to wait before retrying
        retry_after: Option<u64>,
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
}

use super::response::ServiceError;

impl From<ApiError> for ServiceError {
    fn from(err: ApiError) -> Self {
        match err {
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
            ApiError::Internal { message, error_id } => ServiceError::with_details(
                "INTERNAL_ERROR",
                message.clone(),
                {
                    #[cfg(feature = "timestamp")]
                    let details = serde_json::json!({ "error_id": error_id, "timestamp": chrono::Utc::now().timestamp() });
                    #[cfg(not(feature = "timestamp"))]
                    let details = serde_json::json!({ "error_id": error_id });
                    details
                },
                500,
            ),
            ApiError::ServiceUnavailable {
                service,
                retry_after,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ApiError::NotFound variant
    #[test]
    fn test_api_error_not_found() {
        let error = ApiError::NotFound {
            resource: "user".to_string(),
            resource_id: Some("123".to_string()),
        };
        assert!(error.to_string().contains("Resource not found"));
        assert!(error.to_string().contains("user"));
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
    }

    /// Test ApiError::AuthenticationFailed variant
    #[test]
    fn test_api_error_authentication_failed() {
        let error = ApiError::AuthenticationFailed {
            reason: "Invalid token".to_string(),
        };
        assert!(error.to_string().contains("Authentication failed"));
        assert!(error.to_string().contains("Invalid token"));
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
    }

    /// Test ApiError::RateLimitExceeded variant
    #[test]
    fn test_api_error_rate_limit_exceeded() {
        let error = ApiError::RateLimitExceeded {
            limit: 100,
            window_seconds: 60,
        };
        assert!(error.to_string().contains("Rate limit exceeded"));
    }

    /// Test ApiError::Internal variant
    #[test]
    fn test_api_error_internal() {
        let error = ApiError::Internal {
            message: "Database connection failed".to_string(),
            error_id: "abc123".to_string(),
        };
        assert!(error.to_string().contains("Internal server error"));
        // Message should be sanitized (internal details not leaked)
    }

    /// Test ApiError::ServiceUnavailable variant
    #[test]
    fn test_api_error_service_unavailable() {
        let error = ApiError::ServiceUnavailable {
            service: "external_service".to_string(),
            retry_after: Some(30),
        };
        assert!(error.to_string().contains("Service unavailable"));
        assert!(error.to_string().contains("external_service"));
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
}
