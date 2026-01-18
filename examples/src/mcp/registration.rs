// Copyright (c) 2026 Kirky.X
//! MCP tool registration examples
//!
//! This module demonstrates how MCP tools are automatically registered.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Request body for add operation
#[derive(Debug, Deserialize, Serialize)]
pub struct AddRequest {
    pub a: f64,
    pub b: f64,
}

/// First tool for registration demo
///
/// All `#[service_api]` annotated functions with `tool_name` attribute
/// are automatically registered with the MCP server.
#[service_api(
    name = "mcp_calc_add",
    version = "v1",
    path = "/mcp/calc/add",
    method = "POST",
    tool_name = "add",
    description = "Add two numbers"
)]
async fn mcp_add(request: AddRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "result": request.a + request.b
    }))
}

/// Request body for subtract operation
#[derive(Debug, Deserialize, Serialize)]
pub struct SubtractRequest {
    pub a: f64,
    pub b: f64,
}

/// Second tool for registration demo
#[service_api(
    name = "mcp_calc_subtract",
    version = "v1",
    path = "/mcp/calc/subtract",
    method = "POST",
    tool_name = "subtract",
    description = "Subtract two numbers"
)]
async fn mcp_subtract(request: SubtractRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "result": request.a - request.b
    }))
}

/// Request body for multiply operation
#[derive(Debug, Deserialize, Serialize)]
pub struct MultiplyRequest {
    pub a: f64,
    pub b: f64,
}

/// Third tool for registration demo
#[service_api(
    name = "mcp_calc_multiply",
    version = "v1",
    path = "/mcp/calc/multiply",
    method = "POST",
    tool_name = "multiply",
    description = "Multiply two numbers"
)]
async fn mcp_multiply(request: MultiplyRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "result": request.a * request.b
    }))
}

/// Tool without MCP registration
///
/// This function doesn't have `tool_name` attribute,
/// so it won't be registered as an MCP tool.
#[service_api(
    name = "http_only",
    version = "v1",
    path = "/http-only",
    method = "GET",
    description = "HTTP only endpoint"
)]
async fn http_only() -> Result<String, ApiError> {
    Ok("This is HTTP only".to_string())
}
