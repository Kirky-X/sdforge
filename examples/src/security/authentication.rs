// Copyright (c) 2026 Kirky.X
//! Authentication examples
//!
//! This module demonstrates API key and Bearer token authentication patterns.

use sdforge::prelude::*;

/// Public endpoint - no authentication required
///
/// This endpoint is accessible without any authentication.
#[service_api(
    name = "public_data",
    version = "v1",
    path = "/public/data",
    method = "GET",
    tool_name = "public_data",
    description = "Public endpoint without authentication"
)]
async fn public_data() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "This is public data",
        "accessible": "anyone"
    }))
}

/// API key protected endpoint
///
/// This endpoint demonstrates API key authentication structure.
/// In a full implementation, the security feature would handle API key validation.
#[service_api(
    name = "api_key_protected",
    version = "v1",
    path = "/protected/api-key",
    method = "GET",
    tool_name = "api_key_protected",
    description = "API key protected endpoint"
)]
async fn api_key_protected() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Accessed with API key",
        "data": {
            "user_id": "user_123",
            "permissions": ["read", "write"]
        }
    }))
}

/// Bearer token protected endpoint
///
/// This endpoint demonstrates Bearer token authentication structure.
#[service_api(
    name = "bearer_protected",
    version = "v1",
    path = "/protected/bearer",
    method = "GET",
    tool_name = "bearer_protected",
    description = "Bearer token protected endpoint"
)]
async fn bearer_protected() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Accessed with Bearer token",
        "user": {
            "id": "user_456",
            "email": "user@example.com"
        }
    }))
}

/// Multi-auth endpoint
///
/// Endpoint that supports multiple authentication methods.
#[service_api(
    name = "multi_auth",
    version = "v1",
    path = "/protected/multi",
    method = "GET",
    tool_name = "multi_auth",
    description = "Endpoint with multiple auth methods"
)]
async fn multi_auth() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Successfully authenticated",
        "method": "any supported method"
    }))
}
