// Copyright (c) 2026 Kirky.X
//! API versioning examples
//!
//! This module demonstrates different API versioning strategies.

use sdforge::prelude::*;

/// API version v1
///
/// The version is specified in the `#[service_api]` macro and affects
/// both HTTP paths and MCP tool names.
#[service_api(
    name = "get_data",
    version = "v1",
    path = "/data",
    method = "GET",
    tool_name = "get_data_v1",
    description = "Get data - Version 1"
)]
async fn get_data_v1() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "version": "v1",
        "data": {
            "id": 1,
            "name": "Data V1"
        }
    }))
}

/// API version v2
///
/// Shows how to implement v2 of the same API with different response format.
#[service_api(
    name = "get_data",
    version = "v2",
    path = "/v2/data",
    method = "GET",
    tool_name = "get_data_v2",
    description = "Get data - Version 2"
)]
async fn get_data_v2() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "version": "v2",
        "data": {
            "id": 1,
            "name": "Data V1",
            "metadata": {
                "created_at": "2024-01-01",
                "updated_at": "2024-01-17"
            }
        }
    }))
}

/// Version with breaking changes
///
/// Demonstrates major version with breaking changes.
#[service_api(
    name = "process",
    version = "v2",
    path = "/v2/process",
    method = "POST",
    tool_name = "process_v2",
    description = "Process data - Version 2 with new format"
)]
async fn process_v2(input: serde_json::Value) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "version": "v2",
        "result": input,
        "processed_at": "2024-01-17T00:00:00Z"
    }))
}

/// Beta version API
///
/// Shows how to create experimental/beta API versions.
#[service_api(
    name = "experimental_feature",
    version = "beta",
    path = "/beta/feature",
    method = "GET",
    tool_name = "experimental_feature_beta",
    description = "Experimental feature - Beta"
)]
async fn experimental_feature_beta() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "This is an experimental beta feature",
        "status": "unstable"
    }))
}

/// Multiple versions coexistence
///
/// Shows that multiple versions can coexist and serve different purposes.
#[service_api(
    name = "get_status",
    version = "v1",
    path = "/status",
    method = "GET",
    tool_name = "get_status_v1",
    description = "Get system status - V1"
)]
async fn get_status_v1() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "status": "ok"
    }))
}

#[service_api(
    name = "get_status",
    version = "v2",
    path = "/v2/status",
    method = "GET",
    tool_name = "get_status_v2",
    description = "Get system status - V2 with details"
)]
async fn get_status_v2() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "status": "ok",
        "version": "2.0.0",
        "uptime": "99.9%",
        "components": {
            "database": "healthy",
            "cache": "healthy",
            "storage": "healthy"
        }
    }))
}
