// Copyright (c) 2026 Kirky.X
//!
//! # 速率限制示例
//!
//! 本模块展示如何实现和管理 API 速率限制。
//!
//! ## 速率限制策略
//!
//! ### 1. 固定窗口
//!
//! 在固定时间窗口内限制请求数。
//!
//! ```text
//! 窗口: 1 分钟
//! 限制: 60 请求
//! 计数: 请求1, 请求2, ... 请求60
//! ```
//!
//! ### 2. 滑动窗口
//!
//! 动态调整的速率限制。
//!
//! ### 3. Token Bucket
//!
//! 令牌桶算法，允许突发流量。
//!
//! ## 常见速率限制级别
//!
//! | 级别 | 限制 | 适用场景 |
//! |------|------|---------|
//! | 免费版 | 60/min | 开发测试 |
//! | 标准版 | 600/min | 普通用户 |
//! | 专业版 | 6000/min | 企业用户 |
//! | 登录 | 10/min | 防止暴力破解 |
//!
//! ## HTTP 响应头
//!
//! ### 速率限制信息
//! ```
//! X-RateLimit-Limit: 60
//! X-RateLimit-Remaining: 45
//! X-RateLimit-Reset: 1640000000
//! ```
//!
//! ### 超限响应
//! ```
//! HTTP/1.1 429 Too Many Requests
//! Retry-After: 30
//! ```
//!
//! ## 客户端最佳实践
//!
//! 1. **实现重试逻辑** - 使用指数退避
//! 2. **缓存响应** - 减少重复请求
//! 3. **批量请求** - 合并多个操作
//! 4. **监控限制状态** - 关注响应头

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 速率限制相关类型
// ============================================================================

/// 速率限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 每分钟限制数
    pub limit_per_minute: u32,
    /// 每小时限制数
    pub limit_per_hour: u32,
    /// 每天限制数
    pub limit_per_day: u32,
}

/// 速率限制状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    /// 当前限制级别
    pub tier: String,
    /// 每分钟剩余请求数
    pub remaining_minute: u32,
    /// 每小时剩余请求数
    pub remaining_hour: u32,
    /// 重置时间戳
    pub reset_at: String,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 标准速率限制端点
///
/// 适用于一般 API 访问的速率限制。
///
/// # 默认限制
//! - 60 请求/分钟
//! - 1000 请求/小时
//!
//! # HTTP 响应头
//! ```
//! X-RateLimit-Limit: 60
//! X-RateLimit-Remaining: 59
//! X-RateLimit-Reset: 1640000060
//! ```
///
/// # 超限响应 (429)
/// ```json
/// {
///     "error": "Too Many Requests",
///     "message": "速率限制已超出",
///     "retry_after": 30
/// }
/// ```
#[service_api(
    name = "rate_limited_standard",
    version = "v1",
    path = "/rate-limited/standard",
    method = "GET",
    tool_name = "rate_limited_standard",
    description = "标准速率限制端点"
)]
async fn rate_limited_standard() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "请求已处理",
        "limit": "standard"
    }))
}

/// 严格速率限制端点
///
/// 适用于敏感操作的更严格限制。
///
/// # 默认限制
//! - 10 请求/分钟
//! - 100 请求/小时
//!
//! # 适用场景
//! - 密码修改
//! - 支付操作
//! - 删除操作
#[service_api(
    name = "rate_limited_strict",
    version = "v1",
    path = "/rate-limited/strict",
    method = "POST",
    tool_name = "rate_limited_strict",
    description = "严格速率限制端点"
)]
async fn rate_limited_strict(
    data: Json<serde_json::Value>,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "请求已处理",
        "limit": "strict",
        "data": data
    }))
}

/// 认证速率限制端点
///
/// 专门针对认证端点的速率限制，防止暴力破解。
///
/// # 默认限制
//! - 5 请求/分钟
//! - 20 请求/小时
//!
//! # 适用场景
//! - 登录
//! - 注册
//! - 密码重置
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/auth/login \
///   -H "Content-Type: application/json" \
///   -d '{"username": "user", "password": "pass"}'
/// ```
///
/// # 响应
/// ```json
/// {
///     "message": "登录尝试已记录",
///     "rate_limited": "per IP"
/// }
/// ```
#[service_api(
    name = "auth_login",
    version = "v1",
    path = "/auth/login",
    method = "POST",
    tool_name = "auth_login",
    description = "带速率限制的登录端点"
)]
async fn auth_login(_request: Json<LoginRequest>) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "登录尝试已记录",
        "rate_limited": "per IP"
    }))
}

/// 公开 API 速率限制端点
///
/// 针对公开 API 的速率限制。
///
/// # 默认限制
//! - 100 请求/分钟
//! - 5000 请求/小时
//!
//! # HTTP 响应头
/// ```
/// X-RateLimit-Limit: 100
/// X-RateLimit-Remaining: 99
/// X-RateLimit-Reset: 1640000060
/// ```
#[service_api(
    name = "public_api_data",
    version = "v1",
    path = "/api/public/data",
    method = "GET",
    tool_name = "public_api_data",
    description = "公开 API 端点 (带速率限制)"
)]
async fn public_api_data() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "data": [1, 2, 3, 4, 5],
        "api_version": "v1"
    }))
}

/// 获取速率限制状态
///
/// 查看当前账户的速率限制状态。
///
/// # HTTP 用法
/// ```bash
/// curl -H "X-API-Key: your_api_key" \
///   http://localhost:3000/api/v1/rate-limit/status
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "tier": "standard",
///     "limits": {
///         "per_minute": 60,
///         "per_hour": 1000,
///         "per_day": 10000
///     },
///     "remaining": {
///         "minute": 45,
///         "hour": 850,
///         "day": 9500
///     },
///     "reset_at": "2024-01-17T12:01:00Z"
/// }
/// ```
#[service_api(
    name = "rate_limit_status",
    version = "v1",
    path = "/rate-limit/status",
    method = "GET",
    tool_name = "rate_limit_status",
    description = "获取速率限制状态"
)]
async fn rate_limit_status() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "tier": "standard",
        "limits": {
            "per_minute": 60,
            "per_hour": 1000,
            "per_day": 10000
        },
        "remaining": {
            "minute": 45,
            "hour": 850,
            "day": 9500
        },
        "reset_at": chrono::Utc::now()
            .checked_add_signed(chrono::Duration::minutes(1))
            .unwrap()
            .to_rfc3339()
    }))
}

/// 批量请求端点 (带特殊限制)
///
/// 演示如何处理批量请求的速率限制。
///
/// # 请求体
/// ```json
/// {
///     "operations": [
///         {"type": "read", "path": "/users/1"},
///         {"type": "read", "path": "/users/2"}
///     ]
/// }
/// ```
///
/// # 响应
/// ```json
/// {
///     "results": [...],
///     "operations_count": 2,
///     "rate_limited": true
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct BatchRequest {
    pub operations: Vec<serde_json::Value>,
}

#[service_api(
    name = "batch_request",
    version = "v1",
    path = "/batch",
    method = "POST",
    tool_name = "batch_request",
    description = "批量请求端点"
)]
async fn batch_request(request: Json<BatchRequest>) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "results": request.operations.clone(),
        "operations_count": request.operations.len(),
        "rate_limited": true
    }))
}
