// Copyright (c) 2026 Kirky.X
//! Multi-protocol example
//!
//! This example demonstrates serving the same API via multiple protocols
//! (HTTP and MCP) from the same code.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Get greeting
///
/// This API is served via both HTTP and MCP protocols.
/// HTTP: GET /api/v1/greeting?name=World
/// MCP: Call tool "get_greeting" with {"name": "World"}
#[service_api(
    name = "get_greeting",
    version = "v1",
    path = "/greeting",
    method = "GET",
    tool_name = "get_greeting",
    description = "Get a personalized greeting"
)]
async fn get_greeting(name: Option<String>) -> Result<serde_json::Value, ApiError> {
    let name = name.unwrap_or_else(|| "World".to_string());
    Ok(serde_json::json!({
        "greeting": format!("Hello, {}!", name),
        "protocols": ["http", "mcp"]
    }))
}

/// Request body for calculation
#[derive(Debug, Deserialize, Serialize)]
pub struct CalculateRequest {
    pub operation: String,
    pub a: f64,
    pub b: f64,
}

/// Calculate
///
/// Multi-protocal calculator endpoint.
#[service_api(
    name = "calculate",
    version = "v1",
    path = "/calculate",
    method = "POST",
    tool_name = "calculate",
    description = "Perform calculation"
)]
async fn calculate(request: CalculateRequest) -> Result<serde_json::Value, ApiError> {
    let result = match request.operation.as_str() {
        "add" => request.a + request.b,
        "subtract" => request.a - request.b,
        "multiply" => request.a * request.b,
        "divide" => request.a / request.b,
        _ => {
            return Err(ApiError::InvalidInput {
                message: "Unknown operation".to_string(),
                field: Some("operation".to_string()),
                value: None,
            })
        }
    };

    Ok(serde_json::json!({
        "operation": request.operation,
        "a": request.a,
        "b": request.b,
        "result": result,
        "protocols": ["http", "mcp"]
    }))
}

/// List items
///
/// Multi-protocol list endpoint.
#[service_api(
    name = "list_items",
    version = "v1",
    path = "/protocol-items",
    method = "GET",
    tool_name = "list_items",
    description = "List all items"
)]
async fn list_items(limit: Option<u32>) -> Result<serde_json::Value, ApiError> {
    let items: Vec<serde_json::Value> = (1..=limit.unwrap_or(10))
        .map(|i| {
            serde_json::json!({
                "id": i,
                "name": format!("Item {}", i)
            })
        })
        .collect();

    Ok(serde_json::json!({
        "items": items,
        "count": items.len(),
        "protocols": ["http", "mcp"]
    }))
}

/// Get item
///
/// Multi-protocol single item endpoint.
#[service_api(
    name = "get_item",
    version = "v1",
    path = "/items/:id",
    method = "GET",
    tool_name = "get_item",
    description = "Get a single item"
)]
async fn get_item(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": format!("Item {}", id),
        "protocols": ["http", "mcp"]
    }))
}
