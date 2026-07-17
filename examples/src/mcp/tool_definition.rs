// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # MCP 工具定义示例
//!
//! 本模块展示如何定义各种类型的 MCP 工具。
//!
//! ## 工具类型
//!
//! ### 1. 无参数工具
//!
//! ```json
//! {
//!     "tool": "hello",
//!     "input": {}
//! }
//! ```
//!
//! ### 2. 单参数工具
//!
//! ```json
//! {
//!     "tool": "greet",
//!     "input": {"name": "John"}
//! }
//! ```
//!
//! ### 3. 复杂参数工具
//!
//! ```json
//! {
//!     "tool": "process_data",
//!     "input": {
//!         "data": {"key": "value"},
//!         "options": {"transform": true}
//!     }
//! }
//! ```
//!
//! ## 响应格式
//!
//! MCP 工具响应通常是 JSON 格式：
//!
//! ```json
//! {
//!     "success": true,
//!     "result": {...}
//! }
//! ```
//!
//! 或者直接返回字符串：
//!
//! ```json
//! "Hello, World!"
//! ```

use sdforge::prelude::*;
use sdforge::serde::{Deserialize, Serialize};

// ============================================================================
// 请求类型定义
// ============================================================================

/// 问候请求
///
/// 用于多语言问候工具
#[derive(Debug, Deserialize, Serialize)]
pub struct GreetRequest {
    /// 被问候的人名
    pub name: String,
    /// 语言代码 (可选，默认英语)
    ///
    /// 支持的语言:
    /// - `en` - 英语
    /// - `es` - 西班牙语
    /// - `fr` - 法语
    /// - `de` - 德语
    pub language: Option<String>,
}

/// 数据处理请求
///
/// 用于演示复杂输入的处理
#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessRequest {
    /// 要处理的数据
    pub data: serde_json::Value,
    /// 处理选项 (可选)
    pub options: Option<serde_json::Value>,
}

// ============================================================================
// API 端点定义
// ============================================================================

/// 简单问候工具
///
/// 无参数的问候工具，返回固定消息。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "hello",
///     "input": {}
/// }
/// ```
///
/// # 响应
/// `"Hello from MCP!"`
#[forge(
    name = "mcp_hello",
    version = "v1",
    path = "/mcp/hello",
    method = "GET",
    tool_name = "hello",
    description = "简单的问候工具"
)]
async fn mcp_hello() -> Result<String, ApiError> {
    Ok("Hello from MCP!".to_string())
}

/// 多语言问候工具
///
/// 演示：
/// - 接收参数
/// - 条件逻辑
/// - 多语言支持
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "greet",
///     "input": {
///         "name": "John",
///         "language": "es"
///     }
/// }
/// ```
///
/// # 响应
/// `"Hola, John!"`
///
/// # 支持的语言
/// - `en`: Hello, {name}!
/// - `es`: Hola, {name}!
/// - `fr`: Bonjour, {name}!
/// - `de`: Hallo, {name}!
#[forge(
    name = "mcp_greet",
    version = "v1",
    path = "/mcp/greet",
    method = "POST",
    tool_name = "greet",
    description = "用指定语言问候某人"
)]
async fn mcp_greet(request: GreetRequest) -> Result<String, ApiError> {
    let greeting = match request.language.as_deref() {
        Some("es") => format!("Hola, {}!", request.name),
        Some("fr") => format!("Bonjour, {}!", request.name),
        Some("de") => format!("Hallo, {}!", request.name),
        Some("ja") => format!("こんにちは、{}!", request.name),
        Some("zh") => format!("你好，{}！", request.name),
        _ => format!("Hello, {}!", request.name),
    };

    Ok(greeting)
}

/// 数据获取工具
///
/// 根据 ID 获取数据项。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "get_data",
///     "input": {"id": 42}
/// }
/// ```
///
/// # 响应
/// ```json
/// {
///     "id": 42,
///     "name": "Data Item",
///     "value": 42
/// }
/// ```
#[forge(
    name = "mcp_get_data",
    version = "v1",
    path = "/mcp/data",
    method = "GET",
    tool_name = "get_data",
    description = "根据ID获取数据"
)]
async fn mcp_get_data(id: u64) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "id": id,
        "name": "Data Item",
        "value": id * 10,
        "description": format!("This is data item with ID {}", id)
    }))
}

