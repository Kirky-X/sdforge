// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 简单 API 定义示例
//!
//! 本模块演示如何使用 `#[forge]` 宏定义各种类型的服务 API。
//!
//! ## 核心概念
//!
//! ### 1. 宏属性
//!
//! `#[forge]` 宏是 SDForge 的核心，它接受以下参数：
//!
//! | 参数 | 说明 | 示例 |
//! |------|------|------|
//! | `name` | API 内部名称 | `"get_user"` |
//! | `version` | API 版本 | `"v1"` |
//! | `path` | HTTP 路由路径 | `"/users/:id"` |
//! | `method` | HTTP 方法 | `"GET"`, `"POST"`, `"PUT"`, `"DELETE"` |
//! | `tool_name` | MCP 工具名称 | `"get_user"` |
//! | `description` | API 描述 | `"获取用户信息"` |
//!
//! ### 2. 路径参数
//!
//! 路径参数使用 `:param_name` 语法，参数名必须与函数参数名匹配。
//!
//! ### 3. 请求体
//!
//! POST 和 PUT 请求可以通过结构体接收 JSON 请求体。
//!
//! ## 使用示例
//!
//! ### 基本 GET 请求
//!
//! ```rust,ignore
//! #[forge(
//!     name = "get_hello",
//!     version = "v1",
//!     path = "/hello",
//!     method = "GET",
//!     tool_name = "get_hello",
//!     description = "返回问候语"
//! )]
//! async fn get_hello() -> Result<String, ApiError> {
//!     Ok("Hello, World!".to_string())
//! }
//! ```
//!
//! ### 带路径参数
//!
//! ```rust,ignore
//! #[forge(
//!     name = "get_user",
//!     version = "v1",
//!     path = "/users/:id",
//!     method = "GET",
//!     tool_name = "get_user",
//!     description = "根据 ID 获取用户"
//! )]
//! async fn get_user(id: u64) -> Result<UserResponse, ApiError> {
//!     // 使用 id 参数查询用户
//!     Ok(UserResponse { id, name: "John".into() })
//! }
//! ```
//!
//! ### 带请求体
//!
//! ```rust,ignore
//! #[derive(Debug, Deserialize)]
//! struct CreateUserRequest {
//!     name: String,
//!     email: String,
//! }
//!
//! #[forge(
//!     name = "create_user",
//!     version = "v1",
//!     path = "/users",
//!     method = "POST",
//!     tool_name = "create_user",
//!     description = "创建新用户"
//! )]
//! async fn create_user(user: CreateUserRequest) -> Result<UserResponse, ApiError> {
//!     // 使用 user 请求体创建用户
//!     Ok(UserResponse { id: 1, name: user.name })
//! }
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 响应类型定义
// ============================================================================

/// 用户响应结构体
///
/// # 示例
/// ```json
/// {
///     "id": 1,
///     "name": "John Doe",
///     "email": "john@example.com",
///     "created_at": "2024-01-01T00:00:00Z"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// 用户唯一标识符
    pub id: u64,
    /// 用户姓名
    pub name: String,
    /// 用户邮箱
    pub email: String,
    /// 创建时间 (ISO 8601 格式)
    pub created_at: String,
}

/// 用户请求结构体
///
/// 用于接收创建用户时的请求数据
#[derive(Debug, Clone, Deserialize)]
pub struct UserRequest {
    /// 用户 ID
    pub id: u64,
    /// 是否包含详细信息
    pub include_details: bool,
}

/// Echo 请求体
///
/// 用于演示 POST 请求体解析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoRequest {
    /// 要回显的任意数据
    pub data: serde_json::Value,
}

/// Echo 响应体
///
/// 返回接收到的数据
#[derive(Debug, Serialize, Deserialize)]
pub struct EchoResponse {
    /// 接收到的数据
    pub received: serde_json::Value,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 简单的 GET 问候语 API
///
/// 这是最基础的 API 示例，演示如何：
/// - 定义无参数的 GET 端点
/// - 返回简单的字符串响应
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/hello
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "get_hello",
///     "input": {}
/// }
/// ```
#[forge(
    name = "get_hello",
    version = "v1",
    path = "/hello",
    method = "GET",
    tool_name = "get_hello",
    description = "返回简单的问候语"
)]
async fn get_hello() -> Result<String, ApiError> {
    // 直接返回 Ok，框架会自动包装响应
    Ok("Hello, World!".to_string())
}

/// 根据 ID 获取用户信息
///
/// 演示：
/// - 路径参数提取 (`:id`)
/// - 路径参数与函数参数名匹配
/// - 返回 JSON 结构体
///
/// # 参数
/// - `id: u64` - 用户的唯一标识符
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/users/123
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "get_user",
///     "input": {"id": 123}
/// }
/// ```
#[forge(
    name = "get_user",
    version = "v1",
    path = "/users/:id",
    method = "GET",
    tool_name = "get_user",
    description = "根据 ID 获取用户信息"
)]
async fn get_user(id: u64) -> Result<UserResponse, ApiError> {
    // 在实际应用中，这里会查询数据库
    // 为了演示，我们返回模拟数据
    let user = UserResponse {
        id,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    Ok(user)
}

/// 创建新用户
///
/// 演示：
/// - 从请求体提取数据
/// - POST 方法处理
/// - 使用请求数据构造响应
///
/// # 参数
/// - `user: UserRequest` - 用户请求体，包含用户信息
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/users \
///   -H "Content-Type: application/json" \
///   -d '{"id": 1, "include_details": true}'
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "create_user",
///     "input": {"id": 1, "include_details": true}
/// }
/// ```
#[forge(
    name = "create_user",
    version = "v1",
    path = "/users",
    method = "POST",
    tool_name = "create_user",
    description = "创建新用户"
)]
async fn create_user(user: UserRequest) -> Result<UserResponse, ApiError> {
    // 使用请求中的数据构造响应
    let user = UserResponse {
        id: user.id,
        name: "New User".to_string(),
        email: "new@example.com".to_string(),
        created_at: "2024-01-17T00:00:00Z".to_string(),
    };

    Ok(user)
}

/// 获取嵌套资源示例
///
/// 演示：
/// - 多个路径参数
/// - 嵌套路由 `/users/:user_id/posts/:post_id`
///
/// # 参数
/// - `user_id: u64` - 用户 ID
/// - `post_id: u64` - 帖子 ID
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/users/1/posts/42
/// ```
#[forge(
    name = "get_user_post",
    version = "v1",
    path = "/users/:user_id/posts/:post_id",
    method = "GET",
    tool_name = "get_user_post",
    description = "获取用户发布的指定帖子"
)]
async fn get_user_post(user_id: u64, post_id: u64) -> Result<String, ApiError> {
    // 格式化响应消息
    Ok(format!("Post {} by User {}", post_id, user_id))
}

/// Echo 回显 API
///
/// 演示：
/// - POST 请求体处理
/// - 返回接收到的数据
///
/// # 参数
/// - `body: EchoRequest` - 要回显的请求体
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/echo \
///   -H "Content-Type: application/json" \
///   -d '{"data": {"message": "Hello"}}'
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "received": {"message": "Hello"}
/// }
/// ```
#[forge(
    name = "post_echo",
    version = "v1",
    path = "/echo",
    method = "POST",
    tool_name = "post_echo",
    description = "回显请求体内容"
)]
async fn post_echo(body: EchoRequest) -> Result<EchoResponse, ApiError> {
    Ok(EchoResponse {
        received: body.data,
    })
}
