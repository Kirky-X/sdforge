// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! # MCP 工具注册示例
//!
//! 本模块展示 MCP 工具的注册机制和最佳实践。
//!
//! ## 自动注册
//!
//! SDForge 使用 `inventory` 库实现自动注册。所有带有 `tool_name` 属性的
//! `#[forge]` 函数会自动注册到 MCP 服务器。
//!
//! ## 注册流程
//!
//! 1. 编译时：`#[forge]` 宏生成注册代码
//! 2. 运行时：`inventory::submit!` 注册工具
//! 3. 启动时：`init_all_plugins()` 收集所有注册的工具
//!
//! ## 工具组织
//!
//! ### 按功能分组
//!
//! 将相关工具放在同一模块中：
//!
//! ```text
//! // calc.rs
//! mod calculator {
//!     pub fn add(request: ...) -> Result<...> { ... }
//!     pub fn subtract(request: ...) -> Result<...> { ... }
//!     pub fn multiply(request: ...) -> Result<...> { ... }
//! }
//!
//! // 注册
//! #[forge(tool_name = "add", ...)]
//! async fn add(...) { calculator::add(...) }
//! ```
//!
//! ### HTTP Only 端点
//!
//! 不指定 `tool_name` 的端点不会注册为 MCP 工具：
//!
//! ```text
//! # use sdforge::prelude::ApiError;
//! #[forge(
//!     name = "http_only",
//!     tool_name = ...  // 不指定 tool_name
//! )]
//! async fn http_only() -> Result<String, ApiError> {
//!     Ok("HTTP only".to_string())
//! }
//! ```
//!
//! ## 最佳实践
//!
//! 1. **清晰的工具名称** - 使用动词 + 名词，如 `get_user`
//! 2. **详细的描述** - 帮助 AI 理解工具用途
//! 3. **一致的参数命名** - 使用 snake_case
//! 4. **类型安全** - 使用强类型参数

use sdforge::i18n::t;
use sdforge::prelude::*;
use sdforge::serde::{Deserialize, Serialize};

// ============================================================================
// 计算器工具请求类型
// ============================================================================

/// 加法请求
#[derive(Debug, Deserialize, Serialize)]
pub struct AddRequest {
    /// 第一个数
    pub a: f64,
    /// 第二个数
    pub b: f64,
}

/// 减法请求
#[derive(Debug, Deserialize, Serialize)]
pub struct SubtractRequest {
    /// 被减数
    pub a: f64,
    /// 减数
    pub b: f64,
}

/// 乘法请求
#[derive(Debug, Deserialize, Serialize)]
pub struct MultiplyRequest {
    /// 第一个因数
    pub a: f64,
    /// 第二个因数
    pub b: f64,
}

/// 除法请求
#[derive(Debug, Deserialize, Serialize)]
pub struct DivideRequest {
    /// 被除数
    pub a: f64,
    /// 除数
    pub b: f64,
}

// ============================================================================
// 计算器工具 (自动注册)
// ============================================================================

/// 加法工具
///
/// 将两个数相加。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "add",
///     "input": {"a": 10, "b": 20}
/// }
/// ```
///
/// # 响应
/// ```json
/// {"result": 30}
/// ```
#[forge(
    name = "mcp_calc_add",
    version = "v1",
    path = "/mcp/calc/add",
    method = "POST",
    tool_name = "add",
    description = "Add two numbers",
    i18n_key = "example.tool_registration.mcp_calc_add.description"
)]
async fn mcp_add(request: AddRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "result": request.a + request.b
    }))
}

/// 减法工具
///
/// 将两个数相减。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "subtract",
///     "input": {"a": 10, "b": 20}
/// }
/// ```
///
/// # 响应
/// ```json
/// {"result": -10}
/// ```
#[forge(
    name = "mcp_calc_subtract",
    version = "v1",
    path = "/mcp/calc/subtract",
    method = "POST",
    tool_name = "subtract",
    description = "Subtract two numbers",
    i18n_key = "example.tool_registration.mcp_calc_subtract.description"
)]
async fn mcp_subtract(request: SubtractRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "result": request.a - request.b
    }))
}

/// 乘法工具
///
/// 将两个数相乘。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "multiply",
///     "input": {"a": 10, "b": 20}
/// }
/// ```
///
/// # 响应
/// ```json
/// {"result": 200}
/// ```
#[forge(
    name = "mcp_calc_multiply",
    version = "v1",
    path = "/mcp/calc/multiply",
    method = "POST",
    tool_name = "multiply",
    description = "Multiply two numbers",
    i18n_key = "example.tool_registration.mcp_calc_multiply.description"
)]
async fn mcp_multiply(request: MultiplyRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "result": request.a * request.b
    }))
}

