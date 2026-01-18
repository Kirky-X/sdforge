// Copyright (c) 2026 Kirky.X
//! SSE streaming examples

use sdforge::prelude::*;

/// SSE streaming endpoint
///
/// This endpoint would stream Server-Sent Events.
#[service_api(
    name = "sse_stream",
    version = "v1",
    path = "/stream/sse",
    method = "GET",
    tool_name = "sse_stream",
    description = "SSE streaming endpoint",
    streaming = true
)]
async fn sse_stream() -> Result<String, ApiError> {
    Ok("This would stream SSE events".to_string())
}

/// Event stream
///
/// Demonstrates streaming multiple events.
#[service_api(
    name = "event_stream",
    version = "v1",
    path = "/stream/events",
    method = "GET",
    tool_name = "event_stream",
    description = "Event stream",
    streaming = true
)]
async fn event_stream() -> Result<String, ApiError> {
    Ok("This would stream multiple events".to_string())
}

/// Progress stream
///
/// Demonstrates streaming progress updates.
#[service_api(
    name = "progress_stream",
    version = "v1",
    path = "/stream/progress",
    method = "GET",
    tool_name = "progress_stream",
    description = "Progress stream",
    streaming = true
)]
async fn progress_stream() -> Result<String, ApiError> {
    Ok("This would stream progress updates".to_string())
}
