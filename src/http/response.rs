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
            #[cfg(feature = "logging")]
            tracing::error!("Failed to serialize response");

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
            // Create a error response with the same error
            let error_response = ServiceResponse::<serde_json::Value>::error(error.clone());
            build_json_response(status, &error_response, "Service error")
        } else {
            build_json_response(status, &self, "Response error")
        }
    }
}
