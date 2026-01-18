// Copyright (c) 2026 Kirky.X
//! Parameter extraction examples
//!
//! This module demonstrates different ways to extract parameters from HTTP requests.

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

/// Path parameter extraction
///
/// Parameters in the URL path are extracted by matching the parameter name
/// to the path segment pattern (e.g., `:id`).
#[service_api(
    name = "get_user_by_id",
    version = "v1",
    path = "/http-users/:user_id",
    method = "GET",
    tool_name = "get_user_by_id",
    description = "Get user by ID from path"
)]
async fn get_user_by_id(user_id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "name": "User".to_string()
    }))
}

/// Query parameter extraction
///
/// Query parameters (after the `?`) are automatically extracted.
#[service_api(
    name = "search_users",
    version = "v1",
    path = "/users/search",
    method = "GET",
    tool_name = "search_users",
    description = "Search users with query parameters"
)]
async fn search_users(
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<serde_json::Value, ApiError> {
    let limit = limit.unwrap_or(10);
    let offset = offset.unwrap_or(0);

    Ok(serde_json::json!({
        "query": query,
        "limit": limit,
        "offset": offset,
        "results": []
    }))
}

/// Multiple path parameters
///
/// Shows extraction of multiple path parameters for nested resources.
#[service_api(
    name = "get_user_post_comment",
    version = "v1",
    path = "/users/:user_id/posts/:post_id/comments/:comment_id",
    method = "GET",
    tool_name = "get_user_post_comment",
    description = "Get a nested resource"
)]
async fn get_user_post_comment(
    user_id: u64,
    post_id: u64,
    comment_id: u64,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "post_id": post_id,
        "comment_id": comment_id,
        "text": "Comment content"
    }))
}

/// Request body for updating a post
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdatePostRequest {
    pub title: String,
    pub content: String,
}

/// POST with path and body parameters
///
/// Demonstrates combining path parameters with request body.
#[service_api(
    name = "update_user_post",
    version = "v1",
    path = "/users/:user_id/posts/:post_id",
    method = "POST",
    tool_name = "update_user_post",
    description = "Update a post with path and body params"
)]
async fn update_user_post(
    user_id: u64,
    post_id: u64,
    request: UpdatePostRequest,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "post_id": post_id,
        "title": request.title,
        "content": request.content
    }))
}

/// Mixed query and path parameters
///
/// Shows combining path parameters with query parameters.
#[service_api(
    name = "get_user_posts",
    version = "v1",
    path = "/users/:user_id/posts",
    method = "GET",
    tool_name = "get_user_posts",
    description = "Get user posts with filters"
)]
async fn get_user_posts(
    user_id: u64,
    published: Option<bool>,
    limit: Option<u32>,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "published": published,
        "limit": limit,
        "posts": []
    }))
}
