// Copyright (c) 2026 Kirky.X
//! Error handling examples
//!
//! This module demonstrates how to handle errors in SDForge APIs.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Custom error types for the application
///
/// Using `thiserror` to define application-specific errors that
/// can be converted to `ApiError`.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("User not found: {user_id}")]
    UserNotFound { user_id: u64 },

    #[error("Invalid input: {message}")]
    ValidationError {
        message: String,
        field: Option<String>,
    },

    #[error("Database error: {details}")]
    DatabaseError { details: String },
}

/// Implement From trait to convert AppError to ApiError
///
/// This allows using `?` operator to automatically convert errors.
impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::UserNotFound { user_id } => ApiError::NotFound {
                resource: "User".to_string(),
                resource_id: Some(user_id.to_string()),
            },
            AppError::ValidationError { message, field } => ApiError::InvalidInput {
                message,
                field,
                value: None,
            },
            AppError::DatabaseError { details } => ApiError::InvalidInput {
                message: format!("Database error: {}", details),
                field: None,
                value: None,
            },
        }
    }
}

/// API that demonstrates error handling
///
/// Shows how to return different error types.
#[service_api(
    name = "get_user_with_error",
    version = "v1",
    path = "/error-users/:id",
    method = "GET",
    tool_name = "get_user_with_error",
    description = "Get a user with error handling"
)]
async fn get_user_with_error(id: u64) -> Result<String, ApiError> {
    if id == 0 {
        // Return an error directly
        return Err(ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        });
    }

    if id > 1000 {
        // Use custom error via From trait
        return Err(AppError::UserNotFound { user_id: id }.into());
    }

    Ok(format!("User {}", id))
}

/// API that demonstrates validation errors
/// Request body for user validation
#[derive(Debug, Deserialize, Serialize)]
pub struct ValidateUserRequest {
    pub name: String,
    pub email: String,
}

///
/// /// Shows how to handle input validation.
#[service_api(
    name = "validate_user",
    version = "v1",
    path = "/users/validate",
    method = "POST",
    tool_name = "validate_user",
    description = "Validate user input"
)]
async fn validate_user(request: ValidateUserRequest) -> Result<String, ApiError> {
    // Simple validation
    if request.name.is_empty() {
        return Err(AppError::ValidationError {
            message: "Name cannot be empty".to_string(),
            field: Some("name".to_string()),
        }
        .into());
    }

    if !request.email.contains('@') {
        return Err(AppError::ValidationError {
            message: "Invalid email format".to_string(),
            field: Some("email".to_string()),
        }
        .into());
    }

    Ok("Validation passed".to_string())
}
