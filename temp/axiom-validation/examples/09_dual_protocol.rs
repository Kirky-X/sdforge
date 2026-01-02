//! 09_dual_protocol - 双协议示例
//!
//! 这个示例演示如何同时使用 HTTP 和 MCP 协议。

use axiom::prelude::*;
use axiom::service_api;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Status {
    http: bool,
    mcp: bool,
    timestamp: i64,
}

#[service_api(
    name = "status",
    version = "v1",
    description = "Get service status",
    path = "/status",
    method = "GET",
    tool_name = "status"
)]
async fn status() -> Result<Status, ApiError> {
    Ok(Status {
        http: true,
        mcp: true,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    println!("========================================");
    println!("Axiom 双协议示例");
    println!("========================================");
    println!();

    // 同时构建 HTTP 和 MCP 服务
    let _http_router = axiom::http::build();
    let _mcp_server = axiom::mcp::build().await;

    println!("✅ 双协议服务已启动");
    println!();
    println!("📡 服务地址:");
    println!("  HTTP: http://localhost:8080");
    println!("  MCP:  stdio (交互模式)");
    println!();
    println!("💡 测试 HTTP:");
    println!("  curl http://localhost:8080/api/v1/status");
    println!();
    println!("💡 测试 MCP:");
    println!("  通过 MCP 客户端调用 status 工具");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    println!("双协议服务运行中...");
    tokio::signal::ctrl_c().await?;
    println!("\n👋 双协议服务已停止");

    Ok(())
}