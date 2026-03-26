// Copyright (c) 2026 Kirky.X
//!
//! # SDForge Examples Library
//!
//! 本库包含展示 SDForge 框架功能的示例模块。
//!
//! ## 模块概览
//!
//! | 模块 | 说明 | 功能 |
//! |------|------|------|
//! | [`basics`](basics) | 基础示例 | 简单 API、类型系统、响应构建 |
//! | [`http`](http) | HTTP 协议 | 路由、查询参数、中间件 |
//! | [`mcp`](mcp) | MCP 协议 | 工具定义、工具注册 |
//! | [`websocket`](websocket) | WebSocket | 基础连接、聊天功能 |
//! | [`streaming`](streaming) | 流式传输 | SSE 事件流 |
//! | [`security`](security) | 安全功能 | API Key、速率限制 |
//! | [`combined`](combined) | 组合示例 | 全功能集成 |
//!
//! ## 使用方式
//!
//! ### 启用特定模块
//!
//! ```toml
//! # Cargo.toml
//! [dependencies]
//! sdforge-examples = { features = ["http_examples"] }
//! ```
//!
//! ### 使用示例代码
//!
//! ```rust,ignore
//! use sdforge_examples::basics::simple_api::{get_hello, get_user};
//! use sdforge_examples::http::routing::path_params::get_user_by_id;
//! ```
//!
//! ## 功能特性
//!
//! 通过 Cargo features 控制编译：
//!
//! ```toml
//! [features]
//! default = []
//! http_examples = ["sdforge/http"]
//! mcp_examples = ["sdforge/mcp"]
//! websocket_examples = ["sdforge/websocket"]
//! streaming_examples = ["sdforge/streaming"]
//! security_examples = ["sdforge/security"]
//! cache_examples = ["sdforge/cache"]
//! config_examples = ["sdforge/http", "sdforge/hot-reload"]
//! logging_examples = ["sdforge/logging"]
//! combined_examples = ["http_examples", "mcp_examples", "websocket_examples",
//!                      "grpc_examples", "streaming_examples", "security_examples"]
//! ```

// ============================================================================
// 模块导出
// ============================================================================

/// 基础示例模块
///
/// 包含核心功能示例：
/// - [`simple_api`](basics::simple_api) - 简单 API 定义
/// - [`types_and_errors`](basics::types_and_errors) - 类型和错误处理
/// - [`response_building`](basics::response_building) - 响应构建
pub mod basics;

/// HTTP 协议示例模块
///
/// 包含 HTTP 功能示例：
/// - [`routing`](http::routing) - 路由配置
///   - [`path_params`](http::routing::path_params) - 路径参数
///   - [`query_params`](http::routing::query_params) - 查询参数
/// - [`middleware`](http::middleware) - 中间件
///   - [`cors`](http::middleware::cors) - CORS 跨域
pub mod http;

/// MCP 协议示例模块
///
/// 包含 MCP 功能示例：
/// - [`tool_definition`](mcp::tool_definition) - 工具定义
/// - [`tool_registration`](mcp::tool_registration) - 工具注册
pub mod mcp;

/// WebSocket 示例模块
///
/// 包含 WebSocket 功能示例：
/// - [`basic`](websocket::basic) - 基础连接
/// - [`chat`](websocket::chat) - 聊天功能
pub mod websocket;

/// 流式传输示例模块
///
/// 包含流式传输示例：
/// - [`sse`](streaming::sse) - Server-Sent Events
pub mod streaming;

/// 安全功能示例模块
///
/// 包含安全功能示例：
/// - [`api_key`](security::api_key) - API Key 认证
/// - [`rate_limiting`](security::rate_limiting) - 速率限制
pub mod security;

/// 组合示例模块
///
/// 展示多种功能组合使用：
/// - [`full_example`](combined::full_example) - 完整功能示例
pub mod combined;

// ============================================================================
// 重新导出 sdforge prelude
// ============================================================================

/// 重新导出 sdforge 的 prelude 模块
///
/// 方便在示例中使用框架类型
pub use sdforge::prelude::*;
