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
