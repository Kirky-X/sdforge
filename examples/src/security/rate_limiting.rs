// Copyright (c) 2026 Kirky.X
//! Rate limiting examples
//!
//! This module demonstrates rate limiting patterns.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Standard rate limited endpoint
///
/// This endpoint would be rate-limited in a full implementation.
#[service_api(
    name = "rate_limited_standard",
    version = "v1",
    path = "/rate-limited/standard",
    method = "GET",
    tool_name = "rate_limited_standard",
    description = "Standard rate limited endpoint"
)]
async fn rate_limited_standard() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Request processed",
        "limit": "standard",
        "remaining": "check headers"
    }))
}

/// Strict rate limited endpoint
///
/// More restrictive rate limiting for sensitive operations.
#[service_api(
    name = "rate_limited_strict",
    version = "v1",
    path = "/rate-limited/strict",
    method = "POST",
    tool_name = "rate_limited_strict",
    description = "Strict rate limited endpoint"
)]
async fn rate_limited_strict(data: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Request processed",
        "limit": "strict",
        "data": data
    }))
}

/// Request body for login
#[derive(Debug, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Auth rate limited endpoint
///
/// Rate limiting specifically for authentication endpoints.
#[service_api(
    name = "auth_login",
    version = "v1",
    path = "/auth/login",
    method = "POST",
    tool_name = "auth_login",
    description = "Login with rate limiting"
)]
async fn auth_login(_request: LoginRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Login attempt recorded",
        "rate_limited": "per IP"
    }))
}

/// Public API rate limited
///
/// Rate limiting for public API consumers.
#[service_api(
    name = "public_api_data",
    version = "v1",
    path = "/api/public/data",
    method = "GET",
    tool_name = "public_api_data",
    description = "Public API with rate limiting"
)]
async fn public_api_data() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "data": [1, 2, 3, 4, 5],
        "api_version": "v1"
    }))
}
