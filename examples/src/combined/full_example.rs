// Copyright (c) 2026 Kirky.X
//! Full example demonstrating multiple features
//!
//! This example shows how to combine multiple SDForge features
//! in a single application.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Health check endpoint
///
/// Basic health check for the service.
#[service_api(
    name = "health_check",
    version = "v1",
    path = "/health",
    method = "GET",
    tool_name = "health_check",
    description = "Service health check"
)]
async fn health_check() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "status": "healthy",
        "version": "0.2.0",
        "timestamp": "2024-01-17T00:00:00Z"
    }))
}

/// Get user with all features
///
/// Demonstrates user endpoint with comprehensive features.
#[service_api(
    name = "get_full_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_full_user",
    description = "Get user with caching, logging, and auth"
)]
async fn get_full_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": "Demo User",
        "email": "demo@example.com",
        "features": ["caching", "logging", "auth"]
    }))
}

/// Request body for creating a user
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

/// Create user
///
/// User creation with logging and validation.
#[service_api(
    name = "create_full_user",
    version = "v1",
    path = "/full-users",
    method = "POST",
    tool_name = "create_full_user",
    description = "Create user with all features"
)]
async fn create_full_user(request: CreateUserRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": 1,
        "name": request.name,
        "email": request.email,
        "created": true
    }))
}

/// Request body for updating a user
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Update user
///
/// User update with caching and logging.
#[service_api(
    name = "update_full_user",
    version = "v1",
    path = "/users/:id",
    method = "PUT",
    tool_name = "update_full_user",
    description = "Update user with features"
)]
async fn update_full_user(
    id: u64,
    request: UpdateUserRequest,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": request.name.unwrap_or_default(),
        "email": request.email.unwrap_or_default(),
        "updated": true
    }))
}

/// Delete user
///
/// User deletion with audit logging.
#[service_api(
    name = "delete_full_user",
    version = "v1",
    path = "/users/:id",
    method = "DELETE",
    tool_name = "delete_full_user",
    description = "Delete user with audit logging"
)]
async fn delete_full_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "deleted": true,
        "audit_logged": true
    }))
}
