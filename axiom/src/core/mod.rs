// Copyright (c) 2026 Kirky.X
//! Core types and error handling
//!
//! This module is organized into submodules:
//! - `types`: Core type definitions like ApiMetadata
//! - `response`: Response wrappers like ServiceResponse and ServiceError
//! - `error`: Framework errors like ApiError
//! - `validation`: Request validation utilities

pub mod error;
pub mod json;
pub mod response;
pub mod str;
pub mod types;
pub mod validation;

// Re-export types from submodules for convenience
pub use error::ApiError;
pub use json::{api_metadata_response, error_response, paginated_response, success_response};
pub use response::{ServiceError, ServiceResponse};
pub use str::{
    format_empty_error, format_env_key, format_invalid_error, format_not_found, format_range_error,
    format_validation_error, sanitize_for_identifier, truncate_with_ellipsis,
};
pub use types::ApiMetadata;

/// Macro to implement Default trait via new() constructor.
///
/// # Usage
///
/// ```rust
/// use axiom::impl_default_new;
///
/// struct MyStruct {
///     value: i32,
/// }
///
/// impl MyStruct {
///     pub fn new() -> Self {
///         Self { value: 42 }
///     }
/// }
///
/// impl_default_new!(MyStruct);
///
/// // Now MyStruct implements Default
/// let _default: MyStruct = MyStruct::default();
/// ```
#[macro_export]
macro_rules! impl_default_new {
    ($type:ident) => {
        impl Default for $type {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

#[cfg(feature = "http")]
use axum::body::Body;
#[cfg(feature = "http")]
use axum::http;
#[cfg(feature = "http")]
use axum::response::IntoResponse;
use serde::Serialize;

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
