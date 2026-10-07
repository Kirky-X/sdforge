// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Parameter validation and type conversion utilities
//!
//! This module provides utilities for validating request parameters and
//! converting between different types. Requires the `http` feature.

// =============================================================================
// Security Limits - Input Size Constraints
// =============================================================================

/// Maximum request body size in bytes (10 MB)
///
/// This limit prevents denial-of-service attacks through large payload submission.
/// Adjust based on your application's needs, but always set some reasonable limit.
pub const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 MB

/// Maximum header name length in characters (128)
///
/// Prevents header-based attacks and excessive memory usage.
pub const MAX_HEADER_NAME_LENGTH: usize = 128;

/// Maximum header value length in bytes (8 KB)
///
/// Limits individual header sizes to prevent buffer exhaustion.
pub const MAX_HEADER_VALUE_LENGTH: usize = 8 * 1024; // 8 KB

/// Maximum URI path length in characters (2048)
///
/// Standard limit for most web servers. Longer paths may indicate attacks.
pub const MAX_URI_PATH_LENGTH: usize = 2048;

/// Maximum query string length in bytes (8 KB)
///
/// Prevents excessive query parameter processing.
pub const MAX_QUERY_STRING_LENGTH: usize = 8 * 1024; // 8 KB

/// Maximum number of headers per request (100)
///
/// Limits header count to prevent header flooding attacks.
pub const MAX_HEADER_COUNT: usize = 100;

/// Maximum API key length in characters (512)
///
/// API keys should be reasonably sized. Longer keys may indicate attacks.
pub const MAX_API_KEY_LENGTH: usize = 512;

/// Maximum JWT token length in characters (4096)
///
/// JWT tokens have a practical maximum size based on claims.
pub const MAX_JWT_TOKEN_LENGTH: usize = 4096;

/// Maximum username length in characters (256)
///
/// Prevents username-based attacks and database field overflow.
pub const MAX_USERNAME_LENGTH: usize = 256;

/// Maximum email length in characters (320)
///
/// RFC 5322 specifies maximum email length as 320 characters.
pub const MAX_EMAIL_LENGTH: usize = 320;

/// Maximum password length in characters (1024)
///
/// While passwords shouldn't be this long, we set a reasonable limit.
pub const MAX_PASSWORD_LENGTH: usize = 1024;

/// Minimum password length in characters (8)
///
/// Security best practice for password policies.
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Maximum description or text field length (10 KB)
///
/// For general text fields that don't need article-length content.
pub const MAX_TEXT_FIELD_LENGTH: usize = 10 * 1024; // 10 KB

/// Maximum JSON field name length in characters (256)
///
/// Prevents excessively long field names in JSON payloads.
pub const MAX_JSON_FIELD_NAME_LENGTH: usize = 256;

/// Maximum array length in JSON payloads (10000)
///
/// Prevents array-based DoS attacks.
pub const MAX_JSON_ARRAY_LENGTH: usize = 10_000;

/// Maximum nesting depth for JSON objects (100)
///
/// Prevents deeply nested JSON parsing attacks.
pub const MAX_JSON_DEPTH: usize = 100;

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
/// Type for validated parameters
///
/// A marker trait for types that can be deserialized and validated.
/// Used by [`extract_validated`] for JSON parameter extraction.
pub trait ValidatedParam: for<'de> Deserialize<'de> + Validate {}

#[cfg(feature = "http")]
impl<T: for<'de> Deserialize<'de> + Validate> ValidatedParam for T {}

/// Validation result type
///
/// A result type for validation operations that can return multiple errors.
#[cfg(feature = "http")]
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
/// Common validation helpers
pub mod validators {
    use super::*;
    use once_cell::sync::Lazy;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use validator::ValidationError;

