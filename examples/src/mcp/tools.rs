// Copyright (c) 2026 Kirky.X
//! MCP tool definition examples
//!
//! This module shows how to define MCP tools using SDForge.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Simple MCP tool
///
/// MCP tools are defined using the same `#[service_api]` macro.
/// The `tool_name` attribute specifies the MCP tool name.
#[service_api(
    name = "mcp_hello",
    version = "v1",
    path = "/mcp/hello",
    method = "GET",
    tool_name = "hello",
    description = "A simple greeting tool"
)]
async fn mcp_hello() -> Result<String, ApiError> {
    Ok("Hello from MCP!".to_string())
}

/// Request body for greet operation
#[derive(Debug, Deserialize, Serialize)]
pub struct GreetRequest {
    pub name: String,
    pub language: Option<String>,
}

/// MCP tool with parameters
///
/// MCP tools can accept parameters that are passed via JSON.
#[service_api(
    name = "mcp_greet",
    version = "v1",
    path = "/mcp/greet",
    method = "POST",
    tool_name = "greet",
    description = "Greet a person by name"
)]
async fn mcp_greet(request: GreetRequest) -> Result<String, ApiError> {
    let greeting = match request.language.as_deref() {
        Some("es") => format!("Hola, {}!", request.name),
        Some("fr") => format!("Bonjour, {}!", request.name),
        Some("de") => format!("Hallo, {}!", request.name),
        _ => format!("Hello, {}!", request.name),
    };

    Ok(greeting)
}

/// MCP tool for data retrieval
///
/// Demonstrates MCP tool that returns structured data.
#[service_api(
    name = "mcp_get_data",
    version = "v1",
    path = "/mcp/data",
    method = "GET",
    tool_name = "get_data",
    description = "Retrieve data by ID"
)]
async fn mcp_get_data(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": "Data Item",
        "value": 42
    }))
}

/// Request body for process operation
#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessRequest {
    pub data: serde_json::Value,
    pub options: Option<serde_json::Value>,
}

/// MCP tool with complex input
///
/// Shows how to handle complex JSON objects as input.
#[service_api(
    name = "mcp_process",
    version = "v1",
    path = "/mcp/process",
    method = "POST",
    tool_name = "process_data",
    description = "Process data with options"
)]
async fn mcp_process(request: ProcessRequest) -> Result<serde_json::Value, ApiError> {
    let mut result = request.data.clone();

    if let Some(opts) = request.options {
        if let Some(transform) = opts.get("transform") {
            // Simple example: add a transformation marker
            result["transformed"] = transform.clone();
        }
        if let Some(validate) = opts.get("validate").and_then(|v| v.as_bool()) {
            result["validated"] = serde_json::json!(validate);
        }
    }

    result["processed"] = serde_json::json!(true);

    Ok(result)
}
