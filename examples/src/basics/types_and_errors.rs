// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 类型系统和错误处理示例
//!
//! 本模块展示 SDForge 框架的核心类型和错误处理模式。
//!
//! ## 核心类型
//!
//! ### ApiError
//!
//! `ApiError` 是 SDForge 的核心错误类型，通过 `#[forge]` 宏自动处理转换。
//!
//! ### ApiMetadata
//!
//! API 元数据，包含名称、版本、描述等信息。
//!
//! ### `ServiceResponse<T>`
//!
//! 服务响应的通用包装类型。
//!
//! ## 错误处理模式
//!
//! ### 1. 直接返回 ApiError
//!
//! ```rust,ignore
//! async fn get_user(id: u64) -> Result<UserResponse, ApiError> {
//!     if id == 0 {
//!         return Err(ApiError::NotFound {
//!             resource: "User".to_string(),
//!             resource_id: Some("0".to_string()),
//!         });
//!     }
//!     // ... 正常逻辑
//! }
//! ```
//!
//! ### 2. 使用自定义错误类型
//!
//! ```rust,ignore
//! use thiserror::Error;
//!
//! #[derive(Debug, Error)]
//! pub enum AppError {
//!     #[error("User not found: {user_id}")]
//!     UserNotFound { user_id: u64 },
//! }
//!
//! impl From<AppError> for ApiError {
//!     fn from(err: AppError) -> Self {
//!         match err {
//!             AppError::UserNotFound { user_id } => ApiError::NotFound { ... },
//!         }
//!     }
//! }
//! ```
//!
//! ### 3. 使用 ? 运算符自动转换
//!
//! ```rust,ignore
//! async fn get_user(id: u64) -> Result<UserResponse, ApiError> {
//!     let user = find_user(id)?; // 自动转换为 ApiError
//!     Ok(user)
//! }
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// 自定义错误类型
// ============================================================================

/// 应用程序自定义错误类型
///
/// 使用 `thiserror` 派生宏定义错误类型，
/// 然后通过 `From` trait 实现自动转换为 `ApiError`。
///
/// # 错误类型
/// - `UserNotFound` - 用户不存在
/// - `ValidationError` - 输入验证失败
/// - `DatabaseError` - 数据库操作失败
#[derive(Debug, Error)]
pub enum AppError {
    /// 用户不存在错误
    ///
    /// # 示例
    /// ```rust,ignore
    /// AppError::UserNotFound { user_id: 123 }
    /// ```
    #[error("用户不存在: {user_id}")]
    UserNotFound { user_id: u64 },

    /// 输入验证错误
    ///
    /// # 字段
    /// - `message` - 错误消息
    /// - `field` - 出错的字段名 (可选)
    #[error("输入验证失败: {message}")]
    ValidationError {
        /// 错误描述
        message: String,
        /// 出错的字段名
        field: Option<String>,
    },

    /// 数据库错误
    ///
    /// # 示例
    /// ```rust,ignore
    /// AppError::DatabaseError { details: "Connection timeout".into() }
    /// ```
    #[error("数据库错误: {details}")]
    DatabaseError { details: String },
}

/// 将 AppError 转换为 ApiError
///
/// 实现 `From<AppError>` 使得可以使用 `?` 运算符自动转换错误。
/// 这是在实际应用中处理错误的推荐方式。
impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        match err {
            // 用户不存在 -> 404 Not Found
            AppError::UserNotFound { user_id } => ApiError::NotFound {
                resource: "User".to_string(),
                resource_id: Some(user_id.to_string()),
            },
            // 验证错误 -> 400 Bad Request
            AppError::ValidationError { message, field } => ApiError::InvalidInput {
                message,
                field,
                value: None,
            },
            // 数据库错误 -> 500 Internal Server Error
            AppError::DatabaseError { details } => ApiError::InvalidInput {
                message: format!("数据库错误: {}", details),
                field: None,
                value: None,
            },
        }
    }
}

// ============================================================================
// 请求类型定义
// ============================================================================

/// 用户验证请求
///
/// 用于演示输入验证
#[derive(Debug, Deserialize, Serialize)]
pub struct ValidateUserRequest {
    /// 用户名 (必填，非空)
    pub name: String,
    /// 邮箱 (必填，格式验证)
    pub email: String,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// API 元数据信息端点
///
/// 演示：
/// - 访问 API 元数据
/// - 返回 JSON Value
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/meta
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "message": "元数据由 SDForge 自动管理",
///     "available_fields": ["name", "version", "description"]
/// }
/// ```
#[forge(
    name = "get_metadata_info",
    version = "v1",
    path = "/meta",
    method = "GET",
    tool_name = "get_metadata_info",
    description = "获取 API 元数据信息"
)]
async fn get_metadata_info() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "message": "元数据由 SDForge 自动管理",
        "available_fields": ["name", "version", "description"]
    }))
}

