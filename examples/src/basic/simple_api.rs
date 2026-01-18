// Copyright (c) 2026 Kirky.X
//! Simple API definition example
//!
//! This example demonstrates how to define a basic service API using the
//! `#[service_api]` macro.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// Define a simple request type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRequest {
    pub id: u64,
    pub include_details: bool,
}

// Define a response type
#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub created_at: String,
}

/// Simple API to get user by ID
///
/// This demonstrates the most basic usage of the `#[service_api]` macro.
/// The macro automatically generates HTTP and MCP protocol handlers.
#[service_api(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "Get a user by their ID"
)]
async fn get_user(
    // The parameter name "id" matches the path parameter ":id"
    id: u64,
) -> Result<UserResponse, ApiError> {
    // In a real application, this would query a database
    let user = UserResponse {
        id,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    Ok(user)
}

/// API to create a new user
///
/// Demonstrates POST requests with request body.
#[service_api(
    name = "create_user",
    version = "v1",
    path = "/users",
    method = "POST",
    tool_name = "create_user",
    description = "Create a new user"
)]
async fn create_user(
    // Request body is automatically deserialized
    user: UserRequest,
) -> Result<UserResponse, ApiError> {
    let user = UserResponse {
        id: user.id,
        name: "New User".to_string(),
        email: "new@example.com".to_string(),
        created_at: "2024-01-17T00:00:00Z".to_string(),
    };

    Ok(user)
}

/// API with multiple path parameters
///
/// Demonstrates extracting multiple parameters from the path.
#[service_api(
    name = "get_user_post",
    version = "v1",
    path = "/users/:user_id/posts/:post_id",
    method = "GET",
    tool_name = "get_user_post",
    description = "Get a specific post by a user"
)]
async fn get_user_post(user_id: u64, post_id: u64) -> Result<String, ApiError> {
    Ok(format!("Post {} by User {}", post_id, user_id))
}

/// Basic GET endpoint
///
/// Shows the simplest form of HTTP routing with GET method.
#[service_api(
    name = "get_hello",
    version = "v1",
    path = "/hello",
    method = "GET",
    tool_name = "get_hello",
    description = "Simple GET endpoint"
)]
async fn get_hello() -> Result<String, ApiError> {
    Ok("Hello, World!".to_string())
}

/// POST endpoint with request body
///
/// Demonstrates handling POST requests with JSON body.
#[service_api(
    name = "post_echo",
    version = "v1",
    path = "/echo",
    method = "POST",
    tool_name = "post_echo",
    description = "Echo back the request body"
)]
async fn post_echo(body: EchoRequest) -> Result<EchoResponse, ApiError> {
    Ok(EchoResponse {
        received: body.data,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoRequest {
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EchoResponse {
    pub received: serde_json::Value,
}
