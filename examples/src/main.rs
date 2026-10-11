// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
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

use sdforge::i18n::t;

/// 主函数
///
/// 根据启用的 features 启动相应的服务
#[cfg(feature = "http_examples")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    // 显式引用 sdforge_examples 的子模块，确保 #[forge] 宏生成的 inventory
    // 段链接进当前 binary（否则 LTO 会移除未直接调用的代码，inventory 段会丢失）。
    #[cfg(feature = "http_examples")]
    {
        // basics / http / config 子模块（pub mod）会被解析为 use 语句的根命名空间。
        // 这里通过 take 一下 mod 内的 pub struct，强制子模块被包含进二进制。
        let _: fn() -> _ = || {
            let _: Option<sdforge_examples::basics::simple_api::UserResponse> = None;
            let _: Option<sdforge_examples::http::routing::path_params::UpdatePostRequest> = None;
            let _: Option<sdforge_examples::http::routing::query_params::SortOrder> = None;
        };
    }
    #[cfg(feature = "mcp_examples")]
    {
        let _: Option<sdforge_examples::mcp::tool_definition::GreetRequest> = None;
    }
    #[cfg(feature = "websocket_examples")]
    {
        let _: Option<sdforge_examples::websocket::basic::WsMessage> = None;
    }
    #[cfg(feature = "security_examples")]
    {
        let _: Option<sdforge_examples::security::api_key::AuthContext> = None;
    }

    let counts = sdforge::init_all_plugins();

    // 注册示例路由描述的简体中文译文（宿主侧运行时注册，先例见
    // tests/integration/openapi_tests.rs 的 i18n_key 探针）；英文为默认语言，
    // 未激活 zh locale 时协议消费点直接渲染英文源串。
    sdforge_examples::i18n::register_zh_translations();

    println!("===========================================");
    println!("       SDForge Examples Server");
    println!("===========================================");
    println!();
    println!(
        "{}",
        t("example-main-version", &[("version", "0.2.0".to_string())])
    );
    println!();
    println!("{}", t("example-main-registered-components", &[]));
    println!(
        "  {}",
        t(
            "example-main-http-routes",
            &[("count", counts.routes.to_string())]
        )
    );
    #[cfg(feature = "mcp_examples")]
    println!(
        "  {}",
        t(
            "example-main-mcp-tools",
            &[("count", counts.mcp_tools.to_string())]
        )
    );
    #[cfg(feature = "websocket_examples")]
    println!(
        "  {}",
        t(
            "example-main-ws-routes",
            &[("count", counts.ws_routes.to_string())]
        )
    );
    #[cfg(feature = "grpc_examples")]
    println!(
        "  {}",
        t(
            "example-main-grpc-routes",
            &[("count", counts.grpc_routes.to_string())]
        )
    );
    println!();

    let port: u16 = env::var("HTTP_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    println!("{}", t("example-main-starting-http", &[]));
    println!(
        "{}",
        t("example-main-listening", &[("port", port.to_string())])
    );
    println!();
    println!("{}", t("example-main-available-endpoints", &[]));
    println!("  {}", t("example-main-http-api", &[]));
    println!(
        "    GET  /api/v1/hello           - {}",
        t("example-main-endpoint-hello", &[])
    );
    println!(
        "    GET  /api/v1/users/:id       - {}",
        t("example-main-endpoint-user", &[])
    );
    println!(
        "    POST /api/v1/echo            - {}",
        t("example-main-endpoint-echo", &[])
    );
    println!();

    let app = sdforge::http::build();
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    println!("{}", t("example-main-server-started", &[]));
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
    println!("{}", t("example-main-enable-http", &[]));
    println!();
    println!("  cargo run --features http_examples");
    println!();
    println!("{}", t("example-main-available-modules", &[]));
    println!("  - basics:      {}", t("example-main-module-basics", &[]));
    println!("  - http:        {}", t("example-main-module-http", &[]));
    println!("  - mcp:         {}", t("example-main-module-mcp", &[]));
    println!(
        "  - websocket:   {}",
        t("example-main-module-websocket", &[])
    );
    println!(
        "  - streaming:   {}",
        t("example-main-module-streaming", &[])
    );
    println!(
        "  - security:    {}",
        t("example-main-module-security", &[])
    );
    println!(
        "  - combined:    {}",
        t("example-main-module-combined", &[])
    );
    println!();
    println!("{}", t("example-main-examples", &[]));
    println!("  # {}", t("example-main-run-basic-hint", &[]));
    println!("  cargo run --example sdforge --features http_examples");
    println!();
    println!("  # {}", t("example-main-run-combined-hint", &[]));
    println!("  cargo run --example sdforge --features combined_examples");
    println!("===========================================");
}