    /// Regex pattern cache (thread-safe with Mutex<HashMap>)
    static REGEX_CACHE: Lazy<Mutex<HashMap<String, regex::Regex>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));

    /// Validate that a string is a valid email
    pub fn validate_email(email: &str) -> Result<(), ValidationError> {
        if !email.contains('@') {
            return Err(ValidationError::new("email"));
        }
        Ok(())
    }

    /// Validate that a string matches a regex pattern (with caching)
    ///
    /// Poison-aware: 若全局 REGEX_CACHE 中毒（之前 panic 永久污染），
    /// 降级为每次重新编译 regex，避免输入校验永久失效。
    pub fn validate_regex(value: &str, pattern: &str) -> Result<(), ValidationError> {
        let regex = match REGEX_CACHE.lock() {
            Ok(mut cache) => {
                if let Some(cached) = cache.get(pattern) {
                    cached.clone()
                } else {
                    let new_regex =
                        regex::Regex::new(pattern).map_err(|_| ValidationError::new("regex"))?;
                    cache.insert(pattern.to_string(), new_regex.clone());
                    new_regex
                }
            }
            Err(_) => {
                // lock poisoned: 降级到无缓存编译，避免校验永久失效
                regex::Regex::new(pattern).map_err(|_| ValidationError::new("regex"))?
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

#[cfg(feature = "http")]
/// Extract validated parameters from JSON
///
/// Deserializes JSON into a type `T` and validates it using the [`Validate`] trait.
/// Returns a [`ValidationResult`] containing either the validated type or validation errors.
///
/// # Example
/// ```ignore
/// let params: MyParams = extract_validated(&json).await?;
/// ```
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
    // Custom Validator Tests
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
    // Range Validation Tests
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
    // Security Limits Validation Tests
    // ============================================================================

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_security_limits_constants_defined() {
        // Verify all security limit constants are properly defined
        assert!(MAX_REQUEST_BODY_SIZE > 0);
        assert!(MAX_HEADER_NAME_LENGTH > 0);
        assert!(MAX_HEADER_VALUE_LENGTH > 0);
        assert!(MAX_URI_PATH_LENGTH > 0);
        assert!(MAX_QUERY_STRING_LENGTH > 0);
        assert!(MAX_HEADER_COUNT > 0);
        assert!(MAX_API_KEY_LENGTH > 0);
        assert!(MAX_JWT_TOKEN_LENGTH > 0);
        assert!(MAX_USERNAME_LENGTH > 0);
        assert!(MAX_EMAIL_LENGTH > 0);
        assert!(MAX_PASSWORD_LENGTH > 0);
        assert!(MIN_PASSWORD_LENGTH > 0);
        assert!(MAX_TEXT_FIELD_LENGTH > 0);
        assert!(MAX_JSON_FIELD_NAME_LENGTH > 0);
        assert!(MAX_JSON_ARRAY_LENGTH > 0);
        assert!(MAX_JSON_DEPTH > 0);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_security_limits_reasonable_values() {
        // Verify limits are within reasonable ranges

        // Request body: 1MB - 100MB is reasonable
        assert!(MAX_REQUEST_BODY_SIZE >= 1024 * 1024); // At least 1MB
        assert!(MAX_REQUEST_BODY_SIZE <= 100 * 1024 * 1024); // At most 100MB

        // Password limits
        assert!(MIN_PASSWORD_LENGTH >= 6); // Minimum 6 characters
        assert!(MAX_PASSWORD_LENGTH >= 128); // At least 128 characters supported

        // Email per RFC 5322
        assert_eq!(MAX_EMAIL_LENGTH, 320);

        // Header limits
        assert!(MAX_HEADER_NAME_LENGTH >= 64);
        assert!(MAX_HEADER_VALUE_LENGTH >= 1024);

        // JSON limits
        assert!(MAX_JSON_ARRAY_LENGTH >= 1000);
        assert!(MAX_JSON_DEPTH >= 50);
    }

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn test_password_length_validation() {
        // Test password length constraints
        assert!(MIN_PASSWORD_LENGTH < MAX_PASSWORD_LENGTH);

        // Typical passwords should be within range
        assert!("password123".len() >= MIN_PASSWORD_LENGTH);
        assert!("password123".len() <= MAX_PASSWORD_LENGTH);
    }

    // ============================================================================
    // From<ValidationErrorsWrapper> for ApiError conversion tests
    // ============================================================================

    #[test]
    fn test_from_validation_errors_wrapper_with_errors() {
        // When errors are present, the first error's field and constraint
        // should be propagated into the ApiError::ValidationError variant.
        let errors = vec![FieldValidationError {
            field: "email".to_string(),
            constraints: vec!["email".to_string()],
        }];
        let wrapper = ValidationErrorsWrapper::new(errors);
        let api_error: ApiError = wrapper.into();

        match api_error {
            ApiError::ValidationError { field, constraint } => {
                assert_eq!(field, "email");
                assert_eq!(constraint, "email");
            }
            _ => panic!("Expected ValidationError variant"),
        }
    }

    #[test]
    fn test_from_validation_errors_wrapper_with_empty_constraints() {
        // When the first error has no constraints, the constraint should
        // fall back to "invalid".
        let errors = vec![FieldValidationError {
            field: "name".to_string(),
            constraints: vec![],
        }];
        let wrapper = ValidationErrorsWrapper::new(errors);
        let api_error: ApiError = wrapper.into();

        match api_error {
            ApiError::ValidationError { field, constraint } => {
                assert_eq!(field, "name");
                assert_eq!(constraint, "invalid");
            }
            _ => panic!("Expected ValidationError variant"),
        }
    }

    #[test]
    fn test_from_validation_errors_wrapper_empty() {
        // When there are no errors, the conversion should produce
        // ApiError::InvalidInput with a generic message.
        let wrapper = ValidationErrorsWrapper::new(vec![]);
        let api_error: ApiError = wrapper.into();

        match api_error {
            ApiError::InvalidInput {
                message,
                field,
                value,
            } => {
                assert!(message.contains("Validation failed"));
                assert!(field.is_none());
                assert!(value.is_none());
            }
            _ => panic!("Expected InvalidInput variant"),
        }
    }

    // ============================================================================
    // validate_or_error tests
    // ============================================================================

    #[test]
    fn test_validate_or_error_success() {
        // When the validation closure succeeds, validate_or_error should
        // return Ok(()).
        let result: Result<(), ApiError> = validators::validate_or_error(
            || Ok(()),
            || ValidationErrorsWrapper::new(vec![]).into(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_or_error_failure() {
        // When the validation closure fails, validate_or_error should
        // invoke the error mapper and return Err.
        let result: Result<(), ApiError> = validators::validate_or_error(
            || Err(validator::ValidationError::new("test")),
            || ValidationErrorsWrapper::new(vec![]).into(),
        );
        assert!(result.is_err());
    }

    // ============================================================================
    // sanitize_filename edge case tests
    // ============================================================================

    // ============================================================================
    // extract_validated deserialization failure path
    //
    // The `serde_json::from_value(...).map_err(|_| ValidationErrorsWrapper::new(vec![]))`
    // closure is exercised when the input JSON cannot be deserialized into T
    // (e.g., wrong field types or missing required fields). Existing tests
    // only feed JSON that deserializes successfully but fails validation, so
    // the deserialization-error branch was previously uncovered.
    // ============================================================================

    /// Test extract_validated returns Err when JSON has a wrong type for a
    /// field (string where u32 is expected). Covers the
    /// `serde_json::from_value` error branch in extract_validated.
    #[tokio::test]
    async fn test_extract_validated_deserialization_failure_wrong_type() {
        // age is expected to be u32, but we pass a string
        let json = serde_json::json!({
            "name": "John",
            "email": "john@example.com",
            "age": "not a number"
        });

        let result = extract_validated::<TestParams>(&json).await;
        assert!(
            result.is_err(),
            "Deserialization failure should produce Err"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.errors.is_empty(),
            "Deserialization errors produce empty errors vec"
        );
    }

    /// Test extract_validated returns Err when JSON is missing a required
    /// field. Covers the `serde_json::from_value` error branch.
    #[tokio::test]
    async fn test_extract_validated_deserialization_failure_missing_field() {
        // Missing the age field entirely
        let json = serde_json::json!({
            "name": "John",
            "email": "john@example.com"
        });

        let result = extract_validated::<TestParams>(&json).await;
        assert!(
            result.is_err(),
            "Missing field should produce deserialization Err"
        );
    }

    /// Test extract_validated returns Err when JSON root is not an object
    /// (e.g., an array). Covers the `serde_json::from_value` error branch.
    #[tokio::test]
    async fn test_extract_validated_deserialization_failure_non_object() {
        let json = serde_json::json!([1, 2, 3]);

        let result = extract_validated::<TestParams>(&json).await;
        assert!(
            result.is_err(),
            "Non-object JSON should produce deserialization Err"
        );
    }
}
