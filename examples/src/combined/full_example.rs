// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # 全功能示例
//!
//! 本模块展示 SDForge 框架的完整功能组合使用。
//!
//! ## 特性组合
//!
//! 本示例同时展示了以下功能的集成：
//!
//! | 功能 | 说明 |
//! |------|------|
//! | HTTP | RESTful API 端点 |
//! | MCP | AI 工具集成 |
//! | WebSocket | 实时通信 |
//! | SSE | 服务器推送 |
//! | Security | 认证和授权 |
//!
//! ## API 设计
//!
//! ### 用户管理 API
//!
//! | 端点 | 方法 | 说明 | 协议 |
//! |------|------|------|------|
//! | `/health` | GET | 健康检查 | HTTP |
//! | `/full-users/:id` | GET | 获取用户 | HTTP, MCP |
//! | `/full-users` | POST | 创建用户 | HTTP, MCP |
//! | `/full-users/:id` | PUT | 更新用户 | HTTP |
//! | `/full-users/:id` | DELETE | 删除用户 | HTTP, MCP |
//!
//! ## 多协议服务
//!
//! ### HTTP 访问
//!
//! ```bash
//! # 健康检查
//! curl http://localhost:3000/api/v1/health
//!
//! # 获取用户
//! curl http://localhost:3000/api/v1/full-users/123
//! ```
//!
//! ### MCP 访问
//!
//! ```json
//! {
//!     "tool": "get_full_user",
//!     "input": {"id": 123}
//! }
//! ```
//!
//! ### WebSocket 实时更新
//!
//! ```javascript
//! const ws = new WebSocket('ws://localhost:3000/ws/full-example');
//! ws.onmessage = (e) => console.log(JSON.parse(e.data));
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 类型定义
// ============================================================================

/// 用户数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户唯一标识符
    pub id: u64,
    /// 用户姓名
    pub name: String,
    /// 用户邮箱
    pub email: String,
    /// 用户角色
    pub role: String,
    /// 账户状态
    pub status: String,
    /// 创建时间
    pub created_at: String,
}

/// 创建用户请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    /// 用户姓名
    pub name: String,
    /// 用户邮箱
    pub email: String,
    /// 用户角色 (可选)
    pub role: Option<String>,
}

/// 更新用户请求
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    /// 用户姓名 (可选)
    pub name: Option<String>,
    /// 用户邮箱 (可选)
    pub email: Option<String>,
    /// 用户角色 (可选)
    pub role: Option<String>,
    /// 账户状态 (可选)
    pub status: Option<String>,
}

/// 实时用户更新消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserUpdateMessage {
    /// 事件类型
    #[serde(rename = "type")]
    pub event_type: String,
    /// 用户 ID
    pub user_id: u64,
    /// 更新数据
    pub data: serde_json::Value,
    /// 时间戳
    pub timestamp: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 健康检查端点
///
/// 检查服务健康状态。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/health
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "health_check",
///     "input": {}
/// }
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "status": "healthy",
///     "version": "0.2.0",
///     "timestamp": "2024-01-17T12:00:00Z",
///     "uptime_seconds": 3600,
///     "services": {
///         "database": "connected",
///         "cache": "connected",
///         "websocket": "active"
///     }
/// }
/// ```
#[service_api(
    name = "health_check",
    version = "v1",
    path = "/health",
    method = "GET",
    tool_name = "health_check",
    description = "服务健康检查"
)]
async fn health_check() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "status": "healthy",
        "version": "0.2.0",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "services": {
            "database": "connected",
            "cache": "connected",
            "websocket": "active"
        }
    }))
}

/// 获取完整用户信息
///
/// 获取用户的详细信息，支持缓存和日志。
///
/// # 参数
/// - `id: u64` - 用户 ID
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/full-users/123
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "get_full_user",
///     "input": {"id": 123}
/// }
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": 123,
///     "name": "Demo User",
///     "email": "demo@example.com",
///     "role": "user",
///     "status": "active",
///     "created_at": "2024-01-01T00:00:00Z"
/// }
/// ```
#[service_api(
    name = "get_full_user",
    version = "v1",
    path = "/full-users/:id",
    method = "GET",
    tool_name = "get_full_user",
    description = "获取完整用户信息 (支持缓存和日志)"
)]
async fn get_full_user(id: u64) -> Result<User, ApiError> {
    Ok(User {
        id,
        name: "Demo User".to_string(),
        email: "demo@example.com".to_string(),
        role: "user".to_string(),
        status: "active".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    })
}

