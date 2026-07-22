// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;

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
            status_code: None,
            #[cfg(feature = "timestamp")]
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// Create a successful response with an explicit success status code.
    ///
    /// Dynamic entry point for runtime-decided status codes (upsert 201/200,
    /// conditional requests 304, async tasks 202, etc.). The `code` is stored
    /// in `status_code` and takes precedence over any macro-level `status`
    /// argument at response-build time (see `with_status_code_opt`).
    ///
    /// LOW-4: `debug_assert` enforces the HTTP status code range `100..=999`
    /// in debug builds. Out-of-range codes are a programmer error (the macro
    /// `status` argument is range-checked at compile time; this guard catches
    /// runtime callers that bypass the macro). Release builds keep the value
    /// as-is to avoid breaking forward compatibility with future HTTP
    /// extensions, but the assertion makes bugs visible during development.
    pub fn success_with_status(data: T, code: u16) -> Self {
        debug_assert!(
            (100..=999).contains(&code),
            "success_with_status code must be in 100..=999, got {}",
            code
        );
        Self {
            success: true,
            data: Some(data),
            error: None,
            status_code: Some(code),
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
            status_code: None,
            #[cfg(feature = "timestamp")]
            timestamp: Some(chrono::Utc::now().timestamp()),
        }
    }

    /// Returns the success-side status code, if any.
    ///
    /// `None` means the status code is decided by the macro `status` argument
    /// (for bare return types) or defaults to 200. `Some(code)` means the
    /// response should be built with that code, overriding the macro default.
    pub fn status_code(&self) -> Option<u16> {
        self.status_code
    }

    /// Merge a fallback status code into the response when none is set.
    ///
    /// Used by the `#[forge]` macro handler: the macro `status` argument is
    /// passed here as the fallback, and an explicit `success_with_status`
    /// value already on the response takes precedence (field > macro > 200).
    /// Only fills the field when `self.status_code.is_none()`; passing
    /// `None` is a no-op (leaves the field as-is).
    pub fn with_status_code_opt(mut self, code: Option<u16>) -> Self {
        if self.status_code.is_none() {
            self.status_code = code;
        }
        self
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
