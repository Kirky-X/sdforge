// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Unified error contract.
//!
//! All protocol error responses share one structure — error code, message,
//! and the ambient request `trace_id` (from the request context when the
//! `context` feature is on). This collapses the historical HTTP-400 vs
//! gRPC-422 divergence (SIMPL-001) onto a single payload shape with a
//! per-protocol status mapping table.
//!
//! ```json
//! {"code":"INVALID_INPUT","message":"...","trace_id":"trace-...","field":"age"}
//! ```

use serde::Serialize;

/// Unified, protocol-neutral error payload.
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedError {
    /// Stable machine-readable error code (e.g. `INVALID_INPUT`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Ambient trace id from the request context, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Offending field for validation-type errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl UnifiedError {
    /// Build an error payload with the ambient trace id attached.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            trace_id: current_trace_id(),
            field: None,
        }
    }

    /// Attach an offending field.
    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    /// Render as a JSON value.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Ambient trace id from the request context, when the `context`
/// feature is enabled and a context is installed.
pub fn current_trace_id() -> Option<String> {
    #[cfg(feature = "context")]
    {
        crate::context::current().map(|ctx| ctx.trace_id().to_string())
    }
    #[cfg(not(feature = "context"))]
    {
        None
    }
}

/// Default error-code mapping per [`crate::error::api_error::ApiError`]
/// category (HTTP status → code). Shared by protocol adapters so one table
/// drives every protocol's payload.
pub fn code_for_http_status(status: u16) -> &'static str {
    match status {
        400 => "BAD_REQUEST",
        401 => "UNAUTHORIZED",
        403 => "FORBIDDEN",
        404 => "NOT_FOUND",
        409 => "CONFLICT",
        422 => "UNPROCESSABLE_ENTITY",
        429 => "TOO_MANY_REQUESTS",
        500 => "INTERNAL",
        503 => "UNAVAILABLE",
        _ => "ERROR",
    }
}

/// HTTP 状态的直读入口（T004 热路径去重）：单次 `to_service_error` 构造。
///
/// 状态真值仍是 `to_service_error().http_status()` —— 本函数只是避免调用方
/// 为拿一个 u16 而完整构造（含 details JSON 分配）。
pub fn http_status_for(e: &crate::core::ApiError) -> u16 {
    e.to_service_error().http_status()
}

/// **Single source of truth** mapping an [`ApiError`] to its
/// `(HTTP status, machine-readable code)` pair.
///
/// Every protocol adapter (HTTP `IntoResponse`, gRPC status mapping, …)
/// must derive its wire codes from here — never from a private match table.
/// The HTTP status matches `ApiError::to_service_error().http_status`; the
/// code string is derived via [`code_for_http_status`].
pub fn mapping_for(e: &crate::core::ApiError) -> (u16, &'static str) {
    let status = http_status_for(e);
    (status, code_for_http_status(status))
}

impl From<&crate::core::ApiError> for UnifiedError {
    fn from(e: &crate::core::ApiError) -> Self {
        // 单次构造：status/code/message 全部取自同一个 ServiceError。
        let svc = e.to_service_error();
        let status = svc.http_status();
        let message = svc.message().to_string();
        let field = match e {
            crate::core::ApiError::InvalidInput { field: Some(f), .. }
            | crate::core::ApiError::ValidationError { field: f, .. } => Some(f.clone()),
            _ => None,
        };
        Self {
            code: code_for_http_status(status).to_string(),
            message,
            trace_id: current_trace_id(),
            field,
        }
    }
}

/// **Single source of truth** mapping an [`ApiError`] to its
/// [`tonic::Code`] (gRPC status code). Companion to [`mapping_for`] on the
/// gRPC wire.
#[cfg(feature = "grpc")]
pub fn grpc_code_for(e: &crate::core::ApiError) -> tonic::Code {
    use crate::core::ApiError;
    match e {
        ApiError::NotFound { .. } => tonic::Code::NotFound,
        ApiError::InvalidInput { .. } | ApiError::ValidationError { .. } => {
            tonic::Code::InvalidArgument
        }
        ApiError::AuthenticationFailed { .. } => tonic::Code::Unauthenticated,
        ApiError::AccessDenied { .. } => tonic::Code::PermissionDenied,
        ApiError::RateLimitExceeded { .. } | ApiError::QuotaExhausted { .. } => {
            tonic::Code::ResourceExhausted
        }
        ApiError::ServiceUnavailable { .. } => tonic::Code::Unavailable,
        ApiError::Internal { .. } => tonic::Code::Internal,
    }
}

