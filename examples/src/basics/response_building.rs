// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # 响应构建示例
//!
//! 本模块展示如何在 SDForge 中构建各种类型的 HTTP 响应。
//!
//! ## 响应类型
//!
//! ### 1. 简单字符串响应
//!
//! ```rust,no_run
//! # use sdforge::prelude::ApiError;
//! async fn simple() -> Result<String, ApiError> {
//!     Ok("Hello".to_string())
//! }
//! ```
//!
//! ### 2. JSON 结构体响应
//!
//! ```text
//! # use sdforge::prelude::ApiError;
//! #[derive(Serialize)]
//! struct Response {
//!     message: String,
//!     data: Vec<i32>,
//! }
//!
//! async fn json_response() -> Result<Response, ApiError> {
//!     Ok(Response { message: "OK".into(), data: vec![1, 2, 3] })
//! }
//! ```
//!
//! ### 3. 任意 JSON Value
//!
//! ```rust,no_run
//! # use serde_json;
//! # use sdforge::prelude::ApiError;
//! async fn json_value() -> Result<serde_json::Value, ApiError> {
//!     Ok(serde_json::json!({ "key": "value" }))
//! }
//! ```

use sdforge::prelude::*;
use sdforge::serde::{Deserialize, Serialize};

// ============================================================================
// 响应类型定义
// ============================================================================

/// 标准 API 响应包装器
///
/// 提供统一的响应格式，便于客户端处理。
///
/// # 格式
/// ```json
/// {
///     "success": true,
///     "data": {...},
///     "timestamp": "2024-01-01T00:00:00Z"
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// 操作是否成功
    pub success: bool,
    /// 响应数据
    pub data: T,
    /// 响应时间戳
    pub timestamp: String,
}

/// 列表响应
///
/// 用于返回数据列表的响应结构。
///
/// # 格式
/// ```json
/// {
///     "items": [...],
///     "count": 10,
///     "total": 100
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct ListResponse<T: Serialize> {
    /// 数据列表
    pub items: Vec<T>,
    /// 当前返回数量
    pub count: usize,
    /// 总数量
    pub total: usize,
}

/// 状态响应
///
/// 用于返回操作状态的响应结构。
///
/// # 格式
/// ```json
/// {
///     "operation": "delete_user",
///     "status": "completed",
///     "affected_rows": 1
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    /// 操作名称
    pub operation: String,
    /// 状态 (pending, completed, failed)
    pub status: String,
    /// 受影响的行数
    pub affected_rows: usize,
}

/// 数据项
///
/// 用于列表响应中的单个数据项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataItem {
    /// 唯一标识符
    pub id: u64,
    /// 名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 是否启用
    pub enabled: bool,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 获取单个数据项
///
/// 演示：返回单个数据项的 JSON 响应
///
/// # 参数
/// - `id: u64` - 数据项 ID
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/items/42
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "id": 42,
///     "name": "Item 42",
///     "description": "Description for item 42",
///     "enabled": true
/// }
/// ```
#[forge(
    name = "get_item",
    version = "v1",
    path = "/items/:id",
    method = "GET",
    tool_name = "get_item",
    description = "获取单个数据项"
)]
async fn get_item(id: u64) -> Result<DataItem, ApiError> {
    // 返回模拟数据
    Ok(DataItem {
        id,
        name: format!("Item {}", id),
        description: format!("Description for item {}", id),
        enabled: true,
    })
}

/// 获取数据项列表
///
/// 演示：返回列表类型的响应
///
/// # 参数
/// - `limit: Option<u32>` - 返回数量限制
/// - `offset: Option<u32>` - 偏移量
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/items?limit=5&offset=10"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "items": [...],
///     "count": 5,
///     "total": 100
/// }
/// ```
#[forge(
    name = "list_items",
    version = "v1",
    path = "/items",
    method = "GET",
    tool_name = "list_items",
    description = "获取数据项列表"
)]
async fn list_items(
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<ListResponse<DataItem>, ApiError> {
    let limit = limit.unwrap_or(10).min(100) as usize;
    let offset = offset.unwrap_or(0) as usize;
    let total = 100;

    // 生成模拟数据
    let items: Vec<DataItem> = (0..limit)
        .map(|i| {
            let id = (offset + i + 1) as u64;
            DataItem {
                id,
                name: format!("Item {}", id),
                description: format!("Description for item {}", id),
                enabled: id.is_multiple_of(2),
            }
        })
        .collect();

    Ok(ListResponse {
        items,
        count: limit,
        total,
    })
}

/// 执行删除操作
///
/// 演示：返回操作状态的响应
///
/// # 参数
/// - `id: u64` - 要删除的项 ID
///
/// # HTTP 用法
/// ```bash
/// curl -X DELETE http://localhost:3000/api/v1/items/42
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "operation": "delete_item",
///     "status": "completed",
///     "affected_rows": 1
/// }
/// ```
#[forge(
    name = "delete_item",
    version = "v1",
    path = "/items/:id",
    method = "DELETE",
    tool_name = "delete_item",
    description = "删除数据项"
)]
async fn delete_item(_id: u64) -> Result<StatusResponse, ApiError> {
    Ok(StatusResponse {
        operation: "delete_item".to_string(),
        status: "completed".to_string(),
        affected_rows: 1,
    })
}

