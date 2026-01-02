//! 07_security_auth - 安全认证示例
//!
//! 这个示例演示如何使用 Axiom 框架的安全认证功能。

use axiom::prelude::*;
use axiom::service_api;
use axiom::security::{ApiKeyAuth, RateLimiter, auth_middleware, rate_limit_middleware};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretData {
    message: String,
}

#[service_api(
    name = "get_secret",
    version = "v1",
    description = "Get secret data (requires authentication)",
    path = "/secret",
    method = "GET"
)]
async fn get_secret() -> Result<SecretData, ApiError> {
    Ok(SecretData {
        message: "This is a secret message".to_string(),
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
    println!("Axiom 安全认证示例");
    println!("========================================");
    println!();

    // 创建 API Key 认证
    let auth = Arc::new(ApiKeyAuth::new());
    auth.add_key("secret-api-key-123", vec!["read".to_string(), "write".to_string()]);

    println!("✅ API Key 认证已配置");
    println!("  API Key: secret-api-key-123");
    println!();

    // 创建速率限制器
    let limiter = Arc::new(RateLimiter::new(None));

    println!("✅ 速率限制器已配置");
    println!("  限制: 100 请求 / 60 秒");
    println!();

    println!("📡 服务地址: http://0.0.0.0:8080");
    println!();
    println!("💡 测试认证:");
    println!("  curl -H \"X-API-Key: secret-api-key-123\" \\");
    println!("       http://localhost:8080/api/v1/secret");
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