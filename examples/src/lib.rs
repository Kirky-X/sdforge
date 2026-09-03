// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! # SDForge Examples Library
//!
//! 本库包含展示 SDForge 框架功能的示例模块。
//!
//! ## 模块概览
//!
//! | 模块 | 说明 | 所需 feature |
//! |------|------|-------------|
//! | [`basics`] | 基础示例 | `http_examples` |
//! | [`http`] | HTTP 协议 | `http_examples` |
//! | [`mcp`] | MCP 协议 | `mcp_examples` |
//! | [`websocket`] | WebSocket | `websocket_examples` |
//! | [`streaming`] | 流式传输 | `streaming_examples` |
//! | [`security`] | 安全功能 | `security_examples` |
//! | [`cache`] | 缓存功能 | `cache_examples` |
//! | [`grpc`] | gRPC 协议 | `grpc_examples` |
//! | [`config`] | 配置管理 | `http_examples` |
//! | [`logging`] | 结构化日志 | `logging_examples` |
//! | [`openapi`] | OpenAPI 生成 | `openapi_examples` |
//! | [`combined`] | 组合示例 | `combined_examples` |
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
//! ```text
//! use sdforge_examples::basics::simple_api::{get_hello, get_user};
//! use sdforge_examples::http::routing::path_params::get_user_by_id;
//! ```
//!
//! ## 功能特性
//!
//! 通过 Cargo features 控制编译，每个示例类别启用对应的 sdforge feature：
//!
//! ```toml
//! [features]
//! default = ["http_examples"]
//! http_examples = ["sdforge/http"]
//! mcp_examples = ["sdforge/mcp"]
//! websocket_examples = ["sdforge/websocket"]
//! streaming_examples = ["sdforge/streaming"]
//! security_examples = ["sdforge/security"]
//! cache_examples = ["sdforge/cache", "sdforge/http"]
//! grpc_examples = ["sdforge/grpc"]
//! logging_examples = ["sdforge/logging"]
//! openapi_examples = ["sdforge/openapi", "http_examples"]
//! combined_examples = ["http_examples", "mcp_examples", "websocket_examples",
//!                      "streaming_examples", "security_examples", "openapi_examples"]
//! ```

//! # Crate 级 lint 配置
//!
//! 示例库的 `main()` / `demo_*()` 等函数用于文档展示和教学目的，在 lib
//! 构建中不会被直接调用，因此允许 `dead_code`。`#[forge]` 宏生成
//! 的 `cfg(feature = "mcp"/"websocket"/...)` 引用的是 sdforge crate 的
//! features（而非 examples crate 的 `*_examples` features），因此允许
//! `unexpected_cfgs` 以避免误报警告。
#![allow(dead_code, unexpected_cfgs)]

// ============================================================================
// 模块导出 — 每个模块由对应的 feature 门控
// ============================================================================

/// 基础示例模块
///
/// 包含核心功能示例：
/// - [`simple_api`](basics::simple_api) - 简单 API 定义
/// - [`types_and_errors`](basics::types_and_errors) - 类型和错误处理
/// - [`response_building`](basics::response_building) - 响应构建
#[cfg(feature = "http_examples")]
pub mod basics;

/// HTTP 协议示例模块
///
/// 包含 HTTP 功能示例：
/// - [`routing`](http::routing) - 路由配置
///   - [`path_params`](http::routing::path_params) - 路径参数
///   - [`query_params`](http::routing::query_params) - 查询参数
/// - [`middleware`](http::middleware) - 中间件
///   - [`cors`](http::middleware::cors) - CORS 跨域
#[cfg(feature = "http_examples")]
pub mod http;

/// MCP 协议示例模块
///
/// 包含 MCP 功能示例：
/// - [`tool_definition`](mcp::tool_definition) - 工具定义
/// - [`tool_registration`](mcp::tool_registration) - 工具注册
#[cfg(feature = "mcp_examples")]
pub mod mcp;

/// WebSocket 示例模块
///
/// 包含 WebSocket 功能示例：
/// - [`basic`](websocket::basic) - 基础连接
/// - [`chat`](websocket::chat) - 聊天功能
#[cfg(feature = "websocket_examples")]
pub mod websocket;

/// 流式传输示例模块
///
/// 包含流式传输示例：
/// - [`sse`](streaming::sse) - Server-Sent Events
#[cfg(feature = "streaming_examples")]
pub mod streaming;

/// 安全功能示例模块
///
/// 包含安全功能示例：
/// - [`api_key`](security::api_key) - API Key 认证
/// - [`comprehensive`](security::comprehensive) - 综合安全示例
#[cfg(feature = "security_examples")]
pub mod security;

/// 缓存功能示例模块
///
/// 包含缓存功能示例：
/// - [`performance`](cache::performance) - 缓存与性能优化
#[cfg(feature = "cache_examples")]
pub mod cache;

/// gRPC 协议示例模块
///
/// 包含 gRPC 功能示例：
/// - [`server`](grpc::server) - gRPC 服务端构建与路由注册
#[cfg(feature = "grpc_examples")]
pub mod grpc;

/// 配置管理示例模块
///
/// 包含配置功能示例：
/// - [`app_config`](config::app_config) - 应用配置构建与加载
#[cfg(feature = "http_examples")]
pub mod config;

/// 结构化日志示例模块
///
/// 包含日志功能示例：
/// - [`structured`](logging::structured) - 结构化日志记录
#[cfg(feature = "logging_examples")]
pub mod logging;

/// OpenAPI 自动生成示例模块
///
/// 包含 OpenAPI 功能示例：
/// - [`basic`](openapi::basic) - OpenAPI 基础用法（默认规范生成、自定义 builder、手动注册路由）
#[cfg(feature = "openapi_examples")]
pub mod openapi;

/// 组合示例模块
///
/// 展示多种功能组合使用：
/// - [`full_example`](combined::full_example) - 完整功能示例
#[cfg(feature = "combined_examples")]
pub mod combined;

// ============================================================================
// 重新导出 sdforge prelude
// ============================================================================

/// 重新导出 sdforge 的 prelude 模块
///
/// 方便在示例中使用框架类型
pub use sdforge::prelude::*;