/// 数据处理工具
///
/// 演示处理复杂 JSON 数据的工具。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "process_data",
///     "input": {
///         "data": {"key": "value", "count": 5},
///         "options": {
///             "transform": "uppercase",
///             "validate": true
///         }
///     }
/// }
/// ```
///
/// # 响应
/// ```json
/// {
///     "original_data": {"key": "value", "count": 5},
///     "processed": true,
///     "transformed": "uppercase",
///     "validated": true
/// }
/// ```
#[forge(
    name = "mcp_process",
    version = "v1",
    path = "/mcp/process",
    method = "POST",
    tool_name = "process_data",
    description = "处理数据并应用选项"
)]
async fn mcp_process(request: ProcessRequest) -> Result<serde_json::Value, ApiError> {
    let mut result = serde_json::json!({
        "original_data": request.data,
        "processed": true
    });

    // 处理选项
    if let Some(opts) = request.options {
        if let Some(transform) = opts.get("transform") {
            result["transformed"] = transform.clone();
        }
        if let Some(validate) = opts.get("validate").and_then(|v| v.as_bool()) {
            result["validated"] = serde_json::json!(validate);
        }
        if let Some(enrich) = opts.get("enrich").and_then(|v| v.as_bool()) {
            if enrich {
                result["enriched"] = serde_json::json!(true);
                result["enrichment_timestamp"] =
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339());
            }
        }
    }

    Ok(result)
}

/// 时间获取工具
///
/// 返回当前时间。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "get_time",
///     "input": {}
/// }
/// ```
///
/// # 响应
/// ```json
/// {
///     "timestamp": "2024-01-17T12:00:00Z",
///     "timezone": "UTC",
///     "formatted": "Wednesday, January 17, 2024"
/// }
/// ```
#[forge(
    name = "mcp_get_time",
    version = "v1",
    path = "/mcp/time",
    method = "GET",
    tool_name = "get_time",
    description = "获取当前时间"
)]
async fn mcp_get_time() -> Result<serde_json::Value, ApiError> {
    let now = chrono::Utc::now();
    Ok(serde_json::json!({
        "timestamp": now.to_rfc3339(),
        "timezone": "UTC",
        "unix_timestamp": now.timestamp(),
        "formatted": now.format("%A, %B %d, %Y").to_string()
    }))
}

/// 计算器工具
///
/// 演示简单的算术运算。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "calculate",
///     "input": {
///         "operation": "add",
///         "a": 10,
///         "b": 20
///     }
/// }
/// ```
///
/// # 支持的操作
/// - `add` - 加法
/// - `subtract` - 减法
/// - `multiply` - 乘法
/// - `divide` - 除法
#[derive(Debug, Deserialize, Serialize)]
pub struct CalculateRequest {
    /// 操作类型: add, subtract, multiply, divide
    pub operation: String,
    /// 第一个操作数
    pub a: f64,
    /// 第二个操作数
    pub b: f64,
}

#[forge(
    name = "mcp_calculate",
    version = "v1",
    path = "/mcp/calculate",
    method = "POST",
    tool_name = "calculate",
    description = "执行数学运算"
)]
async fn mcp_calculate(request: CalculateRequest) -> Result<serde_json::Value, ApiError> {
    let result = match request.operation.as_str() {
        "add" => request.a + request.b,
        "subtract" => request.a - request.b,
        "multiply" => request.a * request.b,
        "divide" => {
            if request.b == 0.0 {
                return Err(ApiError::InvalidInput {
                    message: "除数不能为零".to_string(),
                    field: Some("b".to_string()),
                    value: Some(serde_json::json!(request.b)),
                });
            }
            request.a / request.b
        }
        _ => {
            return Err(ApiError::InvalidInput {
                message: format!("不支持的操作: {}", request.operation),
                field: Some("operation".to_string()),
                value: Some(serde_json::json!(request.operation)),
            });
        }
    };

    Ok(serde_json::json!({
        "operation": request.operation,
        "a": request.a,
        "b": request.b,
        "result": result
    }))
}