/// 获取统一包装的响应
///
/// 演示：使用 ApiResponse 包装器提供统一的响应格式
///
/// # HTTP 用法
/// ```bash
/// curl http://localhost:3000/api/v1/wrapped/items/42
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "success": true,
///     "data": {
///         "id": 42,
///         "name": "Item 42",
///         "description": "Description for item 42",
///         "enabled": true
///     },
///     "timestamp": "2024-01-17T12:00:00Z"
/// }
/// ```
#[forge(
    name = "get_wrapped_item",
    version = "v1",
    path = "/wrapped/items/:id",
    method = "GET",
    tool_name = "get_wrapped_item",
    description = "获取包装后的数据项"
)]
async fn get_wrapped_item(id: u64) -> Result<ApiResponse<DataItem>, ApiError> {
    let item = DataItem {
        id,
        name: format!("Item {}", id),
        description: format!("Description for item {}", id),
        enabled: true,
    };

    // 获取当前时间戳
    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(ApiResponse {
        success: true,
        data: item,
        timestamp,
    })
}

/// 创建新数据项
///
/// 演示：处理创建请求并返回创建后的数据
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/items \
///   -H "Content-Type: application/json" \
///   -d '{"name": "New Item", "description": "New Description", "enabled": true}'
/// ```
#[derive(Debug, Deserialize)]
pub struct CreateItemRequest {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[forge(
    name = "create_item",
    version = "v1",
    path = "/items",
    method = "POST",
    tool_name = "create_item",
    description = "创建新数据项"
)]
async fn create_item(_request: CreateItemRequest) -> Result<StatusResponse, ApiError> {
    // 模拟创建操作
    Ok(StatusResponse {
        operation: "create_item".to_string(),
        status: "completed".to_string(),
        affected_rows: 1,
    })
}

/// 更新数据项
///
/// 演示：处理更新请求并返回更新后的数据
///
/// # HTTP 用法
/// ```bash
/// curl -X PUT http://localhost:3000/api/v1/items/42 \
///   -H "Content-Type: application/json" \
///   -d '{"name": "Updated Name", "description": "Updated Description", "enabled": false}'
/// ```
#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[forge(
    name = "update_item",
    version = "v1",
    path = "/items/:id",
    method = "PUT",
    tool_name = "update_item",
    description = "更新数据项"
)]
async fn update_item(
    id: u64,
    request: UpdateItemRequest,
) -> Result<ApiResponse<DataItem>, ApiError> {
    let item = DataItem {
        id,
        name: request.name.clone(),
        description: request.description.clone(),
        enabled: request.enabled,
    };

    let timestamp = chrono::Utc::now().to_rfc3339();

    Ok(ApiResponse {
        success: true,
        data: item,
        timestamp,
    })
}

// ============================================================================
// 自定义成功状态码（#[forge(status = ...)] + ServiceResponse::success_with_status）
// ============================================================================

/// 创建数据项（静态声明 201）
///
/// 演示：通过 `#[forge(status = 201)]` 静态声明成功状态码，
/// 适用于 POST 创建资源时返回 201 Created。
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/items-with-status \
///   -H "Content-Type: application/json" \
///   -d '{"name": "New Item", "description": "New Description", "enabled": true}'
/// ```
///
/// # 响应
/// HTTP 201 Created，Body 为创建的资源（裸类型，由宏注入 status）。
#[forge(
    name = "create_item_with_status",
    version = "v1",
    path = "/items-with-status",
    method = "POST",
    status = 201,
    tool_name = "create_item_with_status",
    description = "创建数据项（静态 201）"
)]
async fn create_item_with_status(request: CreateItemRequest) -> Result<DataItem, ApiError> {
    Ok(DataItem {
        id: 100,
        name: request.name,
        description: request.description,
        enabled: request.enabled,
    })
}

/// 创建数据项（动态状态码）
///
/// 演示：通过 `ServiceResponse::success_with_status(data, code)` 在运行时
/// 动态决定成功状态码，适用于 upsert（存在则 200，新建则 201）等场景。
///
/// # HTTP 用法
/// ```bash
/// curl -X POST http://localhost:3000/api/v1/items-dynamic-status \
///   -H "Content-Type: application/json" \
///   -d '{"name": "New Item", "description": "New Description", "enabled": true}'
/// ```
///
/// # 响应
/// HTTP 201 Created，Body 为 `ServiceResponse` 包装结构（含 `status_code` 字段）。
#[forge(
    name = "create_item_dynamic_status",
    version = "v1",
    path = "/items-dynamic-status",
    method = "POST",
    tool_name = "create_item_dynamic_status",
    description = "创建数据项（动态状态码）"
)]
async fn create_item_dynamic_status(
    request: CreateItemRequest,
) -> Result<ServiceResponse<DataItem>, ApiError> {
    let item = DataItem {
        id: 200,
        name: request.name,
        description: request.description,
        enabled: request.enabled,
    };
    // Dynamic entry: code decided at runtime, takes precedence over macro `status`.
    Ok(ServiceResponse::success_with_status(item, 201))
}
