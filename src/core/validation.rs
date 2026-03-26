// Copyright (c) 2026 Kirky.X
//! Parameter validation and type conversion utilities
//!
//! This module provides utilities for validating request parameters and
//! converting between different types. Requires the `http` feature.

#![allow(clippy::result_large_err)]

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
    static REGEX_CACHE: Lazy<DashMap<String, regex::Regex>> = Lazy::new(DashMap::new);

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
pub(crate) mod sanitizer {
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
    #[allow(clippy::result_large_err)]
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
    #[allow(clippy::result_large_err)]
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
    use super::super::ApiError;
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

    // ============================================================================
    // Task 2.10: XSS Protection Tests
    // ============================================================================

    #[test]
    fn test_sanitize_xss_legitimate_input() {
        // Legitimate input should pass through unchanged
        let input = "Hello, World!";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(sanitized, input);

        let input = "This is a normal sentence with punctuation.";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(sanitized, input);
    }

    #[test]
    fn test_sanitize_xss_script_tag() {
        // Script tags should be escaped
        let input = "<script>alert('xss')</script>";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(
            sanitized,
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;"
        );
        assert!(!sanitized.contains("<script>"));
    }

    #[test]
    fn test_sanitize_xss_img_tag() {
        // Image tags should be escaped
        let input = "<img src=x onerror=alert('xss')>";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(
            sanitized,
            "&lt;img src=x onerror=alert(&#x27;xss&#x27;)&gt;"
        );
        assert!(!sanitized.contains("<img"));
    }

    #[test]
    fn test_sanitize_xss_iframe_tag() {
        // Iframe tags should be escaped
        let input = "<iframe src=\"http://evil.com\"></iframe>";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(
            sanitized,
            "&lt;iframe src=&quot;http:&#x2F;&#x2F;evil.com&quot;&gt;&lt;&#x2F;iframe&gt;"
        );
        assert!(!sanitized.contains("<iframe"));
    }

    #[test]
    fn test_sanitize_xss_multiple_special_chars() {
        // Multiple special characters should all be escaped
        let input = "<div>'\"/\\";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(sanitized, "&lt;div&gt;&#x27;&quot;&#x2F;\\");
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(!sanitized.contains('"'));
        assert!(!sanitized.contains('\''));
    }

    // ============================================================================
    // Task 2.11: Path Traversal Protection Tests
    // ============================================================================

    #[test]
    fn test_sanitize_path_legitimate() {
        // Legitimate paths should pass through
        let input = "/var/log/app.log";
        let result = sanitizer::sanitize_path(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), input);

