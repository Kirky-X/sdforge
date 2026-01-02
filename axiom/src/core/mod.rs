//! Core types and error handling

pub mod validation;

#[cfg(feature = "http")]
use axum::body::Body;
#[cfg(feature = "http")]
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API metadata (protocol-agnostic)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiMetadata {
    /// API name
    pub name: &'static str,
    /// API version
    pub version: &'static str,
    /// API description
    pub description: &'static str,
    /// Cache TTL in seconds (None means no caching)
    pub cache_ttl: Option<u64>,
    /// Whether this is a streaming endpoint
    pub is_streaming: bool,
}

impl ApiMetadata {
    /// Create new API metadata
    pub fn new(
        name: &'static str,
        version: &'static str,
        description: &'static str,
        cache_ttl: Option<u64>,
        is_streaming: bool,
    ) -> Self {
        Self {
            name,
            version,
            description,
            cache_ttl,
            is_streaming,
        }
    }
}

/// Unified response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceResponse<T = serde_json::Value> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ServiceError>,
    /// Response timestamp
    #[cfg(feature = "timestamp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

impl<T> ServiceResponse<T>
where
    T: Serialize,
{
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            #[cfg(feature = "timestamp")]
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// Create an error response
    /// Note: The generic parameter T is required by the struct definition but is not used
    /// for error responses (data is always None). Error responses typically use the default
    /// T = serde_json::Value.
    pub fn error(error: ServiceError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            #[cfg(feature = "timestamp")]
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }
}

/// Service error representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// HTTP status code
    pub http_status: u16,
}

impl ServiceError {
    /// Create a new service error
    pub fn new(code: impl Into<String>, message: impl Into<String>, http_status: u16) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            http_status,
        }
    }

    /// Create a service error with details
    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
        http_status: u16,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
            http_status,
        }
    }
}

/// Framework errors
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
    #[error("Internal server error: {message}")]
    Internal {
        /// The error message describing the internal error
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
            ApiError::NotFound {
                resource,
                resource_id: _,
            } => ("NOT_FOUND", format!("Resource not found: {}", resource)),
            ApiError::InvalidInput {
                message,
                field: _,
                value: _,
            } => ("INVALID_INPUT", message.clone()),
            ApiError::AuthenticationFailed { reason } => (
                "AUTHENTICATION_FAILED",
                format!("Authentication failed: {}", reason),
            ),
            ApiError::AccessDenied {
                permission,
                user_id: _,
            } => ("ACCESS_DENIED", format!("Access denied: {}", permission)),
            ApiError::RateLimitExceeded {
                limit: _,
                window_seconds: _,
            } => ("RATE_LIMIT_EXCEEDED", "Rate limit exceeded".to_string()),
            ApiError::Internal {
                message,
                error_id: _,
            } => ("INTERNAL_ERROR", message.clone()),
            ApiError::ServiceUnavailable {
                service,
                retry_after: _,
            } => (
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
            "error": {
                "code": code,
                "message": message
            }
        }))
        .unwrap_or_else(|_| {
            format!(r#"{{"success":false,"error":{{"code":"{code}","message":"{message}"}}}}"#)
        })
    }
}

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
                "Authentication failed".to_string(), // Don't expose reason to user
                serde_json::json!({ "reason": "authentication_failed" }),
                401,
            ),
            ApiError::AccessDenied {
                permission,
                user_id,
            } => ServiceError::with_details(
                "ACCESS_DENIED",
                "Access denied".to_string(), // Don't expose permission details
                serde_json::json!({ "permission": "denied", "user_id": user_id }),
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
                message: _,
                error_id,
            } => ServiceError::with_details(
                "INTERNAL_ERROR",
                "An internal error occurred".to_string(), // Sanitized - don't expose internal message
                serde_json::json!({ "error_id": error_id, "timestamp": chrono::Utc::now().timestamp() }),
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

#[cfg(feature = "http")]
impl<T> IntoResponse for ServiceResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        let status = self.error.as_ref().map(|e| e.http_status).unwrap_or(200);
        let body = serde_json::to_vec(&self).unwrap_or_default();
        axum::response::Response::builder()
            .status(status)
            .body(Body::from(body))
            .unwrap()
    }
}
