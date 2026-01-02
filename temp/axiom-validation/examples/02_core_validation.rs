//! 02_core_validation - 验证 axiom 核心功能
//!
//! 运行方式:
//! ```bash
//! cargo run --bin 02_core_validation
//! ```

use axiom::prelude::*;
use axiom::config::AppConfig;
use axiom::http;
use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Axiom Core Validation ===\n");
    
    // 1. 测试 ApiError
    println!("1. Testing ApiError types...");
    let error = ApiError::NotFound {
        resource: "User".to_string(),
        resource_id: Some("123".to_string()),
    };
    let mcp_json = error.to_mcp_json();
    println!("   ApiError to MCP JSON: {}", mcp_json);
    assert!(mcp_json.contains("NOT_FOUND"));
    println!("   ✓ ApiError works\n");
    
    // 2. 测试 ServiceResponse
    println!("2. Testing ServiceResponse...");
    let response = ServiceResponse::success(User {
        id: 1,
        name: "Alice".to_string(),
    });
    let json = serde_json::to_string(&response)?;
    println!("   ServiceResponse JSON: {}", json);
    println!("   ✓ ServiceResponse works\n");
    
    // 3. 测试 AppConfig
    println!("3. Testing AppConfig...");
    let config = AppConfig::default();
    println!("   Server host: {}", config.server.host);
    println!("   Server port: {}", config.server.port);
    println!("   ✓ AppConfig works\n");
    
    // 4. 测试 HTTP router 构建
    println!("4. Testing HTTP router build...");
    let router = http::build();
    println!("   ✓ HTTP router built successfully\n");
    
    // 5. 测试 HTTP router with config
    println!("5. Testing HTTP router with config...");
    let result = http::build_with_config(&config);
    assert!(result.is_ok());
    println!("   ✓ HTTP router with config works\n");
    
    // 6. 测试 HTTP router with redirect
    println!("6. Testing HTTP router with redirect...");
    let router = http::build_with_redirect();
    println!("   ✓ HTTP router with redirect works\n");
    
    // 7. 创建简单的 HTTP 服务器
    println!("7. Creating simple HTTP server...");
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/message", get(get_message));
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("   Server listening on http://127.0.0.1:8080");
    println!("   Try: curl http://127.0.0.1:8080/health");
    println!("   Try: curl http://127.0.0.1:8080/message\n");
    
    println!("=== All validations passed! ===\n");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn health_check() -> Json<ServiceResponse<String>> {
    Json(ServiceResponse::success("OK".to_string()))
}

async fn get_message() -> Json<ServiceResponse<Message>> {
    Json(ServiceResponse::success(Message {
        text: "Hello from Axiom!".to_string(),
    }))
}
