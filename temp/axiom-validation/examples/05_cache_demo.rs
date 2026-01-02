//! 05_cache_demo - 缓存功能示例
//!
//! 这个示例演示如何使用 Axiom 框架的 HTTP 响应缓存功能。

use axiom::cache::{CacheConfig, CacheMiddleware};
use axiom::http::build_with_config;
use axiom::prelude::*;
use axiom::service_api;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Product {
    id: u64,
    name: String,
    price: f64,
}

#[service_api(
    name = "get_product",
    version = "v1",
    description = "Get product by ID",
    path = "/products/:id",
    method = "GET",
    cache_ttl = 60
)]
async fn get_product(id: u64) -> Result<Product, ApiError> {
    // 模拟数据库查询
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok(Product {
        id,
        name: format!("Product {}", id),
        price: id as f64 * 10.0,
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
    println!("Axiom 缓存功能示例");
    println!("========================================");
    println!();

    let config = CacheConfig::default();
    println!("✅ 缓存配置: TTL={}秒, 最大大小={}MB",
        config.ttl_seconds, config.max_size_bytes / 1024 / 1024);
    println!();
    println!("📡 服务地址: http://0.0.0.0:8080");
    println!();
    println!("💡 测试缓存:");
    println!("  curl -I http://localhost:8080/api/v1/products/1");
    println!("  # 第一次请求会返回 200，后续请求会返回 304");
    println!();
    println!("按 Ctrl+C 停止服务");
    println!("========================================");
    println!();

    let router = axiom::http::build();
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, router).await?;

    Ok(())
}