// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! HTTP response building utilities
//!
//! This module contains HTTP-specific response handling for Axum.
//! These functions are kept separate from core to avoid HTTP dependencies
//! for non-HTTP protocol implementations.

use axum::body::Body;
use axum::http;
use axum::response::IntoResponse;
use serde::Serialize;

use crate::core::{ApiError, ServiceResponse};

/// Build a JSON response with proper error handling and fallbacks
#[inline]
pub fn build_json_response<T: Serialize>(
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
        Err(_e) => build_fallback_response(status, fallback_message),
    }
}

/// Build a fallback response when JSON serialization fails
#[inline]
pub fn build_fallback_response(status: u16, message: &str) -> axum::response::Response {
    let escaped_message = message.replace('"', "\\\"");
    let fallback = format!(
        r#"{{"success":false,"error":{{"code":"SERIALIZATION_ERROR","message":"{}"}}}}"#,
        escaped_message
    );
    axum::response::Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(fallback))
        .unwrap_or_else(|_| axum::response::Response::new(Body::empty()))
}

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

impl<T> IntoResponse for ServiceResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        // 优先级链（LOW-3 防御性修复）：
        //   - 成功路径（error.is_none()）：status_code 字段 > 200
        //     success_with_status 设置的字段直接生效；未设置则默认 200。
        //   - 错误路径（error.is_some()）：始终使用 error.http_status，
        //     绝不读取 status_code 字段。此前实现 `status_code.or_else(|| error.http_status)`
        //     在理论上允许成功侧字段覆盖错误状态码（虽然 ServiceResponse::error
        //     构造器把 status_code 设为 None，但防御性编程要求错误路径
        //     不依赖该不变量 — 任何意外同时设置两者的代码路径都会被
        //     此处的显式分支挡住，保证错误响应始终使用错误状态码）。
        let status = if self.error.is_none() {
            self.status_code.unwrap_or(200)
        } else {
            self.error.as_ref().map(|e| e.http_status).unwrap_or(500)
        };

        if let Some(ref error) = self.error {
            let error_response = ServiceResponse::<serde_json::Value>::error(error.clone());
            build_json_response(status, &error_response, "Service error")
        } else {
            build_json_response(status, &self, "Response error")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    #[test]
    fn test_build_json_response_success() {
        #[derive(serde::Serialize)]
        struct Payload {
            value: i32,
        }

        let resp = build_json_response(200, &Payload { value: 42 }, "fallback");
        assert_eq!(resp.status(), 200);
        let content_type = resp.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_build_fallback_response_status_and_header() {
        let resp = build_fallback_response(500, "error");
        assert_eq!(resp.status(), 500);
        let content_type = resp.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_api_error_into_response_status_mapping() {
        let resp = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("1".to_string()),
        }
        .into_response();
        assert_eq!(resp.status(), 404);
        let resp = ApiError::InvalidInput {
            message: "reason".to_string(),
            field: Some("field".to_string()),
            value: None,
        }
        .into_response();
        assert_eq!(resp.status(), 400);
    }

    #[test]
    fn test_service_response_into_response_success() {
        let resp = ServiceResponse::success("ok").into_response();
        assert_eq!(resp.status(), 200);
    }

    #[test]
    fn test_service_response_into_response_error_status() {
        let err = crate::core::ServiceError::with_details(
            "CODE",
            "message",
            serde_json::json!({"k":"v"}),
            418,
        );
        let resp = ServiceResponse::<String>::error(err).into_response();
        assert_eq!(resp.status(), 418);
    }

    /// Test: all ApiError variants map to the correct HTTP status code.
    /// Covers the previously-uncovered match arms (AuthenticationFailed,
    /// AccessDenied, RateLimitExceeded, Internal, ServiceUnavailable,
    /// ValidationError).
    #[test]
    fn test_api_error_all_variants_status_mapping() {
        let cases: Vec<(u16, ApiError)> = vec![
            (
                401,
                ApiError::AuthenticationFailed {
                    reason: "bad token".to_string(),
                },
            ),
            (
                403,
                ApiError::AccessDenied {
                    permission: "read".to_string(),
                    user_id: None,
                },
            ),
            (
                429,
                ApiError::RateLimitExceeded {
                    limit: 100,
                    window_seconds: 60,
                },
            ),
            (
                500,
                ApiError::Internal {
                    message: "boom".to_string(),
                    error_id: "err-1".to_string(),
                    source: None,
                    context: None,
                },
            ),
            (
                503,
                ApiError::ServiceUnavailable {
                    service: "downstream".to_string(),
                    retry_after: Some(10),
                    source: None,
                },
            ),
            (
                422,
                ApiError::ValidationError {
                    field: "email".to_string(),
                    constraint: "invalid format".to_string(),
                },
            ),
        ];
        for (expected_status, err) in cases {
            let resp = err.into_response();
            assert_eq!(
                resp.status(),
                axum::http::StatusCode::from_u16(expected_status).unwrap(),
                "ApiError variant should map to HTTP {}",
                expected_status
            );
        }
    }

    /// Test: build_json_response falls back when serialization fails.
    /// Covers the `Err(_e)` branch by passing a value whose `Serialize`
    /// implementation returns an error.
    #[test]
    fn test_build_json_response_serialization_failure_fallback() {
        use serde::ser::{self, Serialize, Serializer};

        /// A type that always fails to serialize.
        struct Unserializable;
        impl Serialize for Unserializable {
            fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
                Err(ser::Error::custom("intentional serialization failure"))
            }
        }

        let resp = build_json_response(200, &Unserializable, "fallback message");
        // Should fall back to a 200 response with the fallback body.
        assert_eq!(resp.status(), 200);
        let content_type = resp.headers().get(header::CONTENT_TYPE).unwrap();
        assert_eq!(content_type, "application/json");
    }

    /// Test: build_fallback_response escapes embedded double quotes in the
    /// message to keep the emitted JSON valid.
    #[test]
    fn test_build_fallback_response_escapes_quotes() {
        let resp = build_fallback_response(400, r#"bad "value" here"#);
        assert_eq!(resp.status(), 400);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    // ============================================================================
    // Body content verification tests
    //
    // These tests verify the actual JSON body content of responses, not just
    // status codes and headers.
    // ============================================================================

    #[tokio::test]
    async fn test_build_json_response_body_content() {
        #[derive(serde::Serialize)]
        struct Payload {
            name: String,
            count: i32,
        }

        let resp = build_json_response(
            201,
            &Payload {
                name: "test".to_string(),
                count: 5,
            },
            "fallback",
        );
        assert_eq!(resp.status(), 201);

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["count"], 5);
    }

    #[tokio::test]
    async fn test_build_fallback_response_body_content() {
        let resp = build_fallback_response(500, "something went wrong");
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["success"], false);
        assert_eq!(parsed["error"]["code"], "SERIALIZATION_ERROR");
        assert_eq!(parsed["error"]["message"], "something went wrong");
    }

    #[tokio::test]
    async fn test_build_fallback_response_empty_message() {
        let resp = build_fallback_response(400, "");
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["message"], "");
    }

    #[tokio::test]
    async fn test_build_fallback_response_escaped_body_valid_json() {
        // Verify that escaped quotes produce valid JSON
        let resp = build_fallback_response(400, r#"bad "value" here"#);
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        // Should parse without error
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["message"], r#"bad "value" here"#);
    }

    #[tokio::test]
    async fn test_api_error_response_body_contains_error_info() {
        let resp = ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some("42".to_string()),
        }
        .into_response();

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        // The response should contain error information
        assert!(parsed.is_object());
    }

    #[tokio::test]
    async fn test_service_response_success_body_content() {
        let resp = ServiceResponse::success("hello").into_response();
        assert_eq!(resp.status(), 200);

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["data"], "hello");
        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    }

    #[tokio::test]
    async fn test_service_response_error_body_content() {
        let err = crate::core::ServiceError::with_details(
            "CUSTOM_CODE",
            "custom error message",
            serde_json::json!({"detail": "info"}),
            451,
        );
        let resp = ServiceResponse::<String>::error(err).into_response();
        assert_eq!(resp.status(), 451);

        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["code"], "CUSTOM_CODE");
    }

    #[test]
    fn test_build_json_response_with_various_status_codes() {
        #[derive(serde::Serialize)]
        struct Empty;
        for status in [
            200u16, 201, 204, 301, 400, 401, 403, 404, 422, 429, 500, 503,
        ] {
            let resp = build_json_response(status, &Empty, "fallback");
            assert_eq!(
                resp.status(),
                axum::http::StatusCode::from_u16(status).unwrap(),
                "Status code {} should be preserved",
                status
            );
        }
    }

    #[tokio::test]
    async fn test_build_json_response_serialization_failure_uses_fallback_message() {
        use serde::ser::{self, Serialize, Serializer};

        struct Unserializable;
        impl Serialize for Unserializable {
            fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
                Err(ser::Error::custom("intentional failure"))
            }
        }

        let resp = build_json_response(422, &Unserializable, "custom fallback message");
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"]["message"], "custom fallback message");
        assert_eq!(parsed["error"]["code"], "SERIALIZATION_ERROR");
    }

    // ============================================================================
    // unwrap_or_else fallback branch coverage
    //
    // The `unwrap_or_else` closures in build_json_response (line 27) and
    // build_fallback_response (line 44) are defensive fallbacks triggered when
    // `Response::builder().body()` returns Err. This happens when the status
    // code is invalid (outside the 100..=999 range accepted by
    // `StatusCode::TryFrom<u16>`). Passing a status < 100 (e.g., 99) causes
    // the builder to store an error internally, making `.body()` return Err.
    // ============================================================================

    /// Test build_json_response falls back when given an invalid status code
    /// (< 100). Covers the `unwrap_or_else(|_| build_fallback_response(...))`
    /// branch in build_json_response.
    #[test]
    fn test_build_json_response_invalid_status_triggers_fallback() {
        #[derive(serde::Serialize)]
        struct Payload {
            value: i32,
        }

        // Status 99 is invalid (< 100), causing Response::builder().body() to
        // return Err, which triggers the unwrap_or_else fallback.
        let resp = build_json_response(99, &Payload { value: 42 }, "fallback for invalid status");
        // The response should still be created via the fallback path.
        // The fallback itself also receives the invalid status, so it too
        // falls back to Response::new(Body::empty()).
        // Just verify we get a Response without panic.
        let _ = resp.status();
    }

    /// Test build_fallback_response falls back to an empty body when given an
    /// invalid status code (< 100). Covers the
    /// `unwrap_or_else(|_| axum::response::Response::new(Body::empty()))`
    /// branch in build_fallback_response.
    #[test]
    fn test_build_fallback_response_invalid_status_triggers_empty_body() {
        // Status 99 is invalid, causing both build_json_response and
        // build_fallback_response to hit their unwrap_or_else fallbacks.
        let resp = build_fallback_response(99, "invalid status test");
        // The final fallback is Response::new(Body::empty()), which defaults
        // to status 200. Verify no panic occurs.
        let _ = resp.status();
    }

    /// Test build_json_response with a very large invalid status code (> 999)
    /// also triggers the fallback path.
    #[test]
    fn test_build_json_response_status_above_999_triggers_fallback() {
        #[derive(serde::Serialize)]
        struct Payload {
            value: i32,
        }

        // Status 1000 is invalid (> 999), triggering the fallback.
        let resp = build_json_response(1000, &Payload { value: 42 }, "overflow status");
        let _ = resp.status();
    }

    // ============================================================================
    // forge-success-status-code: ServiceResponse::into_response status_code 优先级
    //
    // R-http-protocol-001: 成功侧 status_code 字段优先
    // R-http-protocol-002: 错误侧不回归
    // R-http-protocol-003: 默认 200 零破坏
    // ============================================================================

    /// R-http-protocol-001: success_with_status("x", 201) → HTTP 201。
    #[test]
    fn test_service_response_into_response_with_status_code() {
        let resp = ServiceResponse::success_with_status("x", 201).into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::CREATED);
    }

    /// R-http-protocol-001: 边界码 100/999 也能正确传递。
    #[test]
    fn test_service_response_into_response_status_code_boundaries() {
        let resp = ServiceResponse::success_with_status("x", 100).into_response();
        assert_eq!(
            resp.status(),
            axum::http::StatusCode::from_u16(100).unwrap()
        );
        let resp = ServiceResponse::success_with_status("x", 999).into_response();
        assert_eq!(
            resp.status(),
            axum::http::StatusCode::from_u16(999).unwrap()
        );
    }

    /// R-http-protocol-002: 错误侧仍按 ServiceError.http_status 取值（不回归）。
    #[test]
    fn test_service_response_into_response_error_status_no_regression() {
        let err = crate::core::ServiceError::new("E", "m", 418);
        let resp = ServiceResponse::<String>::error(err).into_response();
        assert_eq!(
            resp.status(),
            axum::http::StatusCode::from_u16(418).unwrap()
        );
    }

    /// R-http-protocol-003: 无 status_code 字段且无 error → 200（零破坏）。
    #[test]
    fn test_service_response_into_response_default_200_no_regression() {
        let resp = ServiceResponse::success("x").into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }
}
