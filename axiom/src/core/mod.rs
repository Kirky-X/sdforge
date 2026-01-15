//! Core types and error handling

pub mod validation;

#[cfg(feature = "http")]
use axum::body::Body;
#[cfg(feature = "http")]
use axum::http;
#[cfg(feature = "http")]
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API metadata (protocol-agnostic)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiMetadata {
    /// API name
    pub(crate) name: String,
    /// API version
    pub(crate) version: String,
    /// API description
    pub(crate) description: String,
    /// Cache TTL in seconds (None means no caching)
    pub(crate) cache_ttl: Option<u64>,
    /// Whether this is a streaming endpoint
    pub(crate) is_streaming: bool,
}

impl ApiMetadata {
    /// Create new API metadata
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the API endpoint
    /// * `version` - The version string (e.g., "v1")
    /// * `description` - Human-readable description of the API
    /// * `cache_ttl` - Optional cache TTL in seconds (None means no caching)
    /// * `is_streaming` - Whether this is a streaming endpoint (SSE, WebSocket, etc.)
    pub fn new(
        name: String,
        version: String,
        description: String,
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

    /// Get API name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get API version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get API description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get cache TTL
    ///
    /// Returns the cache TTL in seconds, or None if caching is disabled.
    pub fn cache_ttl(&self) -> Option<u64> {
        self.cache_ttl
    }

    /// Check if this is a streaming endpoint
    pub fn is_streaming(&self) -> bool {
        self.is_streaming
    }
}

/// Unified response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceResponse<T = serde_json::Value> {
    /// Whether the request was successful
    pub(crate) success: bool,
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<T>,
    /// Error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<ServiceError>,
    /// Response timestamp
    #[cfg(feature = "timestamp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timestamp: Option<i64>,
}

impl<T> ServiceResponse<T>
where
    T: Serialize,
{
    /// Create a successful response
    ///
    /// # Arguments
    ///
    /// * `data` - The response data to include
    ///
    /// # Returns
    ///
    /// A new `ServiceResponse` with `success: true` and the provided data
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
    ///
    /// Note: The generic parameter T is required by the struct definition but is not used
    /// for error responses (data is always None). Error responses typically use the default
    /// T = serde_json::Value.
    ///
    /// # Arguments
    ///
    /// * `error` - The error details to include
    ///
    /// # Returns
    ///
    /// A new `ServiceResponse` with `success: false` and the provided error
    pub fn error(error: ServiceError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            #[cfg(feature = "timestamp")]
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// Check if the response is successful
    ///
    /// # Returns
    ///
    /// `true` if the response represents a successful operation, `false` otherwise
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get reference to response data
    ///
    /// # Returns
    ///
    /// A reference to the response data if present, `None` otherwise
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Get reference to error details
    ///
    /// # Returns
    ///
    /// A reference to the error details if present, `None` otherwise
    pub fn error_ref(&self) -> Option<&ServiceError> {
        self.error.as_ref()
    }

    /// Get timestamp if available
    ///
    /// # Returns
    ///
    /// The response timestamp in Unix epoch seconds, or `None` if the timestamp feature is disabled
    #[cfg(feature = "timestamp")]
    pub fn timestamp(&self) -> Option<i64> {
        self.timestamp
    }
}

/// Service error representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceError {
    /// Error code
    pub(crate) code: String,
    /// Error message
    pub(crate) message: String,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<serde_json::Value>,
    /// HTTP status code
    pub(crate) http_status: u16,
}

impl ServiceError {
    /// Create a new service error
    ///
    /// # Arguments
    ///
    /// * `code` - Error code identifier (e.g., "NOT_FOUND", "INVALID_INPUT")
    /// * `message` - Human-readable error message
    /// * `http_status` - HTTP status code to return (e.g., 404, 400, 500)
    pub fn new(code: impl Into<String>, message: impl Into<String>, http_status: u16) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            http_status,
        }
    }

    /// Create a service error with additional details
    ///
    /// # Arguments
    ///
    /// * `code` - Error code identifier
    /// * `message` - Human-readable error message
    /// * `details` - Additional error details as JSON value
    /// * `http_status` - HTTP status code to return
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

    /// Get error code
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Get error message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get error details
    ///
    /// # Returns
    ///
    /// A reference to the error details if present, `None` otherwise
    pub fn details(&self) -> Option<&serde_json::Value> {
        self.details.as_ref()
    }

    /// Get HTTP status code
    ///
    /// # Returns
    ///
    /// The HTTP status code associated with this error
    pub fn http_status(&self) -> u16 {
        self.http_status
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
            ApiError::AuthenticationFailed { reason: _ } => ServiceError::with_details(
                "AUTHENTICATION_FAILED",
                "Authentication failed".to_string(), // Don't expose reason to user
                serde_json::json!({ "reason": "authentication_failed" }),
                401,
            ),
            ApiError::AccessDenied {
                permission: _,
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

/// Implement IntoResponse for ApiError to enable direct return in HTTP handlers
/// Build a JSON response with proper error handling and fallbacks
#[inline]
#[cfg(feature = "http")]
fn build_json_response<T: Serialize>(
    status: u16,
    body: &T,
    fallback_message: &str,
) -> axum::response::Response {
    match serde_json::to_vec(body) {
        Ok(body_bytes) => axum::response::Response::builder()
            .status(status)
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body_bytes))
            .unwrap_or_else(|_| build_fallback_response(status, fallback_message)),
        Err(e) => {
            #[cfg(feature = "logging")]
            tracing::error!(error = %e, "Failed to serialize response");

            build_fallback_response(status, fallback_message)
        }
    }
}

/// Build a fallback response when JSON serialization fails
#[inline]
#[cfg(feature = "http")]
fn build_fallback_response(status: u16, message: &str) -> axum::response::Response {
    let fallback = format!(
        r#"{{"success":false,"error":{{"code":"SERIALIZATION_ERROR","message":"{}"}}}}"#,
        message
    );
    axum::response::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(fallback))
        .unwrap_or_else(|_| axum::response::Response::new(Body::empty()))
}

#[cfg(feature = "http")]
impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            ApiError::NotFound { .. } => 404,
            ApiError::InvalidInput { .. } => 400,
            ApiError::AuthenticationFailed { .. } => 401,
            ApiError::AccessDenied { .. } => 403,
            ApiError::RateLimitExceeded { .. } => 429,
            ApiError::Internal { .. } => 500,
            ApiError::ServiceUnavailable { .. } => 503,
            ApiError::ValidationError { .. } => 422,
        };

        build_json_response(status, &self, "Internal server error")
    }
}

#[cfg(feature = "http")]
impl<T> IntoResponse for ServiceResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        let status = self.error.as_ref().map(|e| e.http_status).unwrap_or(200);

        if let Some(ref error) = self.error {
            // Create a error response with the same error
            let error_response = ServiceResponse::<serde_json::Value>::error(error.clone());
            build_json_response(status, &error_response, "Service error")
        } else {
            build_json_response(status, &self, "Response error")
        }
    }
}
