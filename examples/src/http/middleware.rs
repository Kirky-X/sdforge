// Copyright (c) 2026 Kirky.X
//! Middleware examples
//!
//! This module demonstrates HTTP middleware usage in SDForge.

use sdforge::prelude::*;

/// API with logging middleware
///
/// In SDForge, middleware can be applied at various levels.
/// This example shows how to structure APIs that benefit from middleware.
#[service_api(
    name = "api_with_logging",
    version = "v1",
    path = "/api/logged",
    method = "GET",
    tool_name = "api_with_logging",
    description = "API endpoint with logging"
)]
async fn api_with_logging() -> Result<serde_json::Value, ApiError> {
    // In a full implementation, this would use the logging feature
    Ok(serde_json::json!({
        "message": "This request would be logged with the logging feature"
    }))
}

/// API with caching
///
/// Demonstrates how to structure cached endpoints.
#[service_api(
    name = "api_with_cache",
    version = "v1",
    path = "/api/cached",
    method = "GET",
    tool_name = "api_with_cache",
    description = "Cached API endpoint"
)]
async fn api_with_cache() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "This response would be cached with the cache feature"
    }))
}

/// API with rate limiting
///
/// Shows rate-limited endpoint structure.
#[service_api(
    name = "api_rate_limited",
    version = "v1",
    path = "/api/rate-limited",
    method = "GET",
    tool_name = "api_rate_limited",
    description = "Rate-limited API endpoint"
)]
async fn api_rate_limited() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "This endpoint would be rate-limited with the security feature"
    }))
}

/// API with authentication
///
/// Demonstrates authenticated endpoint structure.
#[service_api(
    name = "api_authenticated",
    version = "v1",
    path = "/api/authenticated",
    method = "GET",
    tool_name = "api_authenticated",
    description = "Authenticated API endpoint"
)]
async fn api_authenticated() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "This endpoint requires authentication"
    }))
}
