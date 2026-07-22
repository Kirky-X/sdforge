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
    /// 成功响应的 HTTP/gRPC 状态码；None 表示由宏 `status` 参数或默认 200 决定。
    /// `#[serde(default)]` + `skip_serializing_if` 保证反序列化向后兼容、
    /// 序列化无该键时输出与现状逐字节一致（零破坏）。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub(crate) status_code: Option<u16>,
    /// Response timestamp
    #[cfg(feature = "timestamp")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timestamp: Option<i64>,
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

mod response_impl;

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

    // ============================================================================
    // forge-success-status-code: status_code field + constructors
    //
    // R-core-response-001: 字段与零破坏序列化
    // R-core-response-002: success_with_status 动态构造器
    // R-core-response-003: with_status_code_opt 合并语义（字段优先）
    // R-core-response-004: status_code 访问器
    // ============================================================================

    /// R-core-response-001: success("x") 序列化结果不含 status_code 键。
    #[test]
    fn test_status_code_field_absent_on_success() {
        let response = ServiceResponse::success("x");
        let json = serde_json::to_string(&response).unwrap();
        assert!(
            !json.contains("status_code"),
            "status_code should be omitted when None (zero-breaking): {}",
            json
        );
    }

    /// R-core-response-001: 反序列化历史 JSON（无 status_code 键）成功且字段为 None。
    #[test]
    fn test_status_code_field_backward_compatible_deserialization() {
        let json = r#"{"success":true,"data":"x"}"#;
        let response: ServiceResponse<String> = serde_json::from_str(json).unwrap();
        assert!(response.is_success());
        assert_eq!(response.data(), Some(&"x".to_string()));
        assert_eq!(response.status_code(), None);
    }

    /// R-core-response-002: success_with_status 构造 success=true 且 status_code=Some(code)。
    #[test]
    fn test_success_with_status_sets_field() {
        let response = ServiceResponse::success_with_status("x", 201);
        assert!(response.is_success());
        assert_eq!(response.status_code(), Some(201));
        assert_eq!(response.data(), Some(&"x"));
    }

    /// R-core-response-002: code 取合法边界 100、999 时正常构造。
    #[test]
    fn test_success_with_status_boundary_codes() {
        for code in [100u16, 999] {
            let response = ServiceResponse::success_with_status("x", code);
            assert_eq!(response.status_code(), Some(code));
        }
    }

    /// R-core-response-003: with_status_code_opt 在字段 None 时填入。
    #[test]
    fn test_with_status_code_opt_fills_when_none() {
        let response = ServiceResponse::success("x").with_status_code_opt(Some(201));
        assert_eq!(response.status_code(), Some(201));
    }

    /// R-core-response-003: 字段优先 — 已有值时不被 with_status_code_opt 覆盖。
    #[test]
    fn test_with_status_code_opt_does_not_overwrite_existing() {
        let response = ServiceResponse::success_with_status("x", 200)
            .with_status_code_opt(Some(201));
        assert_eq!(
            response.status_code(),
            Some(200),
            "field-set status_code must take precedence over macro fallback"
        );
    }

    /// R-core-response-003: with_status_code_opt(None) 不改字段（None 不改）。
    #[test]
    fn test_with_status_code_opt_none_is_noop() {
        let response = ServiceResponse::success("x").with_status_code_opt(None);
        assert_eq!(response.status_code(), None);
    }

    /// R-core-response-004: success("x").status_code() == None。
    #[test]
    fn test_status_code_accessor_none_on_success() {
        assert_eq!(ServiceResponse::success("x").status_code(), None);
    }

    /// R-core-response-004: success_with_status("x", 201).status_code() == Some(201)。
    #[test]
    fn test_status_code_accessor_some_on_success_with_status() {
        assert_eq!(
            ServiceResponse::success_with_status("x", 201).status_code(),
            Some(201)
        );
    }

    /// 错误响应的 status_code 也应为 None（错误侧走 ServiceError.http_status）。
    #[test]
    fn test_status_code_none_on_error_response() {
        let err = ServiceError::new("ERR", "msg", 500);
        let response = ServiceResponse::<String>::error(err);
        assert_eq!(response.status_code(), None);
    }

    /// success_with_status 序列化包含 status_code 键（与 success 的零破坏行为对比）。
    #[test]
    fn test_success_with_status_serializes_status_code() {
        let response = ServiceResponse::success_with_status("x", 201);
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status_code\":201"), "json: {}", json);
    }

    /// 反序列化含 status_code 的 JSON 时字段正确还原。
    #[test]
    fn test_status_code_deserialization_roundtrip() {
        let json = r#"{"success":true,"data":"x","status_code":201}"#;
        let response: ServiceResponse<String> = serde_json::from_str(json).unwrap();
        assert_eq!(response.status_code(), Some(201));
    }
}
