// Copyright (c) 2026 Kirky.X
//!
//! # API Key 认证示例
//!
//! 本模块展示如何使用 API Key 进行身份验证。
//!
//! ## API Key 认证流程
//!
//! 1. **客户端申请 API Key**
//!    - 在开发者平台注册
//!    - 生成唯一的 API Key
//!
//! 2. **发送请求时携带 Key**
//!    ```bash
//!    curl -H "X-API-Key: sk_live_abc123..." \
//!      http://localhost:3000/api/v1/protected/resource
//!    ```
//!
//! 3. **服务器验证 Key**
//!    - 检查 Key 存在性
//!    - 验证 Key 有效性
//!    - 检查权限范围
//!
//! ## Key 格式
//!
//! ### Header 方式
//!
//! ```bash
//! curl -H "X-API-Key: your_api_key" url
//! ```
//!
//! ### Query 参数方式 (不推荐用于生产)
//!
//! ```bash
//! curl "url?api_key=your_api_key"
//! ```
//!
//! ## 安全最佳实践
//!
//! 1. **始终使用 HTTPS** - 防止 Key 在传输过程中被截获
//! 2. **不记录敏感信息** - 不要在日志中记录完整的 Key
//! 3. **定期轮换** - 定期更换 API Key
//! 4. **最小权限** - 为每个 Key 设置最小必要权限

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 认证相关类型定义
// ============================================================================

/// 认证上下文
///
/// 包含认证后的用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// 用户 ID
    pub user_id: String,
    /// 用户角色
    pub role: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// API Key 前缀 (用于标识)
    pub key_prefix: String,
}

/// 认证请求
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    /// API Key
    pub api_key: String,
    /// 请求的操作
    pub action: Option<String>,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 公开数据端点
///
/// 无需认证即可访问的公开数据。
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/public/data
/// ```
///
/// # 响应
/// ```json
/// {
///     "message": "这是公开数据",
///     "accessible": "任何人可访问"
/// }
/// ```
#[service_api(
    name = "public_data",
    version = "v1",
    path = "/public/data",
    method = "GET",
    tool_name = "public_data",
    description = "公开数据端点"
)]
async fn public_data() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "这是公开数据",
        "accessible": "任何人可访问"
    }))
}

/// API Key 受保护端点
///
/// 需要有效的 API Key 才能访问。
///
/// # 请求头
/// ```text
/// X-API-Key: sk_live_your_api_key_here
/// ```
///
/// # HTTP 用法
/// ```bash
/// curl -H "X-API-Key: sk_live_abc123..." \
///   http://localhost:3000/api/v1/protected/api-key
/// ```
///
/// # 响应 (成功)
/// ```json
/// {
///     "message": "已通过 API Key 认证",
///     "user_id": "user_123",
///     "permissions": ["read", "write"]
/// }
/// ```
///
/// # 响应 (失败)
/// ```json
/// {
///     "error": "无效的 API Key"
/// }
/// ```
#[service_api(
    name = "api_key_protected",
    version = "v1",
    path = "/protected/api-key",
    method = "GET",
    tool_name = "api_key_protected",
    description = "API Key 保护的端点"
)]
async fn api_key_protected() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "已通过 API Key 认证",
        "user_id": "user_123",
        "permissions": ["read", "write"]
    }))
}

/// Bearer Token 受保护端点
///
/// 使用 Bearer Token 进行认证。
///
/// # 请求头
/// ```text
/// Authorization: Bearer your_jwt_token_here
/// ```
///
/// # HTTP 用法
/// ```bash
/// curl -H "Authorization: Bearer eyJhbGci..." \
///   http://localhost:3000/api/v1/protected/bearer
/// ```
///
/// # 响应
/// ```json
/// {
///     "message": "已通过 Bearer Token 认证",
///     "user": {
///         "id": "user_456",
///         "email": "user@example.com"
///     }
/// }
/// ```
#[service_api(
    name = "bearer_protected",
    version = "v1",
    path = "/protected/bearer",
    method = "GET",
    tool_name = "bearer_protected",
    description = "Bearer Token 保护的端点"
)]
async fn bearer_protected() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "已通过 Bearer Token 认证",
        "user": {
            "id": "user_456",
            "email": "user@example.com"
        }
    }))
}

/// 多认证方式端点
///
/// 支持多种认证方式的端点。
///
/// # 支持的认证方式
/// 1. API Key (X-API-Key header)
/// 2. Bearer Token (Authorization header)
/// # HTTP 用法
/// ```bash
/// # 使用 API Key
/// curl -H "X-API-Key: sk_live_..." http://localhost:3000/api/v1/protected/multi
///
/// # 使用 Bearer Token
/// curl -H "Authorization: Bearer ..." http://localhost:3000/api/v1/protected/multi
/// ```
///
/// # 响应
/// ```json
/// {
///     "message": "认证成功",
///     "method": "api_key",
///     "user_id": "user_123"
/// }
/// ```
#[service_api(
    name = "multi_auth",
    version = "v1",
    path = "/protected/multi",
    method = "GET",
    tool_name = "multi_auth",
    description = "支持多种认证方式的端点"
)]
async fn multi_auth() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "认证成功",
        "method": "any supported method"
    }))
}

/// 刷新 Token 端点
///
/// 使用刷新令牌获取新的访问令牌。
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/auth/refresh \
///   -H "Content-Type: application/json" \
///   -d '{"refresh_token": "your_refresh_token"}'
/// ```
///
/// # 响应
/// ```json
/// {
///     "access_token": "new_access_token",
///     "expires_in": 3600,
///     "token_type": "Bearer"
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[service_api(
    name = "refresh_token",
    version = "v1",
    path = "/auth/refresh",
    method = "POST",
    tool_name = "refresh_token",
    description = "刷新访问令牌"
)]
async fn refresh_token(request: RefreshTokenRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "access_token": format!("new_token_for_{}", request.refresh_token),
        "expires_in": 3600,
        "token_type": "Bearer"
    }))
}
