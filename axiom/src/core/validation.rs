//! Parameter validation and type conversion utilities
//!
//! This module provides utilities for validating request parameters and
//! converting between different types. Requires the `http` feature.

#[cfg(feature = "http")]
use serde::Deserialize;
#[cfg(feature = "http")]
use thiserror::Error;
#[cfg(feature = "http")]
use validator::{Validate, ValidationErrors};

#[cfg(feature = "http")]
/// Parameter validation errors
#[derive(Debug, Error, Clone)]
#[error("Validation failed: {errors:?}")]
pub struct ValidationErrorsWrapper {
    /// Validation errors
    pub errors: Vec<FieldValidationError>,
}

#[cfg(feature = "http")]
impl ValidationErrorsWrapper {
    /// Create new validation errors wrapper
    pub fn new(errors: Vec<FieldValidationError>) -> Self {
        Self { errors }
    }

    /// Convert from validator::ValidationErrors
    pub fn from_validation_errors(errors: &ValidationErrors) -> Self {
        let field_errors: Vec<FieldValidationError> = errors
            .field_errors()
            .into_iter()
            .map(|(field, errors)| FieldValidationError {
                field: field.to_string(),
                constraints: errors.iter().map(|e| e.code.to_string()).collect(),
            })
            .collect();

        Self::new(field_errors)
    }
}

/// Single field validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValidationError {
    /// Field name
    pub field: String,
    /// Validation constraints that failed
    pub constraints: Vec<String>,
}

#[cfg(feature = "http")]
#[allow(dead_code)] // Reserved for future use
/// Type for validated parameters
pub trait ValidatedParam: for<'de> Deserialize<'de> + Validate {}

#[cfg(feature = "http")]
#[allow(dead_code)] // Reserved for future use
impl<T: for<'de> Deserialize<'de> + Validate> ValidatedParam for T {}

/// Validation result type
#[cfg(feature = "http")]
#[allow(dead_code)] // Reserved for future use
pub type ValidationResult<T> = Result<T, ValidationErrorsWrapper>;

#[cfg(feature = "http")]
/// Convert validator errors to API errors
impl From<ValidationErrorsWrapper> for super::ApiError {
    fn from(err: ValidationErrorsWrapper) -> Self {
        let first_error = err.errors.first();
        if let Some(error) = first_error {
            let constraint = error
                .constraints
                .first()
                .cloned()
                .unwrap_or_else(|| "invalid".to_string());
            Self::ValidationError {
                field: error.field.clone(),
                constraint,
            }
        } else {
            Self::InvalidInput {
                message: "Validation failed".to_string(),
                field: None,
                value: None,
            }
        }
    }
}

#[cfg(feature = "http")]
#[allow(dead_code)] // Validators are reserved for future use
/// Common validation helpers
pub mod validators {
    use super::*;
    use dashmap::DashMap;
    use once_cell::sync::Lazy;
    use validator::ValidationError;

    /// Regex pattern cache (thread-safe with fine-grained locking)
    static REGEX_CACHE: Lazy<DashMap<String, regex::Regex>> = Lazy::new(|| DashMap::new());

    /// Validate that a string is a valid email
    pub fn validate_email(email: &str) -> Result<(), ValidationError> {
        if !email.contains('@') {
            return Err(ValidationError::new("email"));
        }
        Ok(())
    }

    /// Validate that a string matches a regex pattern (with caching)
    pub fn validate_regex(value: &str, pattern: &str) -> Result<(), ValidationError> {
        let regex = {
            if let Some(cached) = REGEX_CACHE.get(pattern) {
                cached.clone()
            } else {
                let new_regex =
                    regex::Regex::new(pattern).map_err(|_| ValidationError::new("regex"))?;
                REGEX_CACHE.insert(pattern.to_string(), new_regex.clone());
                new_regex
            }
        };

        if !regex.is_match(value) {
            return Err(ValidationError::new("regex"));
        }
        Ok(())
    }

    /// Validate that a number is within a range
    pub fn validate_range<T: PartialOrd + Copy>(
        value: T,
        min: T,
        max: T,
    ) -> Result<(), ValidationError> {
        if value < min || value > max {
            return Err(ValidationError::new("range"));
        }
        Ok(())
    }

    /// Validate that a string has a specific length
    pub fn validate_length(value: &str, min: usize, max: usize) -> Result<(), ValidationError> {
        let len = value.chars().count();
        if len < min || len > max {
            return Err(ValidationError::new("length"));
        }
        Ok(())
    }

    /// Custom validation that returns ApiError on failure
    pub fn validate_or_error<F, E>(validate_fn: F, _error_map: impl FnOnce() -> E) -> Result<(), E>
    where
        F: FnOnce() -> Result<(), ValidationError>,
        E: From<ValidationErrorsWrapper>,
    {
        validate_fn().map_err(|_| {
            let errors = ValidationErrorsWrapper::new(vec![]);
            errors.into()
        })
    }
}

/// Input sanitization utilities for security protection
///
/// Provides functions to sanitize user input:
/// - XSS (Cross-Site Scripting)
/// - Path traversal
/// - Command injection
///
/// # Security Note
/// For SQL operations, always use parameterized queries.
/// String sanitization alone cannot prevent SQL injection.
#[cfg(feature = "http")]
#[allow(dead_code)] // Reserved for future use
pub mod sanitizer {
    use crate::core::ApiError;
    use std::path::PathBuf;

