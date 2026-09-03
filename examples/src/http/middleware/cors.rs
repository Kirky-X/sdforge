// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # CORS 跨域资源共享示例
//!
//! 本模块演示如何配置 HTTP API 的 CORS (Cross-Origin Resource Sharing) 支持。
//!
//! ## CORS 概念
//!
//! ### 什么是 CORS?
//!
//! CORS 是一种 W3C 规范，允许 Web 服务器控制哪些域可以访问其资源。
//!
//! ### 关键 HTTP 头
//!
//! | 头部 | 说明 |
//! |------|------|
//! | `Access-Control-Allow-Origin` | 允许的来源 |
//! | `Access-Control-Allow-Methods` | 允许的 HTTP 方法 |
//! | `Access-Control-Allow-Headers` | 允许的请求头 |
//! | `Access-Control-Max-Age` | 预检请求缓存时间 |
//!
//! ## 常见 CORS 场景
//!
//! ### 1. 公开 API (允许所有来源)
//!
//! ```text
//! CorsLayer::new()
//!     .allow_origin(tower_http::cors::Any)
//! ```
//!
//! ### 2. 指定来源
//!
//! ```text
//! CorsLayer::new()
//!     .allow_origin("https://example.com".parse::<AllowedOrigins>().unwrap())
//! ```
//!
//! ### 3. 带凭证的 CORS
//!
//! ```text
//! CorsLayer::new()
//!     .allow_origin("https://example.com")
//!     .allow_credentials(true)
//! ```
//!
//! ## API 端点
//!
//! 本模块中的端点展示了不同 CORS 配置下的请求处理。
//!
//! ### 公开端点
//!
//! 无需特殊认证，适合公开数据。
//!
//! ### 受保护端点
//!
//! 需要认证信息的端点示例。

use sdforge::prelude::*;

// ============================================================================
// API 端点定义
// ============================================================================

/// 公开数据端点（CORS 演示专用）
///
/// 此端点：
/// - 允许所有来源访问 (CORS: `Access-Control-Allow-Origin: *`)
/// - 允许 GET 方法
/// - 不需要认证
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/cors/public-data
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "message": "这是公开数据",
///     "accessible": "任何人可访问"
/// }
/// ```
///
/// # CORS 头
/// ```text
/// Access-Control-Allow-Origin: *
/// Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
/// Access-Control-Allow-Headers: Content-Type, Authorization
/// ```
#[forge(
    name = "cors_public_data",
    version = "v1",
    path = "/cors/public-data",
    method = "GET",
    tool_name = "cors_public_data",
    description = "公开数据端点"
)]
async fn public_data() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "这是公开数据",
        "accessible": "任何人可访问"
    }))
}

/// 公开资源列表
///
/// 获取公开资源列表，无需认证。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/public/resources
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "resources": [
///         {"id": 1, "name": "Resource 1"},
///         {"id": 2, "name": "Resource 2"}
///     ],
///     "total": 2
/// }
/// ```
#[forge(
    name = "public_resources",
    version = "v1",
    path = "/public/resources",
    method = "GET",
    tool_name = "public_resources",
    description = "获取公开资源列表"
)]
async fn public_resources() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "resources": [
            {"id": 1, "name": "Resource 1"},
            {"id": 2, "name": "Resource 2"},
            {"id": 3, "name": "Resource 3"}
        ],
        "total": 3
    }))
}

/// CORS 测试端点 - GET
///
/// 用于测试 CORS 配置的 GET 请求。
///
/// # HTTP 用法
/// ```bash
/// curl -X GET http://localhost:3000/api/v1/cors/test \
///   -H "Origin: https://example.com"
/// ```
///
/// # 响应头
/// ```text
/// Access-Control-Allow-Origin: https://example.com
/// ```
#[forge(
    name = "cors_test_get",
    version = "v1",
    path = "/cors/test",
    method = "GET",
    tool_name = "cors_test_get",
    description = "CORS 测试端点 (GET)"
)]
async fn cors_test_get() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "method": "GET",
        "cors_enabled": true,
        "message": "CORS 配置正常工作"
    }))
}

/// CORS 测试端点 - POST
///
/// 用于测试带请求体的 CORS 请求。
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/cors/test \
///   -H "Origin: https://example.com" \
///   -H "Content-Type: application/json" \
///   -d '{"data": "test"}'
/// ```
///
/// # 响应头
/// ```text
/// Access-Control-Allow-Origin: https://example.com
/// Access-Control-Allow-Methods: GET, POST
/// ```
#[derive(Debug, sdforge::serde::Deserialize)]
pub struct CorsTestRequest {
    pub data: String,
}

#[forge(
    name = "cors_test_post",
    version = "v1",
    path = "/cors/test",
    method = "POST",
    tool_name = "cors_test_post",
    description = "CORS 测试端点 (POST)"
)]
async fn cors_test_post(request: CorsTestRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "method": "POST",
        "received_data": request.data,
        "cors_enabled": true,
        "message": "POST 请求 CORS 配置正常工作"
    }))
}

/// 跨域资源访问示例
///
/// 演示从不同来源访问资源的场景。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/cors/resources/123 \
///   -H "Origin: https://different-domain.com"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": 123,
///     "name": "Cross-Origin Resource",
///     "origin": "https://different-domain.com",
///     "accessible": true
/// }
/// ```
#[forge(
    name = "cors_resource",
    version = "v1",
    path = "/cors/resources/:id",
    method = "GET",
    tool_name = "cors_resource",
    description = "跨域资源访问"
)]
async fn cors_resource(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": format!("Resource {}", id),
        "cross_origin_accessible": true
    }))
}

/// API 信息端点
///
/// 返回 API 的基本信息和 CORS 配置状态。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/api/info
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "name": "SDForge API",
///     "version": "v1",
///     "cors": {
///         "enabled": true,
///         "allow_all_origins": true,
///         "allowed_methods": ["GET", "POST", "PUT", "DELETE"]
///     }
/// }
/// ```
#[forge(
    name = "api_info",
    version = "v1",
    path = "/api/info",
    method = "GET",
    tool_name = "api_info",
    description = "API 信息"
)]
async fn api_info() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "name": "SDForge API",
        "version": "0.2.0",
        "description": "多协议 SDK 框架示例",
        "cors": {
            "enabled": true,
            "allow_all_origins": true,
            "allowed_methods": ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
            "allowed_headers": ["Content-Type", "Authorization"]
        }
    }))
}
