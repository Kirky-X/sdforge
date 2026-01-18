// Copyright (c) 2026 Kirky.X
//! Redis cache examples

use sdforge::prelude::*;

/// Redis cached endpoint
///
/// This endpoint would use Redis for distributed caching.
#[service_api(
    name = "redis_cached",
    version = "v1",
    path = "/redis/cached",
    method = "GET",
    tool_name = "redis_cached",
    description = "Redis cached endpoint"
)]
async fn redis_cached() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "data": "This is cached in Redis",
        "backend": "redis"
    }))
}

/// Redis cache with complex data
///
/// Demonstrates caching complex data structures.
#[service_api(
    name = "redis_complex",
    version = "v1",
    path = "/redis/complex",
    method = "GET",
    tool_name = "redis_complex",
    description = "Redis cached complex data"
)]
async fn redis_complex() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "users": [
            {"id": 1, "name": "User 1"},
            {"id": 2, "name": "User 2"}
        ],
        "cached_in": "redis"
    }))
}
