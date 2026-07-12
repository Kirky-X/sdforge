// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 认证失败场景示例
//!
//! 本模块展示 401 Unauthorized 和 403 Forbidden 场景的端点配置与错误响应。
//!
//! ## 场景概览
//!
//! | 状态码 | 场景 | 触发条件 |
//! |--------|------|-----------|
//! | 401 Unauthorized | 未认证 | 安全中间件拦截未携带凭证的请求 |
//! | 403 Forbidden | 权限不足 | 已认证但权限不足 |
//!
//! ## 配置方式
//!
//! 认证由 SDForge 的安全中间件 `sdforge::security::auth_middleware` 处理，
//! 而非 `#[forge]` 宏属性。中间件在路由层拦截未认证请求并返回 401，
//! 已认证但权限不足的请求由业务代码返回 403：
//!
//! ```rust,ignore
//! use sdforge::security::auth_middleware;
//! use axum::{Router, middleware::from_fn};
//!
//! // 在路由上挂载认证中间件
//! let protected = Router::new()
//!     .route("/secure", axum::routing::get(secure_endpoint))
//!     .layer(axum::middleware::from_fn(auth_middleware));
//!
//! #[forge(
//!     name = "secure_endpoint",
//!     version = "v1",
//!     path = "/secure",
//!     method = "GET",
//!     description = "Endpoint protected by auth middleware"
//! )]
//! async fn secure_endpoint() -> Result<_, ApiError> { ... }
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 错误响应类型定义
// ============================================================================

/// 401 Unauthorized 响应体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnauthorizedResponse {
    /// 错误代码
    pub error: String,
    /// 人类可读的错误描述
    pub message: String,
    /// WWW-Authenticate 头建议值
    pub www_authenticate: String,
}

/// 403 Forbidden 响应体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenResponse {
    /// 错误代码
    pub error: String,
    /// 人类可读的错误描述
    pub message: String,
    /// 当前用户拥有的权限
    pub current_permissions: Vec<String>,
    /// 完成操作所需的权限
    pub required_permissions: Vec<String>,
}

// ============================================================================
// 需要认证的端点示例
// ============================================================================

/// 管理员专用端点
///
/// 需要有效的认证凭证。未认证时由 `auth_middleware` 中间件拦截并返回 401。
#[forge(
    name = "admin_only",
    version = "v1",
    path = "/auth-failures/admin",
    method = "GET",
    tool_name = "admin_only",
    description = "Admin-only endpoint — requires authentication"
)]
async fn admin_only() -> Result<serde_json::Value, ApiError> {
    // 实际场景中，认证中间件会在请求到达此处前拦截未认证请求
    Ok(serde_json::json!({
        "message": "Welcome, admin",
        "resource": "sensitive data"
    }))
}

/// 超级管理员专用端点
///
/// 需要超级管理员权限。权限不足时返回 403 Forbidden。
#[forge(
    name = "super_admin_only",
    version = "v1",
    path = "/auth-failures/super-admin",
    method = "GET",
    tool_name = "super_admin_only",
    description = "Super admin endpoint — requires elevated permissions"
)]
async fn super_admin_only() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "Welcome, super admin",
        "resource": "critical system configuration"
    }))
}

// ============================================================================
// 错误响应构造示例
// ============================================================================

/// 构造 401 Unauthorized 响应
///
/// 当受保护端点收到未认证请求时，认证中间件
/// 应返回此响应体。
pub fn demo_unauthorized_response() -> UnauthorizedResponse {
    UnauthorizedResponse {
        error: "UNAUTHORIZED".to_string(),
        message: "Authentication required. Provide a valid API key or Bearer token.".to_string(),
        www_authenticate: "Bearer realm=\"sdforge\", error=\"invalid_token\"".to_string(),
    }
}

/// 构造 403 Forbidden 响应
///
/// 当已认证用户权限不足时返回此响应体。
pub fn demo_forbidden_response() -> ForbiddenResponse {
    ForbiddenResponse {
        error: "FORBIDDEN".to_string(),
        message: "Insufficient permissions to access this resource.".to_string(),
        current_permissions: vec!["read".to_string()],
        required_permissions: vec!["admin".to_string(), "super_admin".to_string()],
    }
}

/// 构造 401 错误的 ApiError
///
/// 展示如何在业务代码中抛出 401 错误（对应 `AuthenticationFailed` 变体）。
pub fn demo_unauthorized_api_error() -> ApiError {
    ApiError::AuthenticationFailed {
        reason: "Missing or invalid authentication credentials".to_string(),
    }
}

/// 构造 403 错误的 ApiError
///
/// 展示如何在业务代码中抛出 403 错误（对应 `AccessDenied` 变体）。
pub fn demo_forbidden_api_error() -> ApiError {
    ApiError::AccessDenied {
        permission: "admin".to_string(),
        user_id: None,
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthorized_response_should_have_www_authenticate_header() {
        let resp = demo_unauthorized_response();
        assert_eq!(resp.error, "UNAUTHORIZED");
        assert!(!resp.message.is_empty());
        assert!(resp.www_authenticate.contains("Bearer"));
        assert!(resp.www_authenticate.contains("realm="));
    }

    #[test]
    fn forbidden_response_should_list_required_permissions() {
        let resp = demo_forbidden_response();
        assert_eq!(resp.error, "FORBIDDEN");
        assert!(!resp.required_permissions.is_empty());
        assert!(
            !resp.current_permissions.contains(&"admin".to_string()),
            "current permissions should not include the required admin permission"
        );
    }

    #[test]
    fn unauthorized_api_error_should_indicate_auth_failure() {
        let err = demo_unauthorized_api_error();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("auth") || msg.contains("authentication"),
            "error message should indicate authentication failure: {msg}"
        );
    }

    #[test]
    fn forbidden_api_error_should_indicate_access_denied() {
        let err = demo_forbidden_api_error();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("access") || msg.contains("denied"),
            "error message should indicate access denied: {msg}"
        );
    }
}
