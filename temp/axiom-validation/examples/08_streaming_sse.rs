//! 08_streaming_sse - 流式响应示例
//!
//! 这个示例演示如何使用 Axiom 框架的 SSE (Server-Sent Events) 流式响应功能。

use axiom::prelude::*;
use axiom::service_api;
use axiom::streaming::{create_stream_channel, StreamEvent};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StreamData {
    id: u64,
    message: String,
    timestamp: i64,
}

#[service_api(
    name = "stream_events",
    version = "v1",
    description = "Stream events via SSE",
    path = "/stream",
    method = "GET",
    stream = true
)]
async fn stream_events() -> Result<axiom::streaming::StreamResponse<StreamData>, ApiError> {
    let (tx, response) = create_stream_channel(32);

    tokio::spawn(async move {
        for i in 1..=10 {
            let data = StreamData {
                id: i,
                message: format!("Event {}", i),
                timestamp: chrono::Utc::now().timestamp(),
            };

            let _ = tx.send(Ok(data)).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    Ok(response)
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
    println!("Axiom 流式响应示例");
    println!("========================================");
    println!();

    println!("✅ SSE 流式响应已配置");
    println!();
    println!("📡 服务地址: http://0.0.0.0:8080");
    println!();
    println!("💡 测试流式响应:");
    println!("  curl -N http://localhost:8080/api/v1/stream");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    let router = axiom::http::build();
    let addr: SocketAddr = "0.0.0.0:8080".parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(listener, router).await?;

    Ok(())
}