// Copyright (c) 2026 Kirky.X
//! Hot reload configuration examples

use sdforge::prelude::*;

/// Hot reload endpoint
///
/// This endpoint would respond to configuration changes.
#[service_api(
    name = "hot_reload_status",
    version = "v1",
    path = "/hot-reload/status",
    method = "GET",
    tool_name = "hot_reload_status",
    description = "Hot reload status"
)]
async fn hot_reload_status() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "watching_config": true,
        "reload_strategy": "automatic"
    }))
}

/// Dynamic configuration
///
/// Endpoint that reflects dynamically reloaded config.
#[service_api(
    name = "dynamic_config",
    version = "v1",
    path = "/dynamic",
    method = "GET",
    tool_name = "dynamic_config",
    description = "Dynamic configuration"
)]
async fn dynamic_config() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "current_setting": "value_from_config",
        "last_reload": "2024-01-17T00:00:00Z"
    }))
}