/// 包装响应示例
///
/// 演示：
/// - 返回任意 JSON 结构
/// - 嵌套对象
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/wrapped
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "key": "value",
///     "nested": {
///         "a": 1,
///         "b": 2
///     }
/// }
/// ```
#[forge(
    name = "get_wrapped_response",
    version = "v1",
    path = "/wrapped",
    method = "GET",
    tool_name = "get_wrapped_response",
    description = "获取包装后的响应"
)]
async fn get_wrapped_response() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "key": "value",
        "nested": {
            "a": 1,
            "b": 2
        }
    }))
}

/// 分页数据示例
///
/// 演示：
/// - 处理分页参数
/// - 返回分页数据结构
///
/// # 参数
/// - `page: u64` - 页码 (从 1 开始)
/// - `per_page: u64` - 每页数量
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/paginated-items?page=1&per_page=10"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "items": [...],
///     "page": 1,
///     "per_page": 10,
///     "total": 100
/// }
/// ```
#[forge(
    name = "get_paginated",
    version = "v1",
    path = "/paginated-items",
    method = "GET",
    tool_name = "get_paginated",
    description = "获取分页数据"
)]
async fn get_paginated(page: u64, per_page: u64) -> Result<serde_json::Value, ApiError> {
    // 生成分页数据项
    let items: Vec<serde_json::Value> = (0..per_page)
        .map(|i| {
            let item_id = (page.saturating_sub(1)) * per_page + i + 1;
            serde_json::json!({
                "id": item_id,
                "name": format!("Item {}", item_id)
            })
        })
        .collect();

    Ok(serde_json::json!({
        "items": items,
        "page": page,
        "per_page": per_page,
        "total": 100  // 假设总共有 100 条数据
    }))
}

/// 带错误处理的获取用户 API
///
/// 演示：
/// - 直接返回 ApiError
/// - 条件判断返回不同错误
/// - 自定义错误类型转换
///
/// # 参数
/// - `id: u64` - 用户 ID
///
/// # 错误处理
/// - `id == 0` - 返回 404 Not Found
/// - `id > 1000` - 返回 UserNotFound 错误 (自动转换)
#[forge(
    name = "get_user_with_error",
    version = "v1",
    path = "/error-users/:id",
    method = "GET",
    tool_name = "get_user_with_error",
    description = "带错误处理的获取用户"
)]
async fn get_user_with_error(id: u64) -> Result<String, ApiError> {
    // 验证: ID 不能为 0
    if id == 0 {
        // 直接返回 ApiError
        return Err(ApiError::NotFound {
            resource: "User".to_string(),
            resource_id: Some(id.to_string()),
        });
    }

    // 验证: ID 不能超过 1000
    if id > 1000 {
        // 使用自定义错误，通过 From trait 自动转换
        return Err(AppError::UserNotFound { user_id: id }.into());
    }

    // 正常情况
    Ok(format!("User {}", id))
}

/// 用户输入验证 API
///
/// 演示：
/// - 必填字段验证
/// - 格式验证
/// - 详细的错误信息
///
/// # 参数
/// - `request: ValidateUserRequest` - 用户验证请求
///
/// # 验证规则
/// - `name` - 不能为空
/// - `email` - 必须包含 "@" 字符
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/users/validate \
///   -H "Content-Type: application/json" \
///   -d '{"name": "John", "email": "john@example.com"}'
/// ```
#[forge(
    name = "validate_user",
    version = "v1",
    path = "/users/validate",
    method = "POST",
    tool_name = "validate_user",
    description = "验证用户输入"
)]
async fn validate_user(request: ValidateUserRequest) -> Result<String, ApiError> {
    // 验证 name 字段
    if request.name.is_empty() {
        return Err(AppError::ValidationError {
            message: "用户名不能为空".to_string(),
            field: Some("name".to_string()),
        }
        .into());
    }

    // 验证 email 格式
    if !request.email.contains('@') {
        return Err(AppError::ValidationError {
            message: "邮箱格式不正确".to_string(),
            field: Some("email".to_string()),
        }
        .into());
    }

    Ok("验证通过".to_string())
}
