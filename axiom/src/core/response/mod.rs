// Copyright (c) 2026 Kirky.X
//! Service response and error types
//!
//! Provides unified response wrappers and error types for the framework.

use serde::{Deserialize, Serialize};

/// Unified response wrapper
///
/// A generic response type that can represent both successful responses
/// and errors. The generic parameter T represents the type of data
/// returned on success.
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
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get reference to response data
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// Get reference to error details
    pub fn error_ref(&self) -> Option<&ServiceError> {
        self.error.as_ref()
    }

    /// Get timestamp if available
    #[cfg(feature = "timestamp")]
    pub fn timestamp(&self) -> Option<i64> {
        self.timestamp
    }
}

/// Service error representation
///
/// Represents an error that occurred during request processing.
/// Includes an error code, message, optional details, and HTTP status.
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
    pub fn new(code: impl Into<String>, message: impl Into<String>, http_status: u16) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            http_status,
        }
    }

    /// Create a service error with additional details
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
    pub fn details(&self) -> Option<&serde_json::Value> {
        self.details.as_ref()
    }

    /// Get HTTP status code
    pub fn http_status(&self) -> u16 {
        self.http_status
    }
}