#[cfg(feature = "http")]
mod http_render {
    use super::UnifiedError;
    use axum::response::{IntoResponse, Response};

    /// Render a unified error as an HTTP response (payload shape shared by
    /// all protocols; status drives the code via `code_for_http_status`).
    pub fn to_response(status: axum::http::StatusCode, err: &UnifiedError) -> Response {
        (status, axum::Json(err.to_json())).into_response()
    }
}

#[cfg(feature = "http")]
pub use http_render::to_response as render_http;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ApiError;

    /// T004: http_status_for / mapping_for / to_service_error 三方一致。
    #[test]
    fn http_status_for_consistent_with_service_error() {
        let variants = vec![
            ApiError::NotFound {
                resource: "x".into(),
                resource_id: None,
            },
            ApiError::InvalidInput {
                message: "x".into(),
                field: None,
                value: None,
            },
            ApiError::ValidationError {
                field: "f".into(),
                constraint: "c".into(),
            },
            ApiError::AuthenticationFailed { reason: "x".into() },
            ApiError::AccessDenied {
                permission: "p".into(),
                user_id: None,
            },
            ApiError::RateLimitExceeded {
                limit: 1,
                window_seconds: 1,
            },
            ApiError::QuotaExhausted { used: 1, total: 2 },
            ApiError::ServiceUnavailable {
                service: "s".into(),
                retry_after: None,
                source: None,
            },
            ApiError::Internal {
                message: "x".into(),
                error_id: "e".into(),
                source: None,
                context: None,
            },
        ];
        for e in variants {
            let direct = e.to_service_error().http_status();
            assert_eq!(http_status_for(&e), direct);
            assert_eq!(mapping_for(&e).0, direct);
        }
    }

    /// 统一映射表（单一事实来源）— 全变体断言。
    /// HTTP 状态与既有 `to_service_error` 的 http_status 一致；
    /// code 字符串走 `code_for_http_status` 派生。
    #[test]
    fn mapping_for_covers_all_variants() {
        let cases: Vec<(ApiError, u16, &str)> = vec![
            (
                ApiError::NotFound {
                    resource: "User".into(),
                    resource_id: None,
                },
                404,
                "NOT_FOUND",
            ),
            (
                ApiError::InvalidInput {
                    message: "x".into(),
                    field: Some("age".into()),
                    value: None,
                },
                400,
                "BAD_REQUEST",
            ),
            (
                ApiError::ValidationError {
                    field: "email".into(),
                    constraint: "format".into(),
                },
                422,
                "UNPROCESSABLE_ENTITY",
            ),
            (
                ApiError::AuthenticationFailed { reason: "x".into() },
                401,
                "UNAUTHORIZED",
            ),
            (
                ApiError::AccessDenied {
                    permission: "read".into(),
                    user_id: None,
                },
                403,
                "FORBIDDEN",
            ),
            (
                ApiError::RateLimitExceeded {
                    limit: 10,
                    window_seconds: 60,
                },
                429,
                "TOO_MANY_REQUESTS",
            ),
            (
                ApiError::QuotaExhausted { used: 1, total: 2 },
                429,
                "TOO_MANY_REQUESTS",
            ),
            (
                ApiError::ServiceUnavailable {
                    service: "db".into(),
                    retry_after: None,
                    source: None,
                },
                503,
                "UNAVAILABLE",
            ),
            (
                ApiError::Internal {
                    message: "boom".into(),
                    error_id: "e1".into(),
                    source: None,
                    context: None,
                },
                500,
                "INTERNAL",
            ),
        ];
        for (e, status, code) in cases {
            let (got_status, got_code) = mapping_for(&e);
            assert_eq!(got_status, status, "status mismatch for {e:?}");
            assert_eq!(got_code, code, "code mismatch for {e:?}");
        }
    }

    /// From<&ApiError> 字段提取：ValidationError/InvalidInput 带 field；
    /// Internal 的 message 必须脱敏（不携带构造期原始内部消息）。
    #[test]
    fn unified_error_from_api_error_extracts_fields() {
        let validation = ApiError::ValidationError {
            field: "email".into(),
            constraint: "format".into(),
        };
        let u = UnifiedError::from(&validation);
        assert_eq!(u.field.as_deref(), Some("email"));
        assert_eq!(u.code, "UNPROCESSABLE_ENTITY");

        let invalid = ApiError::InvalidInput {
            message: "bad".into(),
            field: Some("age".into()),
            value: None,
        };
        let u = UnifiedError::from(&invalid);
        assert_eq!(u.field.as_deref(), Some("age"));
        assert_eq!(u.code, "BAD_REQUEST");

        let internal = ApiError::Internal {
            message: "secret-host-db-path".into(),
            error_id: "e1".into(),
            source: None,
            context: None,
        };
        let u = UnifiedError::from(&internal);
        assert_eq!(u.code, "INTERNAL");
        assert!(u.field.is_none());
        assert!(
            !u.message.contains("secret-host-db-path"),
            "internal message must be sanitized, got: {}",
            u.message
        );
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn grpc_code_for_covers_all_variants() {
        use tonic::Code;
        let cases: Vec<(ApiError, Code)> = vec![
            (
                ApiError::NotFound {
                    resource: "x".into(),
                    resource_id: None,
                },
                Code::NotFound,
            ),
            (
                ApiError::InvalidInput {
                    message: "x".into(),
                    field: None,
                    value: None,
                },
                Code::InvalidArgument,
            ),
            (
                ApiError::ValidationError {
                    field: "f".into(),
                    constraint: "c".into(),
                },
                Code::InvalidArgument,
            ),
            (
                ApiError::AuthenticationFailed { reason: "x".into() },
                Code::Unauthenticated,
            ),
            (
                ApiError::AccessDenied {
                    permission: "p".into(),
                    user_id: None,
                },
                Code::PermissionDenied,
            ),
            (
                ApiError::RateLimitExceeded {
                    limit: 1,
                    window_seconds: 1,
                },
                Code::ResourceExhausted,
            ),
            (
                ApiError::QuotaExhausted { used: 1, total: 2 },
                Code::ResourceExhausted,
            ),
            (
                ApiError::ServiceUnavailable {
                    service: "s".into(),
                    retry_after: None,
                    source: None,
                },
                Code::Unavailable,
            ),
            (
                ApiError::Internal {
                    message: "x".into(),
                    error_id: "e".into(),
                    source: None,
                    context: None,
                },
                Code::Internal,
            ),
        ];
        for (e, code) in cases {
            assert_eq!(grpc_code_for(&e), code, "grpc code mismatch for {e:?}");
        }
    }

    #[test]
    fn unified_error_serializes_code_and_message() {
        let mut err = UnifiedError::new("INVALID_INPUT", "bad value");
        err = err.with_field("age");
        let json = err.to_json();
        assert_eq!(json["code"], "INVALID_INPUT");
        assert_eq!(json["message"], "bad value");
        assert_eq!(json["field"], "age");
    }

    #[test]
    fn trace_id_absent_without_context() {
        // Without the `context` feature (or outside a scope) no trace id is
        // attached; the field is omitted from the payload.
        let err = UnifiedError::new("FORBIDDEN", "missing role");
        let json = err.to_json();
        if err.trace_id.is_none() {
            assert!(json.get("trace_id").is_none());
        }
    }

    #[test]
    fn http_status_codes_map_to_stable_codes() {
        assert_eq!(code_for_http_status(400), "BAD_REQUEST");
        assert_eq!(code_for_http_status(401), "UNAUTHORIZED");
        assert_eq!(code_for_http_status(403), "FORBIDDEN");
        assert_eq!(code_for_http_status(429), "TOO_MANY_REQUESTS");
        assert_eq!(code_for_http_status(500), "INTERNAL");
        assert_eq!(code_for_http_status(418), "ERROR");
    }

    #[cfg(all(feature = "http", feature = "context"))]
    #[tokio::test]
    async fn trace_id_flows_from_request_context() {
        crate::context::scope(
            crate::context::RequestContext::with_ids("r".into(), "trace-42".into()),
            async {
                let err = UnifiedError::new("FORBIDDEN", "nope");
                assert_eq!(err.trace_id.as_deref(), Some("trace-42"));
                assert_eq!(err.to_json()["trace_id"], "trace-42");
            },
        )
        .await;
    }
}
