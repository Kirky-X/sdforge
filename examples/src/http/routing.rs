// Copyright (c) 2026 Kirky.X
//! HTTP routing examples
//!
//! This module demonstrates different HTTP routing patterns.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Request body for updating a user
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateUserRequest {
    pub name: String,
    pub email: String,
}

/// PUT endpoint for updates
///
/// Demonstrates PUT method for resource updates.
#[service_api(
    name = "put_user",
    version = "v1",
    path = "/users/:id",
    method = "PUT",
    tool_name = "put_user",
    description = "Update a user"
)]
async fn put_user(id: u64, request: UpdateUserRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": request.name,
        "email": request.email,
        "updated": true
    }))
}

/// DELETE endpoint
///
/// Demonstrates DELETE method for resource deletion.
#[service_api(
    name = "delete_user",
    version = "v1",
    path = "/users/:id",
    method = "DELETE",
    tool_name = "delete_user",
    description = "Delete a user"
)]
async fn delete_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "deleted": true
    }))
}

/// Multiple HTTP methods on the same path
///
/// Shows how different methods create separate routes on the same path.
#[service_api(
    name = "get_items",
    version = "v1",
    path = "/items",
    method = "GET",
    tool_name = "get_items",
    description = "List all items"
)]
async fn get_items() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!([
        {"id": 1, "name": "Item 1"},
        {"id": 2, "name": "Item 2"}
    ]))
}

#[service_api(
    name = "post_items",
    version = "v1",
    path = "/items/create",
    method = "POST",
    tool_name = "post_items",
    description = "Create a new item"
)]
async fn post_items(name: String) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": 3,
        "name": name
    }))
}
