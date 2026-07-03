// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ServiceResponse::success
    #[test]
    fn test_service_response_success() {
        let response = ServiceResponse::success("test data");
        assert!(response.is_success());
        assert_eq!(response.data(), Some(&"test data"));
        assert!(response.error_ref().is_none());
    }

    /// Test ServiceResponse::error
    #[test]
    fn test_service_response_error_response() {
        let error = ServiceError::new("TEST_ERROR", "Test error message", 400);
        let response = ServiceResponse::<String>::error(error);
        assert!(!response.is_success());
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    /// Test ServiceResponse with generic type
    #[test]
    fn test_service_response_generic() {
        #[derive(Debug, Serialize, Deserialize)]
        struct User {
            name: String,
            age: u32,
        }
        let user = User {
            name: "Alice".to_string(),
            age: 30,
        };
        let response = ServiceResponse::success(user);
        assert!(response.is_success());
        let data = response.data().unwrap();
        assert_eq!(data.name, "Alice");
    }

    /// Test ServiceError::new
    #[test]
    fn test_service_error_new() {
        let error = ServiceError::new("NOT_FOUND", "Resource not found", 404);
        assert_eq!(error.code(), "NOT_FOUND");
        assert_eq!(error.message(), "Resource not found");
        assert_eq!(error.http_status(), 404);
        assert!(error.details().is_none());
    }

    /// Test ServiceError::with_details
    #[test]
    fn test_service_error_with_details() {
        let details = serde_json::json!({
            "resource": "user",
            "id": "123"
        });
        let error =
            ServiceError::with_details("VALIDATION_ERROR", "Invalid input", details.clone(), 422);
        assert_eq!(error.code(), "VALIDATION_ERROR");
        assert_eq!(error.message(), "Invalid input");
        assert_eq!(error.http_status(), 422);
        assert_eq!(error.details(), Some(&details));
    }

    /// Test ServiceError accessors
    #[test]
    fn test_service_error_accessors() {
        let error =
            ServiceError::with_details("TEST", "message", serde_json::json!({"key": "value"}), 500);
        assert_eq!(error.code(), "TEST");
        assert_eq!(error.message(), "message");
        assert_eq!(error.http_status(), 500);
        let details = error.details().unwrap();
        assert_eq!(details["key"], "value");
    }

    /// Test ServiceResponse serialization
    #[test]
    fn test_service_response_serialization() {
        let response = ServiceResponse::success("data");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"data\""));
    }

    /// Test ServiceError serialization
    #[test]
    fn test_service_error_serialization() {
        let error = ServiceError::new("ERROR_CODE", "Error message", 500);
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"code\":\"ERROR_CODE\""));
        assert!(json.contains("\"message\":\"Error message\""));
        assert!(json.contains("\"http_status\":500"));
    }

    /// Test ServiceResponse with None timestamp (without timestamp feature)
    #[test]
    fn test_service_response_no_timestamp() {
        let response = ServiceResponse::success("data");
        // When timestamp feature is disabled, the field is not available
        // We just verify the response is created successfully
        assert!(response.is_success());
        assert!(response.data().is_some());
    }

    /// Test ServiceResponse deserialization
    #[test]
    fn test_service_response_deserialization() {
        let json = r#"{"success":true,"data":"test"}"#;
        let response: ServiceResponse<String> = serde_json::from_str(json).unwrap();
        assert!(response.is_success());
        assert_eq!(response.data(), Some(&"test".to_string()));
    }

    /// Test ServiceError deserialization
    #[test]
    fn test_service_error_deserialization() {
        let json = r#"{"code":"ERR","message":"msg","http_status":400}"#;
        let error: ServiceError = serde_json::from_str(json).unwrap();
        assert_eq!(error.code(), "ERR");
        assert_eq!(error.message(), "msg");
        assert_eq!(error.http_status(), 400);
    }

    /// Test ServiceResponse error path
    #[test]
    fn test_service_response_error_details() {
        let error = ServiceError::with_details(
            "CODE",
            "message",
            serde_json::json!({"field": "value"}),
            400,
        );
        let response = ServiceResponse::<String>::error(error);
        assert!(!response.is_success());
        assert!(response.data.is_none());
        let err = response.error_ref().unwrap();
        assert_eq!(err.code(), "CODE");
    }

    /// Test ServiceError::new produces an error with no details (None).
    #[test]
    fn test_service_error_new_has_no_details() {
        let error = ServiceError::new("NOT_FOUND", "missing", 404);
        assert!(error.details().is_none());
        assert_eq!(error.details(), None);
    }

    /// Test ServiceError::with_details with null JSON value.
    #[test]
    fn test_service_error_with_null_details() {
        let error = ServiceError::with_details("ERR", "msg", serde_json::Value::Null, 500);
        assert_eq!(error.details(), Some(&serde_json::Value::Null));
    }

    /// Test ServiceResponse success then error_ref returns None.
    #[test]
    fn test_service_response_success_has_no_error() {
        let response = ServiceResponse::success("data");
        assert!(response.error_ref().is_none());
    }

    /// Test ServiceResponse error then data() returns None.
    #[test]
    fn test_service_response_error_has_no_data() {
        let error = ServiceError::new("ERR", "msg", 500);
        let response = ServiceResponse::<String>::error(error);
        assert!(response.data().is_none());
    }

    /// Test ServiceError http_status() returns the configured status code.
    #[test]
    fn test_service_error_http_status_various_codes() {
        for status in [200u16, 400, 401, 403, 404, 422, 429, 500, 503] {
            let error = ServiceError::new("CODE", "msg", status);
            assert_eq!(error.http_status(), status);
        }
    }

    /// Test ServiceResponse with a complex generic type (Vec) serializes
    /// correctly and the data field contains the array.
    #[test]
    fn test_service_response_with_vec_serialization() {
        let response = ServiceResponse::success(vec![1, 2, 3]);
        let json = serde_json::to_string(&response).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["data"], serde_json::json!([1, 2, 3]));
    }

    /// Test ServiceError serialization omits details when None (via
    /// skip_serializing_if).
    #[test]
    fn test_service_error_serialization_omits_details_when_none() {
        let error = ServiceError::new("CODE", "msg", 400);
        let json = serde_json::to_string(&error).unwrap();
        assert!(
            !json.contains("details"),
            "details should be omitted when None: {}",
            json
        );
    }

    /// Test ServiceError serialization includes details when Some.
    #[test]
    fn test_service_error_serialization_includes_details_when_some() {
        let error =
            ServiceError::with_details("CODE", "msg", serde_json::json!({"key": "value"}), 400);
        let json = serde_json::to_string(&error).unwrap();
        assert!(
            json.contains("details"),
            "details should be included when Some: {}",
            json
        );
    }

    /// Test ServiceResponse::is_success returns false for error responses.
    #[test]
    fn test_service_response_is_success_false_for_error() {
        let error = ServiceError::new("ERR", "msg", 500);
        let response = ServiceResponse::<String>::error(error);
        assert!(!response.is_success());
    }

    /// Test ServiceError Debug formatting contains the code and message.
    #[test]
    fn test_service_error_debug_format() {
        let error = ServiceError::new("DEBUG_CODE", "debug message", 418);
        let debug = format!("{:?}", error);
        assert!(debug.contains("DEBUG_CODE"));
        assert!(debug.contains("debug message"));
        assert!(debug.contains("418"));
    }
}