    /// Sanitize a string to prevent XSS attacks
    ///
    /// Converts HTML special characters to their entity equivalents.
    /// For production HTML sanitization, consider using the `ammonia` crate.
    pub fn sanitize_xss(input: &str) -> String {
        input
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('/', "&#x2F;")
    }

    /// Sanitize a string to prevent path traversal attacks
    pub fn sanitize_path(input: &str) -> Result<String, ApiError> {
        // Remove null bytes
        let cleaned = input.replace('\0', "");

        // Check for path traversal attempts
        if cleaned.contains("..") || cleaned.contains("//") {
            return Err(ApiError::validation_error(
                "INVALID_PATH",
                "Path contains invalid characters or traversal attempts",
            ));
        }

        // Normalize path
        let _path = PathBuf::from(&cleaned);

        // Ensure the path doesn't escape the intended directory
        // This is a basic check - in production, use proper path canonicalization
        Ok(cleaned)
    }

    /// Sanitize a filename to prevent path traversal and command injection
    pub fn sanitize_filename(input: &str) -> Result<String, ApiError> {
        if input.is_empty() {
            return Err(ApiError::validation_error(
                "INVALID_FILENAME",
                "Filename cannot be empty",
            ));
        }

        // Remove dangerous characters
        let sanitized: String = input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.' || *c == ' ')
            .collect();

        if sanitized.is_empty() {
            return Err(ApiError::validation_error(
                "INVALID_FILENAME",
                "Filename contains only invalid characters",
            ));
        }

        // Check for path separators
        if sanitized.contains('/') || sanitized.contains('\\') {
            return Err(ApiError::validation_error(
                "INVALID_FILENAME",
                "Filename cannot contain path separators",
            ));
        }

        Ok(sanitized)
    }

    /// Validate and sanitize a user ID (must be positive integer)
    pub fn validate_user_id(id: i64) -> Result<i64, ApiError> {
        if id <= 0 {
            Err(ApiError::validation_error(
                "INVALID_ID",
                "User ID must be a positive integer",
            ))
        } else {
            Ok(id)
        }
    }

    /// Validate a string is not empty after trimming
    pub fn validate_not_empty(input: &str, field_name: &str) -> Result<String, ApiError> {
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            Err(ApiError::validation_error(
                "EMPTY_FIELD",
                format!("{} cannot be empty", field_name),
            ))
        } else {
            Ok(trimmed)
        }
    }

    /// Validate string length
    pub fn validate_length(
        input: &str,
        min: usize,
        max: usize,
        field_name: &str,
    ) -> Result<String, ApiError> {
        if min > max {
            return Err(ApiError::InvalidInput {
                message: format!("Invalid validation parameters for {}", field_name),
                field: Some(field_name.to_string()),
                value: None,
            });
        }
        let len = input.len();
        if len < min {
            Err(ApiError::validation_error(
                "TOO_SHORT",
                format!("{} must be at least {} characters", field_name, min),
            ))
        } else if len > max {
            Err(ApiError::validation_error(
                "TOO_LONG",
                format!("{} must be at most {} characters", field_name, max),
            ))
        } else {
            Ok(input.to_string())
        }
    }

    /// Validate an email address format
    pub fn validate_email_format(email: &str) -> Result<String, ApiError> {
        let trimmed = email.trim().to_string();

        // Basic email validation
        if !trimmed.contains('@') {
            return Err(ApiError::validation_error(
                "INVALID_EMAIL",
                "Email must contain @ symbol",
            ));
        }

        if !trimmed.contains('.') {
            return Err(ApiError::validation_error(
                "INVALID_EMAIL",
                "Email must contain a domain",
            ));
        }

        // Check for common patterns
        if trimmed.starts_with('@') || trimmed.ends_with('@') {
            return Err(ApiError::validation_error(
                "INVALID_EMAIL",
                "Invalid email format",
            ));
        }

        Ok(trimmed)
    }
}

#[cfg(feature = "http")]
#[allow(dead_code)] // Reserved for future use
/// Extract validated parameters from JSON
pub async fn extract_validated<T>(json: &serde_json::Value) -> ValidationResult<T>
where
    T: ValidatedParam + Send,
{
    let params: T =
        serde_json::from_value(json.clone()).map_err(|_| ValidationErrorsWrapper::new(vec![]))?;
    params
        .validate()
        .map_err(|e| ValidationErrorsWrapper::from_validation_errors(&e))?;
    Ok(params)
}

#[cfg(all(feature = "http", test))]
mod tests {
    use super::*;
    use serde::Deserialize;
    use validator::Validate;

    #[derive(Debug, Deserialize, Validate)]
    struct TestParams {
        #[validate(length(min = 1, max = 100))]
        name: String,
        #[validate(email)]
        email: String,
        #[validate(range(min = 18, max = 120))]
        age: u32,
    }

    #[tokio::test]
    async fn test_valid_params() {
        let json = serde_json::json!({
            "name": "John",
            "email": "john@example.com",
            "age": 25
        });

        let result = extract_validated::<TestParams>(&json).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_email() {
        let json = serde_json::json!({
            "name": "John",
            "email": "invalid-email",
            "age": 25
        });

        let result = extract_validated::<TestParams>(&json).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_age_out_of_range() {
        let json = serde_json::json!({
            "name": "John",
            "email": "john@example.com",
            "age": 10
        });

        let result = extract_validated::<TestParams>(&json).await;
        assert!(result.is_err());
    }
}
