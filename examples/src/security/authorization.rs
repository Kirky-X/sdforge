// Copyright (c) 2026 Kirky.X
//! Authorization examples
//!
//! This module demonstrates authorization and permission checking patterns.

use sdforge::prelude::*;

/// Read-only resource
///
/// Demonstrates endpoint that requires read permission.
#[service_api(
    name = "read_resource",
    version = "v1",
    path = "/resources/:id",
    method = "GET",
    tool_name = "read_resource",
    description = "Read a resource"
)]
async fn read_resource(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": "Resource",
        "content": "Sample content"
    }))
}

/// Write resource
///
/// Demonstrates endpoint that requires write permission.
#[service_api(
    name = "write_resource",
    version = "v1",
    path = "/resources/:id",
    method = "PUT",
    tool_name = "write_resource",
    description = "Update a resource"
)]
async fn write_resource(id: u64, name: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": name,
        "updated": true
    }))
}

/// Delete resource
///
/// Demonstrates endpoint that requires admin permission.
#[service_api(
    name = "delete_resource",
    version = "v1",
    path = "/resources/:id",
    method = "DELETE",
    tool_name = "delete_resource",
    description = "Delete a resource"
)]
async fn delete_resource(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "deleted": true
    }))
}

/// Admin-only endpoint
///
/// Demonstrates admin-only access.
#[service_api(
    name = "admin_stats",
    version = "v1",
    path = "/admin/stats",
    method = "GET",
    tool_name = "admin_stats",
    description = "Get admin statistics"
)]
async fn admin_stats() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "total_users": 1000,
        "active_sessions": 150,
        "system_health": "good"
    }))
}

/// Resource ownership check
///
/// Demonstrates ownership-based authorization.
#[service_api(
    name = "user_owned_resource",
    version = "v1",
    path = "/users/:user_id/resources/:id",
    method = "GET",
    tool_name = "user_owned_resource",
    description = "Get user's own resource"
)]
async fn user_owned_resource(user_id: String, id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "resource_id": id,
        "owned": true
    }))
}
