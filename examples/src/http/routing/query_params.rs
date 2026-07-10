// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # 查询参数提取示例
//!
//! 本模块演示如何从 URL 查询字符串中提取参数。
//!
//! ## 查询参数特点
//!
//! 1. **可选性** - 查询参数通常是可选的，使用 `Option<T>` 类型
//! 2. **默认值** - 可以提供默认值处理未提供的参数
//! 3. **多值参数** - 某些场景需要支持多个值
//!
//! ## 参数类型
//!
//! ### 1. 必需参数
//!
//! ```bash
//! GET /search?q=keyword
//! ```
//!
//! ```rust,ignore
//! async fn search(q: String) -> Result<...>
//! ```
//!
//! ### 2. 可选参数
//!
//! ```bash
//! GET /items?limit=10
//! ```
//!
//! ```rust,ignore
//! async fn list_items(limit: Option<u32>) -> Result<...>
//! ```
//!
//! ### 3. 带默认值的参数
//!
//! ```rust,ignore
//! async fn list_items(limit: Option<u32>) -> Result<...> {
//!     let limit = limit.unwrap_or(10); // 默认 10
//! }
//! ```
//!
//! ## 使用示例
//!
//! ### 搜索功能
//!
//! ```bash
//! curl "http://localhost:3000/api/v1/users/search?query=john&limit=20&offset=0"
//! ```
//!
//! ### 过滤和排序
//!
//! ```bash
//! curl "http://localhost:3000/api/v1/items?sort=name&order=asc&category=electronics"
//! ```

use sdforge::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// 请求类型定义
// ============================================================================

/// 排序选项枚举
#[derive(Debug, Deserialize, Serialize)]
pub enum SortOrder {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 用户搜索 API
///
/// 演示查询参数的基本用法：
/// - 必需查询参数
/// - 可选查询参数带默认值
///
/// # 参数
/// - `query: String` - 搜索关键词 (必需)
/// - `limit: Option<u32>` - 返回数量 (可选，默认 10)
/// - `offset: Option<u32>` - 偏移量 (可选，默认 0)
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/users/search?query=john&limit=20&offset=0"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "query": "john",
///     "limit": 20,
///     "offset": 0,
///     "results": [
///         {"id": 1, "name": "John Doe"},
///         {"id": 2, "name": "Johnny Smith"}
///     ]
/// }
/// ```
#[service_api(
    name = "search_users",
    version = "v1",
    path = "/users/search",
    method = "GET",
    tool_name = "search_users",
    description = "搜索用户"
)]
async fn search_users(
    query: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<serde_json::Value, ApiError> {
    // 设置默认值
    let limit = limit.unwrap_or(10);
    let offset = offset.unwrap_or(0);

    // 模拟搜索结果
    let results: Vec<serde_json::Value> = (0..limit.min(10))
        .map(|i| {
            serde_json::json!({
                "id": offset + i + 1,
                "name": format!("Match {} for '{}'", i + 1, query)
            })
        })
        .collect();

    Ok(serde_json::json!({
        "query": query,
        "limit": limit,
        "offset": offset,
        "results": results
    }))
}

/// 项目列表 API (带过滤和排序)
///
/// 演示多种查询参数组合：
/// - 布尔类型过滤
/// - 字符串类型过滤
/// - 排序参数
///
/// # 参数
/// - `published: Option<bool>` - 是否已发布
/// - `category: Option<String>` - 分类筛选
/// - `sort: Option<String>` - 排序字段
/// - `order: Option<String>` - 排序方向 (asc/desc)
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/items?published=true&category=tech&sort=created_at&order=desc"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "filters": {
///         "published": true,
///         "category": "tech",
///         "sort": "created_at",
///         "order": "desc"
///     },
///     "items": []
/// }
/// ```
#[service_api(
    name = "list_filtered_items",
    version = "v1",
    path = "/items",
    method = "GET",
    tool_name = "list_filtered_items",
    description = "获取过滤后的项目列表"
)]
async fn list_filtered_items(
    published: Option<bool>,
    category: Option<String>,
    sort: Option<String>,
    order: Option<String>,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "filters": {
            "published": published,
            "category": category,
            "sort": sort.unwrap_or_else(|| "id".to_string()),
            "order": order.unwrap_or_else(|| "asc".to_string())
        },
        "items": []
    }))
}

/// 获取时间范围数据
///
/// 演示日期范围查询：
/// - 开始日期
/// - 结束日期
///
/// # 参数
/// - `start_date: String` - 开始日期 (ISO 8601 格式)
/// - `end_date: String` - 结束日期 (ISO 8601 格式)
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/data/range?start_date=2024-01-01&end_date=2024-12-31"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "start_date": "2024-01-01",
///     "end_date": "2024-12-31",
///     "count": 365
/// }
/// ```
#[service_api(
    name = "get_data_range",
    version = "v1",
    path = "/data/range",
    method = "GET",
    tool_name = "get_data_range",
    description = "获取日期范围内的数据"
)]
async fn get_data_range(
    start_date: String,
    end_date: String,
) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "start_date": start_date,
        "end_date": end_date,
        "count": 0
    }))
}

/// 多值查询参数示例
///
/// 演示如何处理多个 ID 查询：
/// - 使用逗号分隔的值
/// - 解析为数组
///
/// # 参数
/// - `ids: String` - 逗号分隔的 ID 列表
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/items/batch?ids=1,2,3,4,5"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "ids": ["1", "2", "3", "4", "5"],
///     "items": [...]
/// }
/// ```
#[service_api(
    name = "get_batch_items",
    version = "v1",
    path = "/items/batch",
    method = "GET",
    tool_name = "get_batch_items",
    description = "批量获取项目"
)]
async fn get_batch_items(ids: String) -> Result<serde_json::Value, ApiError> {
    // 解析逗号分隔的 ID
    let id_list: Vec<&str> = ids.split(',').collect();

    let items: Vec<serde_json::Value> = id_list
        .iter()
        .enumerate()
        .map(|(i, id)| {
            serde_json::json!({
                "index": i,
                "id": id,
                "name": format!("Item {}", id)
            })
        })
        .collect();

    Ok(serde_json::json!({
        "ids": id_list,
        "items": items
    }))
}

/// 分页查询
///
/// 演示标准分页参数：
/// - 页码 (page)
/// - 每页大小 (page_size)
/// - 返回总数
///
/// # 参数
/// - `page: Option<u32>` - 页码 (默认 1)
/// - `page_size: Option<u32>` - 每页大小 (默认 20，最大 100)
///
/// # HTTP 用法
/// ```bash
/// curl "http://localhost:3000/api/v1/paginated?page=2&page_size=50"
/// ```
///
/// # 响应示例
/// ```json
/// {
///     "page": 2,
///     "page_size": 50,
///     "total": 1000,
///     "total_pages": 20,
///     "has_next": true,
///     "has_prev": true
/// }
/// ```
#[service_api(
    name = "get_paginated_data",
    version = "v1",
    path = "/paginated",
    method = "GET",
    tool_name = "get_paginated_data",
    description = "分页查询数据"
)]
async fn get_paginated_data(
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<serde_json::Value, ApiError> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).min(100);
    let total: u32 = 1000;
    let total_pages = total.div_ceil(page_size);

    Ok(serde_json::json!({
        "page": page,
        "page_size": page_size,
        "total": total,
        "total_pages": total_pages,
        "has_next": page < total_pages,
        "has_prev": page > 1
    }))
}
