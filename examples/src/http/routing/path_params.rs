// Copyright (c) 2026 Kirky.X
//!
//! # 路径参数提取示例
//!
//! 本模块演示如何从 URL 路径中提取参数。
//!
//! ## 参数类型
//!
//! ### 1. 数字类型参数
//!
//! ```bash
//! GET /users/123
//! ```
//!
//! ```rust,ignore
//! async fn get_user(id: u64) -> Result<...>
//! ```
//!
//! ### 2. 字符串类型参数
//!
//! ```bash
//! GET /items/abc-123
//! ```
//!
//! ```rust,ignore
//! async fn get_item(id: String) -> Result<...>
//! ```
//!
//! ### 3. 多级嵌套参数
//!
//! ```bash
//! GET /users/123/posts/456/comments/789
//! ```
//!
//! ```rust,ignore
//! async fn get_comment(user_id: u64, post_id: u64, comment_id: u64) -> Result<...>
//! ```
//!
//! ## 使用示例
//!
//! ### 基础路径参数
//!
//! ```bash
//! curl http://localhost:3000/api/v1/http-users/123
//! ```
//!
//! ### 多参数路由
//!
//! ```bash
//! curl http://localhost:3000/api/v1/users/123/posts/456/comments/789
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 请求类型定义
// ============================================================================

/// 更新帖子请求
#[derive(Debug, Deserialize, Serialize)]
pub struct UpdatePostRequest {
    /// 帖子标题
    pub title: String,
    /// 帖子内容
    pub content: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 根据用户 ID 获取用户信息
///
/// 演示最简单的路径参数用法：
/// - 单个路径参数
/// - 自动类型转换 (String -> u64)
///
/// # 参数
/// - `user_id: u64` - 用户 ID (从路径提取)
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/http-users/123
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "user_id": 123,
///     "name": "User 123"
/// }
/// ```
#[service_api(
    name = "get_user_by_id",
    version = "v1",
    path = "/http-users/:user_id",
    method = "GET",
    tool_name = "get_user_by_id",
    description = "根据用户ID获取用户信息"
)]
async fn get_user_by_id(user_id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "name": format!("User {}", user_id)
    }))
}

/// 获取嵌套评论
///
/// 演示多层嵌套路径参数：
/// - 三层嵌套资源
/// - 多个路径参数同时提取
///
/// # 参数
/// - `user_id: u64` - 用户 ID
/// - `post_id: u64` - 帖子 ID
/// - `comment_id: u64` - 评论 ID
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/users/1/posts/2/comments/3
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "user_id": 1,
///     "post_id": 2,
///     "comment_id": 3,
///     "text": "Comment content"
/// }
/// ```
#[service_api(
    name = "get_user_post_comment",
    version = "v1",
    path = "/users/:user_id/posts/:post_id/comments/:comment_id",
    method = "GET",
    tool_name = "get_user_post_comment",
    description = "获取嵌套评论"
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

/// 更新用户帖子 (带请求体)
///
/// 演示结合路径参数和请求体：
/// - 路径参数用于定位资源
/// - 请求体提供更新数据
///
/// # 参数
/// - `user_id: u64` - 用户 ID
/// - `post_id: u64` - 帖子 ID
/// - `request: UpdatePostRequest` - 更新数据
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/users/123/posts/456 \
///   -H "Content-Type: application/json" \
///   -d '{"title": "New Title", "content": "New Content"}'
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "user_id": 123,
///     "post_id": 456,
///     "title": "New Title",
///     "content": "New Content"
/// }
/// ```
#[service_api(
    name = "update_user_post",
    version = "v1",
    path = "/users/:user_id/posts/:post_id",
    method = "POST",
    tool_name = "update_user_post",
    description = "更新用户帖子"
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

/// 获取用户的所有帖子 (混合参数)
///
/// 演示混合使用路径参数和查询参数：
/// - 路径参数定位资源集合
/// - 查询参数提供过滤条件
///
/// # 参数
/// - `user_id: u64` - 用户 ID (路径参数)
/// - `published: Option<bool>` - 是否已发布 (查询参数)
/// - `limit: Option<u32>` - 返回数量限制 (查询参数)
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/users/123/posts?published=true&limit=10"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "user_id": 123,
///     "published": true,
///     "limit": 10,
///     "posts": []
/// }
/// ```
#[service_api(
    name = "get_user_posts",
    version = "v1",
    path = "/users/:user_id/posts",
    method = "GET",
    tool_name = "get_user_posts",
    description = "获取用户帖子列表"
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

/// 获取分类下的资源
///
/// 演示分类资源的路由设计：
/// - 分类路径参数
/// - RESTful 风格
///
/// # 参数
/// - `category: String` - 分类名称
/// - `id: u64` - 资源 ID
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/categories/electronics/items/123
/// ```
#[service_api(
    name = "get_category_item",
    version = "v1",
    path = "/categories/:category/items/:id",
    method = "GET",
    tool_name = "get_category_item",
    description = "获取分类下的指定资源"
)]
async fn get_category_item(category: String, id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "category": category,
        "id": id,
        "name": format!("Item {} in {}", id, category)
    }))
}