/// 除法工具
///
/// 将两个数相除。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "divide",
///     "input": {"a": 10, "b": 20}
/// }
/// ```
///
/// # 响应
/// ```json
/// {"result": 0.5}
/// ```
///
/// # 错误
/// 除数为零时返回错误。
#[forge(
    name = "mcp_calc_divide",
    version = "v1",
    path = "/mcp/calc/divide",
    method = "POST",
    tool_name = "divide",
    description = "Divide two numbers",
    i18n_key = "example.tool_registration.mcp_calc_divide.description"
)]
async fn mcp_divide(request: DivideRequest) -> Result<serde_json::Value, ApiError> {
    if request.b == 0.0 {
        return Err(ApiError::InvalidInput {
            message: t("example-mcp-tool-registration-divisor-zero", &[]),
            field: Some("b".to_string()),
            value: Some(serde_json::json!(request.b)),
        });
    }

    Ok(serde_json::json!({
        "result": request.a / request.b
    }))
}

// ============================================================================
// HTTP Only 端点 (不注册为 MCP 工具)
// ============================================================================

/// HTTP 仅端点
///
/// 此端点不指定 `tool_name`，因此不会被注册为 MCP 工具。
/// 只能通过 HTTP 访问。
///
/// # HTTP 调用
/// ```bash
/// curl http://localhost:3000/api/v1/http-only
/// ```
///
/// # MCP 调用
/// 此工具不可通过 MCP 调用。
#[forge(
    name = "http_only",
    version = "v1",
    path = "/http-only",
    method = "GET",
    description = "HTTP-only endpoint (not registered as an MCP tool)",
    i18n_key = "example.tool_registration.http_only.description"
)]
async fn http_only() -> Result<String, ApiError> {
    Ok("This is HTTP only".to_string())
}

/// 内部数据端点
///
/// 仅通过 HTTP 访问的内部管理端点。
///
/// # HTTP 调用
/// ```bash
/// curl http://localhost:3000/api/v1/internal/stats
/// ```
#[forge(
    name = "internal_stats",
    version = "v1",
    path = "/internal/stats",
    method = "GET",
    description = "Internal stats info",
    i18n_key = "example.tool_registration.internal_stats.description"
)]
async fn internal_stats() -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "total_requests": 1000,
        "active_connections": 50,
        "uptime_seconds": 3600,
        "memory_usage_mb": 128
    }))
}

// ============================================================================
// 字符串处理工具
// ============================================================================

/// 字符串反转工具
///
/// 反转输入字符串。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "reverse_string",
///     "input": {"text": "Hello"}
/// }
/// ```
///
/// # 响应
/// ```json
/// {"result": "olleH"}
/// ```
#[derive(Debug, Deserialize, Serialize)]
pub struct ReverseRequest {
    pub text: String,
}

#[forge(
    name = "mcp_reverse_string",
    version = "v1",
    path = "/mcp/string/reverse",
    method = "POST",
    tool_name = "reverse_string",
    description = "Reverse a string",
    i18n_key = "example.tool_registration.mcp_reverse_string.description"
)]
async fn reverse_string(request: ReverseRequest) -> Result<serde_json::Value, ApiError> {
    let reversed: String = request.text.chars().rev().collect();
    Ok(serde_json::json!({
        "original": request.text,
        "result": reversed
    }))
}

/// 字符串转大写工具
///
/// 将输入字符串转换为大写。
///
/// # MCP 调用
/// ```json
/// {
///     "tool": "uppercase",
///     "input": {"text": "Hello"}
/// }
/// ```
///
/// # 响应
/// ```json
/// {"result": "HELLO"}
/// ```
#[derive(Debug, Deserialize, Serialize)]
pub struct UppercaseRequest {
    pub text: String,
}

#[forge(
    name = "mcp_uppercase",
    version = "v1",
    path = "/mcp/string/uppercase",
    method = "POST",
    tool_name = "uppercase",
    description = "Convert a string to uppercase",
    i18n_key = "example.tool_registration.mcp_uppercase.description"
)]
async fn uppercase(request: UppercaseRequest) -> Result<serde_json::Value, ApiError> {
    Ok(serde_json::json!({
        "original": request.text,
        "result": request.text.to_uppercase()
    }))
}
