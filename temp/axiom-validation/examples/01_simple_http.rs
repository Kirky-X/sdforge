//! 01_simple_http - 简单的 HTTP 示例
//!
//! 运行方式:
//! ```bash
//! cargo run --bin 01_simple_http
//! ```

use axiom::prelude::*;
use axiom::service_api;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    text: String,
}

#[service_api(
    name = "hello",
    version = "v1",
    description = "Say hello",
    path = "/hello",
    method = "GET"
)]
async fn hello() -> Result<Message, ApiError> {
    Ok(Message {
        text: "Hello, World!".to_string(),
    })
}

#[service_api(
    name = "echo",
    version = "v1",
    description = "Echo message",
    path = "/echo",
    method = "POST"
)]
async fn echo(message: Message) -> Result<Message, ApiError> {
    Ok(message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting simple HTTP server on http://127.0.0.1:8080");
    println!("Try: curl http://127.0.0.1:8080/api/v1/hello");
    
    let router = axiom::http::build();
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, router).await?;
    
    Ok(())
}
