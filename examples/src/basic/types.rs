// Copyright (c) 2026 Kirky.X
//! Type system examples
//!
//! This module demonstrates the core types provided by SDForge.

use sdforge::prelude::*;

/// Example of using ApiMetadata
///
/// Shows how to access API metadata that is automatically generated
/// by the `#[service_api]` macro.
#[service_api(
    name = "get_metadata_info",
    version = "v1",
    path = "/meta",
    method = "GET",
    tool_name = "get_metadata_info",
    description = "Get API metadata information"
)]
async fn get_metadata_info() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Metadata is automatically managed by SDForge",
        "available_fields": ["name", "version", "description"]
    }))
}

/// Example demonstrating response types
///
/// Shows how to return JSON responses.
#[service_api(
    name = "get_wrapped_response",
    version = "v1",
    path = "/wrapped",
    method = "GET",
    tool_name = "get_wrapped_response",
    description = "Get a wrapped response"
)]
async fn get_wrapped_response() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "key": "value",
        "nested": {
            "a": 1,
            "b": 2
        }
    }))
}

/// Example with pagination
///
/// Demonstrates paginated response format.
#[service_api(
    name = "get_paginated",
    version = "v1",
    path = "/paginated-items",
    method = "GET",
    tool_name = "get_paginated",
    description = "Get paginated items"
)]
async fn get_paginated(page: u64, per_page: u64) -> Result<serde_json::Value, ApiError> {
    let items: Vec<serde_json::Value> = (0..per_page)
        .map(|i| {
            serde_json::json!({
                "id": (page - 1) * per_page + i + 1,
                "name": format!("Item {}", (page - 1) * per_page + i + 1)
            })
        })
        .collect();

    Ok(serde_json::json!({
        "items": items,
        "page": page,
        "per_page": per_page,
        "total": 100
    }))
}