/// 创建新用户
///
/// 创建用户账户，支持日志记录。
///
/// # 参数
/// - `request: CreateUserRequest` - 创建请求
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/full-users \
///   -H "Content-Type: application/json" \
///   -d '{"name": "New User", "email": "new@example.com", "role": "user"}'
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "create_full_user",
///     "input": {"name": "New User", "email": "new@example.com"}
/// }
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": 456,
///     "name": "New User",
///     "email": "new@example.com",
///     "role": "user",
///     "status": "active",
///     "created_at": "2024-01-17T12:00:00Z",
///     "created": true
/// }
/// ```
#[service_api(
    name = "create_full_user",
    version = "v1",
    path = "/full-users",
    method = "POST",
    tool_name = "create_full_user",
    description = "创建新用户 (支持日志)"
)]
async fn create_full_user(request: CreateUserRequest) -> Result<serde_json::Value, ApiError> {
    let user_id = 456;
    let role = request.role.clone().unwrap_or_else(|| "user".to_string());

    Ok(serde_json::json!({
        "id": user_id,
        "name": request.name,
        "email": request.email,
        "role": role,
        "status": "active",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "created": true
    }))
}

/// 更新用户信息
///
/// 更新用户数据，支持缓存失效。
///
/// # 参数
/// - `id: u64` - 用户 ID
/// - `request: UpdateUserRequest` - 更新请求
///
/// # HTTP 用法
/// ```bash
/// curl -X PUT http://localhost:3000/api/v1/full-users/123 \
///   -H "Content-Type: application/json" \
///   -d '{"name": "Updated Name", "status": "inactive"}'
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": 123,
///     "name": "Updated Name",
///     "email": "demo@example.com",
///     "role": "user",
///     "status": "inactive",
///     "updated_at": "2024-01-17T12:00:00Z",
///     "updated": true
/// }
/// ```
#[service_api(
    name = "update_full_user",
    version = "v1",
    path = "/full-users/:id",
    method = "PUT",
    tool_name = "update_full_user",
    description = "更新用户信息 (支持缓存失效)"
)]
async fn update_full_user(
    id: u64,
    request: UpdateUserRequest,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": request.name.unwrap_or_else(|| "Demo User".to_string()),
        "email": request.email.unwrap_or_else(|| "demo@example.com".to_string()),
        "role": request.role.unwrap_or_else(|| "user".to_string()),
        "status": request.status.unwrap_or_else(|| "active".to_string()),
        "updated_at": chrono::Utc::now().to_rfc3339(),
        "updated": true
    }))
}

/// 删除用户
///
/// 删除用户账户，带审计日志。
///
/// # 参数
/// - `id: u64` - 用户 ID
///
/// # HTTP 用法
/// ```bash
/// curl -X DELETE http://localhost:3000/api/v1/full-users/123
/// ```
///
/// # MCP 用法
/// ```json
/// {
///     "tool": "delete_full_user",
///     "input": {"id": 123}
/// }
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": 123,
///     "deleted": true,
///     "deleted_at": "2024-01-17T12:00:00Z",
///     "audit_logged": true
/// }
/// ```
#[service_api(
    name = "delete_full_user",
    version = "v1",
    path = "/full-users/:id",
    method = "DELETE",
    tool_name = "delete_full_user",
    description = "删除用户 (带审计日志)"
)]
async fn delete_full_user(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "deleted": true,
        "deleted_at": chrono::Utc::now().to_rfc3339(),
        "audit_logged": true
    }))
}

/// 全功能示例 WebSocket 端点
///
/// 实时用户更新推送。
///
/// # WebSocket URL
/// ```text
/// ws://localhost:3000/ws/full-example
/// ```
///
/// # 推送事件格式
/// ```json
/// {
///     "type": "user_update",
///     "user_id": 123,
///     "data": {"status": "offline"},
///     "timestamp": "2024-01-17T12:00:00Z"
/// }
/// ```
#[service_api(
    name = "full_example_websocket",
    version = "v1",
    path = "/ws/full-example",
    method = "GET",
    tool_name = "full_example_websocket",
    description = "全功能示例 WebSocket 端点"
)]
async fn full_example_websocket() -> Result<String, ApiError> {
    Ok("Full example WebSocket connection established".to_string())
}

/// 用户活动统计
///
/// 获取用户活动统计数据。
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/full-users/stats?user_id=123"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "user_id": 123,
///     "total_requests": 1500,
///     "active_sessions": 2,
///     "last_activity": "2024-01-17T12:00:00Z"
/// }
/// ```
#[service_api(
    name = "user_activity_stats",
    version = "v1",
    path = "/full-users/stats",
    method = "GET",
    tool_name = "user_activity_stats",
    description = "用户活动统计"
)]
async fn user_activity_stats(user_id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "user_id": user_id,
        "total_requests": 1500,
        "active_sessions": 2,
        "last_activity": chrono::Utc::now().to_rfc3339()
    }))
}
