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
                constraints: errors
                    .iter()
                    .map(|e| e.code.to_string())
                    .collect(),
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
pub trait ValidatedParam: for<'de> Deserialize<'de> + Validate {}

#[cfg(feature = "http")]
impl<T: for<'de> Deserialize<'de> + Validate> ValidatedParam for T {}

/// Validation result type
#[cfg(feature = "http")]
pub type ValidationResult<T> = Result<T, ValidationErrorsWrapper>;

#[cfg(feature = "http")]
/// Convert validator errors to API errors
impl From<ValidationErrorsWrapper> for super::ApiError {
    fn from(err: ValidationErrorsWrapper) -> Self {
        let first_error = err.errors.first();
        if let Some(error) = first_error {
            let constraint = error.constraints.first().cloned().unwrap_or_else(|| "invalid".to_string());
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
    use validator::ValidationError;

    /// Validate that a string is a valid email
    pub fn validate_email(email: &str) -> Result<(), ValidationError> {
        if !email.contains('@') {
            return Err(ValidationError::new("email"));
        }
        Ok(())
    }

    /// Validate that a string matches a regex pattern
    pub fn validate_regex(value: &str, pattern: &str) -> Result<(), ValidationError> {
        let regex = regex::Regex::new(pattern).map_err(|_| ValidationError::new("regex"))?;
        if !regex.is_match(value) {
            return Err(ValidationError::new("regex"));
        }
        Ok(())
    }

    /// Validate that a number is within a range
    pub fn validate_range<T: PartialOrd + Copy>(value: T, min: T, max: T) -> Result<(), ValidationError> {
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
    pub fn validate_or_error<F, E>(
        validate_fn: F,
        error_map: impl FnOnce() -> E,
    ) -> Result<(), E>
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
pub async fn extract_validated<T>(json: &serde_json::Value) -> ValidationResult<T>
where
    T: ValidatedParam + Send,
{
    let params: T = serde_json::from_value(json.clone()).map_err(|_| ValidationErrorsWrapper::new(vec![]))?;
    params.validate().map_err(|e| ValidationErrorsWrapper::from_validation_errors(&e))?;
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