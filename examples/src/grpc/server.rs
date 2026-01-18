// Copyright (c) 2026 Kirky.X
//! gRPC server examples

use sdforge::prelude::*;

/// gRPC service endpoint
///
/// This would expose functionality via gRPC protocol.
#[service_api(
    name = "grpc_service",
    version = "v1",
    path = "/grpc/service",
    method = "POST",
    tool_name = "grpc_service",
    description = "gRPC service endpoint"
)]
async fn grpc_service(request: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "response": "This would be served via gRPC",
        "protocol": "grpc"
    }))
}

/// gRPC bidirectional streaming
///
/// Demonstrates bidirectional streaming via gRPC.
#[service_api(
    name = "grpc_bidi",
    version = "v1",
    path = "/grpc/bidi",
    method = "POST",
    tool_name = "grpc_bidi",
    description = "gRPC bidirectional streaming",
    streaming = true
)]
async fn grpc_bidi() -> Result<String, ApiError> {
    Ok("gRPC bidirectional stream".to_string())
}
