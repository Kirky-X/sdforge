// Copyright (c) 2026 Kirky.X
//! Memory cache examples

use sdforge::prelude::*;

/// Cached endpoint
///
/// This endpoint would use in-memory caching.
#[service_api(
    name = "cached_data",
    version = "v1",
    path = "/cached/data",
    method = "GET",
    tool_name = "cached_data",
    description = "Cached data endpoint"
)]
async fn cached_data() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "data": "This would be cached",
        "cached": true
    }))
}

/// Cache with TTL
///
/// Demonstrates time-based cache expiration.
#[service_api(
    name = "cached_ttl",
    version = "v1",
    path = "/cached/ttl",
    method = "GET",
    tool_name = "cached_ttl",
    description = "Cached data with TTL"
)]
async fn cached_ttl() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "data": "Cached for 60 seconds",
        "ttl": 60
    }))
}

/// Cache invalidation
///
/// Demonstrates cache invalidation patterns.
#[service_api(
    name = "cached_invalidate",
    version = "v1",
    path = "/cached/invalidate",
    method = "POST",
    tool_name = "cached_invalidate",
    description = "Trigger cache invalidation"
)]
async fn cached_invalidate(key: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "key": key,
        "invalidated": true
    }))
}
