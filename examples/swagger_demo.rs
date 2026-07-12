// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! SDForge Swagger UI 示例。
//!
//! 演示如何注册 HTTP 路由并集成 Swagger UI，启动 server 后访问
//! `/swagger-ui/` 查看交互式 API 文档。
//!
//! ## 运行
//!
//! ```sh
//! cargo run --example swagger_demo --features docs
//! # 然后访问：
//! #  - http://127.0.0.1:8080/swagger-ui/        (Swagger UI)
//! #  - http://127.0.0.1:8080/api-docs/openapi.json  (OpenAPI JSON)
//! ```

#![cfg(feature = "docs")]

use sdforge::core::ApiError;
use sdforge::forge;
use sdforge::swagger_ui_router;

/// Hello endpoint. 注册为 HTTP 路由 /api/v1/hello，会被 generate_openapi_spec()
/// 收集到 OpenAPI spec 中，Swagger UI 随之展示。
#[forge(
    name = "demo_hello",
    version = "v1",
    path = "/hello",
    method = "GET",
    description = "Hello endpoint for Swagger demo"
)]
async fn hello() -> Result<String, ApiError> {
    Ok("Hello from SDForge!".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // init_all_plugins 触碰 inventory 注册，确保 HTTP 路由 + OpenAPI 路由信息
    // 不被链接器优化掉。
    sdforge::init_all_plugins();

    // swagger_ui_router 挂载：
    //  - /api-docs/openapi.json  → OpenAPI 3.1 JSON（动态生成）
    //  - /swagger-ui/           → Swagger UI 首页
    //  - /swagger-ui/*rest      → Swagger UI 静态资源
    let app = swagger_ui_router();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("===========================================");
    println!("  SDForge Swagger UI Demo");
    println!("===========================================");
    println!();
    println!("Swagger UI:  http://127.0.0.1:8080/swagger-ui/");
    println!("OpenAPI JSON: http://127.0.0.1:8080/api-docs/openapi.json");
    println!();
    println!("按 Ctrl+C 停止");
    println!("===========================================");

    sdforge::axum::serve(listener, app).await?;
    Ok(())
}
