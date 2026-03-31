// Copyright (c) 2026 Kirky.X
//! Core types and error handling
//!
//! This module is organized into submodules:
//! - `types`: Core type definitions like ApiMetadata
//! - `response`: Response wrappers like ServiceResponse and ServiceError
//! - `error`: Framework errors like ApiError
//! - `validation`: Request validation utilities
//!
//! # Note on HTTP Support
//! HTTP-specific response handling is provided in `http::response` module
//! to avoid HTTP dependencies for non-HTTP protocol implementations.

pub mod error;
pub mod json;
pub mod regex_cache;
pub mod registration;
pub mod response;
pub mod str;
pub mod types;
pub mod validation;

// Re-export types from submodules for convenience
pub use error::{ApiError, ErrorCategory, ErrorContext, SdForgeError};
pub use validation::{
    MAX_API_KEY_LENGTH, MAX_EMAIL_LENGTH, MAX_HEADER_COUNT, MAX_HEADER_NAME_LENGTH,
    MAX_HEADER_VALUE_LENGTH, MAX_JSON_ARRAY_LENGTH, MAX_JSON_DEPTH, MAX_JSON_FIELD_NAME_LENGTH,
    MAX_JWT_TOKEN_LENGTH, MAX_PASSWORD_LENGTH, MAX_QUERY_STRING_LENGTH, MAX_REQUEST_BODY_SIZE,
    MAX_TEXT_FIELD_LENGTH, MAX_URI_PATH_LENGTH, MAX_USERNAME_LENGTH, MIN_PASSWORD_LENGTH,
};
pub use json::{api_metadata_response, error_response, paginated_response, success_response};
pub use regex_cache::{common, get_regex, RegexCache, RegexCacheStats};
pub use registration::Registration;
pub use response::{ServiceError, ServiceResponse};
pub use str::{
    format_empty_error, format_env_key, format_invalid_error, format_not_found, format_range_error,
    format_validation_error, sanitize_for_identifier, truncate_with_ellipsis,
};
pub use types::ApiMetadata;
