// Copyright (c) 2026 Kirky.X
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
        Err(_e) => {
            build_fallback_response(status, fallback_message)
        }
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
        let status = self.error.as_ref().map(|e| e.http_status).unwrap_or(200);

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
}
