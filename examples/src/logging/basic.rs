// Copyright (c) 2026 Kirky.X
//! Basic logging examples

use sdforge::prelude::*;

/// Logged endpoint
///
/// This endpoint would generate structured logs.
#[service_api(
    name = "logged_endpoint",
    version = "v1",
    path = "/logged",
    method = "GET",
    tool_name = "logged_endpoint",
    description = "Endpoint with logging"
)]
async fn logged_endpoint() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Request logged",
        "log_level": "info"
    }))
}

/// Request logging endpoint
///
/// Demonstrates detailed request logging.
#[service_api(
    name = "request_log",
    version = "v1",
    path = "/request-log",
    method = "POST",
    tool_name = "request_log",
    description = "Endpoint with request logging"
)]
async fn request_log(data: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "received": data,
        "logged": true
    }))
}