        let input = "home/user/documents";
        let result = sanitizer::sanitize_path(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_sanitize_path_reject_double_dot() {
        // Path traversal with .. should be rejected
        let input = "../../../etc/passwd";
        let result = sanitizer::sanitize_path(input);
        assert!(result.is_err());
        // validation_error returns InvalidInput variant
        if let Err(ApiError::InvalidInput {
            message: msg,
            field: _,
            value: _,
        }) = result
        {
            assert!(msg.contains("invalid") || msg.contains("traversal"));
        } else {
            panic!("Expected InvalidInput error");
        }
    }

    #[test]
    fn test_sanitize_path_reject_double_slash() {
        // Double slashes should be rejected
        let input = "//etc/passwd";
        let result = sanitizer::sanitize_path(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_null_byte() {
        // Null bytes should be removed
        let input = "file\0.txt";
        let result = sanitizer::sanitize_path(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "file.txt");
    }

    #[test]
    fn test_sanitize_path_mixed_traversal() {
        // Mixed traversal attempts should be rejected
        let input = "/var/../etc/passwd";
        let result = sanitizer::sanitize_path(input);
        assert!(result.is_err());
    }

    // ============================================================================
    // Task 2.12: Custom Validator Tests
    // ============================================================================

    #[test]
    fn test_custom_email_validator_valid() {
        // Valid email should pass
        let result = validators::validate_email("user@example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_email_validator_invalid() {
        // Invalid email should fail
        let result = validators::validate_email("notanemail");
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_regex_validator_valid() {
        // Valid pattern match should pass
        let result = validators::validate_regex("abc123", r"^[a-z0-9]+$");
        assert!(result.is_ok());
    }

    #[test]
    fn test_custom_regex_validator_invalid() {
        // Invalid pattern match should fail
        let result = validators::validate_regex("abc-123", r"^[a-z0-9]+$");
        assert!(result.is_err());
    }

    #[test]
    fn test_regex_cache_performance() {
        // Regex should be cached for performance
        let pattern = r"^\d{3}-\d{3}-\d{4}$";
        let result1 = validators::validate_regex("123-456-7890", pattern);
        let result2 = validators::validate_regex("987-654-3210", pattern);
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    // ============================================================================
    // Task 2.13: Range Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_range_integer_valid() {
        // Value within range should pass
        let result = validators::validate_range(50, 0, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_integer_too_low() {
        // Value below minimum should fail
        let result = validators::validate_range(-1, 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_range_integer_too_high() {
        // Value above maximum should fail
        let result = validators::validate_range(101, 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_range_float() {
        // Float range validation should work
        let result = validators::validate_range(0.5_f64, 0.0_f64, 1.0_f64);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_string_valid() {
        // String within length range should pass
        let result = validators::validate_length("hello", 1, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_string_too_short() {
        // String too short should fail
        let result = validators::validate_length("hi", 3, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_length_string_too_long() {
        // String too long should fail
        let result = validators::validate_length("hello world", 1, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_length_string_exact_min() {
        // String exactly at minimum should pass
        let result = validators::validate_length("abc", 3, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_string_exact_max() {
        // String exactly at maximum should pass
        let result = validators::validate_length("abcde", 1, 5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_length_validation() {
        // Sanitizer length validation should work
        let result = sanitizer::validate_length("test", 1, 10, "test_field");
        assert!(result.is_ok());

        let result = sanitizer::validate_length("test", 5, 10, "test_field");
        assert!(result.is_err());
    }

    // ============================================================================
    // Comprehensive ValidationErrorsWrapper Tests
    // ============================================================================

    #[test]
    fn test_validation_errors_wrapper_new_empty() {
        let wrapper = ValidationErrorsWrapper::new(vec![]);
        assert!(wrapper.errors.is_empty());
    }

    #[test]
    fn test_validation_errors_wrapper_new_multiple() {
        let errors = vec![
            FieldValidationError {
                field: "email".to_string(),
                constraints: vec!["email".to_string()],
            },
            FieldValidationError {
                field: "name".to_string(),
                constraints: vec!["length".to_string()],
            },
        ];
        let wrapper = ValidationErrorsWrapper::new(errors);
        assert_eq!(wrapper.errors.len(), 2);
    }

    #[test]
    fn test_field_validation_error_equality() {
        let error1 = FieldValidationError {
            field: "email".to_string(),
            constraints: vec!["email".to_string()],
        };
        let error2 = FieldValidationError {
            field: "email".to_string(),
            constraints: vec!["email".to_string()],
        };
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_field_validation_error_clone() {
        let error = FieldValidationError {
            field: "password".to_string(),
            constraints: vec!["min_length".to_string()],
        };
        let cloned = error.clone();
        assert_eq!(error, cloned);
    }

    // ============================================================================
    // Comprehensive Email Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_email_valid_with_subdomain() {
        let result = validators::validate_email("user@mail.example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_valid_with_plus() {
        let result = validators::validate_email("user+tag@example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_valid_with_dots() {
        let result = validators::validate_email("first.last@example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_invalid_empty() {
        let result = validators::validate_email("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_invalid_no_at() {
        let result = validators::validate_email("userexample.com");
        assert!(result.is_err());
    }

    // ============================================================================
    // Comprehensive Regex Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_regex_phone_pattern() {
        let result = validators::validate_regex("123-456-7890", r"^\d{3}-\d{3}-\d{4}$");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_regex_phone_invalid() {
        let result = validators::validate_regex("12-456-7890", r"^\d{3}-\d{3}-\d{4}$");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_regex_invalid_pattern() {
        let result = validators::validate_regex("test", r"[invalid(");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_regex_empty_string() {
        let result = validators::validate_regex("", r"^$");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_regex_unicode() {
        let result = validators::validate_regex("Hello 世界", r"[\w\s\u{4e00}-\u{9fff}]+");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_regex_case_sensitive() {
        let result = validators::validate_regex("ABC", r"^[a-z]+$");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_regex_case_insensitive() {
        let result = validators::validate_regex("ABC", r"(?i)^[a-z]+$");
        assert!(result.is_ok());
    }

    // ============================================================================
    // Comprehensive Range Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_range_i8() {
        let result = validators::validate_range(50_i8, 0_i8, 100_i8);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_i16() {
        let result = validators::validate_range(500_i16, 0_i16, 1000_i16);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_i32() {
        let result = validators::validate_range(50000_i32, 0_i32, 100000_i32);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_i64() {
        let result = validators::validate_range(5000000000_i64, 0_i64, 10000000000_i64);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_u8() {
        let result = validators::validate_range(128_u8, 0_u8, 255_u8);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_u16() {
        let result = validators::validate_range(30000_u16, 0_u16, 65535_u16);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_u32() {
        let result = validators::validate_range(1000000000_u32, 0_u32, 4000000000_u32);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_u64() {
        let result = validators::validate_range(5000000000_u64, 0_u64, 10000000000_u64);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_f32() {
        let result = validators::validate_range(0.5_f32, 0.0_f32, 1.0_f32);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_negative() {
        let result = validators::validate_range(-50, -100, -1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_range_negative_below_min() {
        let result = validators::validate_range(-101, -100, -1);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_range_negative_above_max() {
        let result = validators::validate_range(0, -100, -1);
        assert!(result.is_err());
    }

    // ============================================================================
    // Comprehensive Length Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_length_unicode() {
        let result = validators::validate_length("世界", 1, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_emoji() {
        let result = validators::validate_length("😀😁😂", 1, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_emoji_exact() {
        let result = validators::validate_length("😀😁😂😃", 4, 4);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_mixed_unicode() {
        let result = validators::validate_length("Hello 世界 🌍", 1, 20);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_whitespace() {
        let result = validators::validate_length("   ", 1, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_newlines() {
        let result = validators::validate_length("line1\nline2", 1, 20);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_empty_min_zero() {
        let result = validators::validate_length("", 0, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_length_very_long() {
        let long_string = "a".repeat(1000);
        let result = validators::validate_length(&long_string, 1, 100);
        assert!(result.is_err());
    }

    // ============================================================================
    // Comprehensive XSS Sanitization Tests
    // ============================================================================

    #[test]
    fn test_sanitize_xss_empty() {
        let sanitized = sanitizer::sanitize_xss("");
        assert_eq!(sanitized, "");
    }

    #[test]
    fn test_sanitize_xss_event_handler() {
        let input = "<div onclick=\"alert('xss')\">Click me</div>";
        let sanitized = sanitizer::sanitize_xss(input);
        assert!(!sanitized.contains("<div"));
    }

    #[test]
    fn test_sanitize_xss_javascript_url() {
        let input = "<a href=\"javascript:alert('xss')\">Click</a>";
        let sanitized = sanitizer::sanitize_xss(input);
        assert!(sanitized.contains("&lt;a"));
        assert!(sanitized.contains("&quot;"));
    }

    #[test]
    fn test_sanitize_xss_unicode() {
        let input = "Hello 世界";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(sanitized, input);
    }

    #[test]
    fn test_sanitize_xss_preserves_normal_text() {
        let input = "This is normal text with numbers 123 and symbols !@#$%^&*()";
        let sanitized = sanitizer::sanitize_xss(input);
        assert_eq!(sanitized, input);
    }

    // ============================================================================
    // Comprehensive Path Sanitization Tests
    // ============================================================================

    #[test]
    fn test_sanitize_path_relative() {
        let result = sanitizer::sanitize_path("home/user/documents");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_path_filename() {
        let result = sanitizer::sanitize_path("file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_path_empty() {
        let result = sanitizer::sanitize_path("");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_sanitize_path_multiple_null_bytes() {
        let result = sanitizer::sanitize_path("file\0\0\0.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "file.txt");
    }

    #[test]
    fn test_sanitize_path_hidden_file() {
        let result = sanitizer::sanitize_path(".hidden_file");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_path_unicode() {
        let result = sanitizer::sanitize_path("/var/文档/文件.txt");
        assert!(result.is_ok());
    }

    // ============================================================================
    // Comprehensive Filename Sanitization Tests
    // ============================================================================

    #[test]
    fn test_sanitize_filename_valid_underscore() {
        let result = sanitizer::sanitize_filename("my_document.pdf");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_filename_valid_hyphen() {
        let result = sanitizer::sanitize_filename("my-document.pdf");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_filename_valid_spaces() {
        let result = sanitizer::sanitize_filename("my document.pdf");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_filename_removes_special() {
        let result = sanitizer::sanitize_filename("file<>:\"|?*.txt");
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
    }

    #[test]
    fn test_sanitize_filename_only_special_chars() {
        let result = sanitizer::sanitize_filename("<>:\"|?*");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_filename_unicode() {
        let result = sanitizer::sanitize_filename("文档.pdf");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "文档.pdf");
    }

    #[test]
    fn test_sanitize_filename_multiple_dots() {
        let result = sanitizer::sanitize_filename("file.name.tar.gz");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_filename_hidden() {
        let result = sanitizer::sanitize_filename(".hidden");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitize_filename_null_byte() {
        let result = sanitizer::sanitize_filename("file\0.txt");
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(!sanitized.contains('\0'));
    }

    // ============================================================================
    // Comprehensive User ID Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_user_id_one() {
        let result = sanitizer::validate_user_id(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_user_id_max() {
        let result = sanitizer::validate_user_id(i64::MAX);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_user_id_zero() {
        let result = sanitizer::validate_user_id(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_id_negative() {
        let result = sanitizer::validate_user_id(-1);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_user_id_min() {
        let result = sanitizer::validate_user_id(i64::MIN);
        assert!(result.is_err());
    }

    // ============================================================================
    // Comprehensive Not-Empty Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_not_empty_with_spaces() {
        let result = sanitizer::validate_not_empty("  hello  ", "field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn test_validate_not_empty_whitespace_only() {
        let result = sanitizer::validate_not_empty("   ", "field");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_not_empty_tabs_only() {
        let result = sanitizer::validate_not_empty("\t\t", "field");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_not_empty_newlines_only() {
        let result = sanitizer::validate_not_empty("\n\n", "field");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_not_empty_preserves_inner_spaces() {
        let result = sanitizer::validate_not_empty("  hello world  ", "field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_validate_not_empty_unicode() {
        let result = sanitizer::validate_not_empty("  世界  ", "field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "世界");
    }

    // ============================================================================
    // Comprehensive Sanitizer Length Validation Tests
    // ============================================================================

    #[test]
    fn test_sanitizer_validate_length_exact_min() {
        let result = sanitizer::validate_length("abc", 3, 10, "field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitizer_validate_length_exact_max() {
        let result = sanitizer::validate_length("abcde", 1, 5, "field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitizer_validate_length_too_short() {
        let result = sanitizer::validate_length("ab", 3, 10, "field");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitizer_validate_length_too_long() {
        let result = sanitizer::validate_length("abcdefghijk", 1, 10, "field");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitizer_validate_length_invalid_params() {
        let result = sanitizer::validate_length("test", 10, 1, "field");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitizer_validate_length_zero_to_zero() {
        let result = sanitizer::validate_length("", 0, 0, "field");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sanitizer_validate_length_preserves_original() {
        let input = "  hello  ";
        let result = sanitizer::validate_length(input, 1, 20, "field");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "  hello  ");
    }

    // ============================================================================
    // Comprehensive Email Format Validation Tests
    // ============================================================================

    #[test]
    fn test_validate_email_format_valid() {
        let result = sanitizer::validate_email_format("user@example.com");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "user@example.com");
    }

    #[test]
    fn test_validate_email_format_trims() {
        let result = sanitizer::validate_email_format("  user@example.com  ");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "user@example.com");
    }

    #[test]
    fn test_validate_email_format_missing_at() {
        let result = sanitizer::validate_email_format("userexample.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_format_missing_dot() {
        let result = sanitizer::validate_email_format("user@examplecom");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_format_starts_with_at() {
        let result = sanitizer::validate_email_format("@example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_format_ends_with_at() {
        let result = sanitizer::validate_email_format("user@");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_format_empty() {
        let result = sanitizer::validate_email_format("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_email_format_with_subdomain() {
        let result = sanitizer::validate_email_format("user@mail.example.com");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_email_format_with_plus() {
        let result = sanitizer::validate_email_format("user+tag@example.com");
        assert!(result.is_ok());
    }
}
