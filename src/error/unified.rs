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
