// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//!
//! # SDForge Examples - 主入口
//!
//! 本程序展示了 SDForge 框架的基本用法。
//!
//! ## 功能特性
//!
//! - HTTP 服务器 - 处理 REST API 请求
//! - MCP 工具 - 支持 AI 模型调用
//! - WebSocket - 实时双向通信
//! - SSE 流 - 服务器推送事件
//! - 安全认证 - API Key 和速率限制
//!
//! ## 启动方式
//!
//! ```bash
//! # 启动 HTTP 服务器 (基础)
//! cargo run --features http_examples
//!
//! # 启动完整功能
//! cargo run --features combined_examples
//!
//! # 自定义端口
//! HTTP_PORT=8080 cargo run --features http_examples
//! ```
//!
//! ## 可用端点
//!
//! ### HTTP API
//!
//! | 端点 | 方法 | 说明 |
//! |------|------|------|
//! | `/api/v1/hello` | GET | 问候语 |
//! | `/api/v1/users/:id` | GET | 获取用户 |
//! | `/api/v1/echo` | POST | 回显请求 |
//!
//! ### WebSocket
//!
//! | 端点 | 说明 |
//! |------|------|
//! | `/ws/basic` | 基础连接 |
//! | `/ws/chat` | 聊天服务 |
//!
//! ### SSE 流
//!
//! | 端点 | 说明 |
//! |------|------|
//! | `/stream/events` | 事件流 |
//! | `/stream/progress` | 进度流 |

#![allow(unexpected_cfgs)]

/// 主函数
///
/// 根据启用的 features 启动相应的服务
#[cfg(feature = "http_examples")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    // 初始化所有注册的插件
    let counts = sdforge::init_all_plugins();

    println!("===========================================");
    println!("       SDForge Examples Server");
    println!("===========================================");
    println!();
    println!("版本: 0.2.0");
    println!();
    println!("已注册的组件:");
    println!("  - HTTP 路由: {}", counts.routes);
    #[cfg(feature = "mcp_examples")]
    println!("  - MCP 工具: {}", counts.mcp_tools);
    #[cfg(feature = "websocket_examples")]
    println!("  - WebSocket 路由: {}", counts.ws_routes);
    #[cfg(feature = "grpc_examples")]
    println!("  - gRPC 路由: {}", counts.grpc_routes);
    println!();

    // 获取端口配置
    let port: u16 = env::var("HTTP_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    println!("启动 HTTP 服务器...");
    println!("监听地址: http://0.0.0.0:{}", port);
    println!();
    println!("可用端点:");
    println!("  HTTP API:");
    println!("    GET  /api/v1/hello           - 问候语");
    println!("    GET  /api/v1/users/:id       - 获取用户");
    println!("    POST /api/v1/echo             - 回显请求");
    println!();

    // 构建并启动服务器
    let app = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    println!("服务器已启动，按 Ctrl+C 停止");
    println!("===========================================");

    sdforge::axum::serve(listener, app).await?;

    Ok(())
}

/// 非 HTTP 特性下的主函数
///
/// 显示提示信息，指导用户启用正确的 features
#[cfg(not(feature = "http_examples"))]
fn main() {
    println!("===========================================");
    println!("       SDForge Examples");
    println!("===========================================");
    println!();
    println!("请启用 HTTP 功能来启动服务器:");
    println!();
    println!("  cargo run --features http_examples");
    println!();
    println!("可用模块:");
    println!("  - basics:      基础 API 定义和类型");
    println!("  - http:        HTTP 路由和参数");
    println!("  - mcp:         MCP 工具定义");
    println!("  - websocket:   WebSocket 连接");
    println!("  - streaming:   SSE 流式传输");
    println!("  - security:    API Key 和速率限制");
    println!("  - combined:    完整功能示例");
    println!();
    println!("示例:");
    println!("  # 运行基础示例");
    println!("  cargo run --example sdforge --features http_examples");
    println!();
    println!("  # 运行组合示例");
    println!("  cargo run --example sdforge --features combined_examples");
    println!("===========================================");
}
