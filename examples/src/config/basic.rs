// Copyright (c) 2026 Kirky.X
//! Basic configuration examples

use sdforge::prelude::*;

/// Configuration endpoint
///
/// Demonstrates reading from configuration.
#[service_api(
    name = "config_show",
    version = "v1",
    path = "/config",
    method = "GET",
    tool_name = "config_show",
    description = "Show current configuration"
)]
async fn config_show() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "app_name": "SDForge Examples",
        "version": "0.2.0",
        "environment": "development"
    }))
}

/// Configurable endpoint
///
/// Endpoint that uses configuration values.
#[service_api(
    name = "configurable",
    version = "v1",
    path = "/configurable",
    method = "GET",
    tool_name = "configurable",
    description = "Configurable endpoint"
)]
async fn configurable() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "max_items": 100,
        "timeout_seconds": 30,
        "feature_flags": ["feature_a", "feature_b"]
    }))
}
