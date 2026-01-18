// Copyright (c) 2026 Kirky.X
//! Basic WebSocket examples

use sdforge::prelude::*;

/// WebSocket endpoint
///
/// This endpoint would handle WebSocket connections.
#[service_api(
    name = "websocket_basic",
    version = "v1",
    path = "/ws/basic",
    method = "GET",
    tool_name = "websocket_basic",
    description = "Basic WebSocket endpoint"
)]
async fn websocket_basic() -> Result<String, ApiError> {
    Ok("WebSocket connection would be established".to_string())
}

/// WebSocket with auth
///
/// WebSocket endpoint with authentication.
#[service_api(
    name = "websocket_auth",
    version = "v1",
    path = "/ws/auth",
    method = "GET",
    tool_name = "websocket_auth",
    description = "Authenticated WebSocket"
)]
async fn websocket_auth() -> Result<String, ApiError> {
    Ok("Authenticated WebSocket connection".to_string())
}
