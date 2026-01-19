//! SDForge Examples - Main Entry Point
//!
//! This binary runs a simple HTTP server demonstrating SDForge functionality.
//!
//! # Usage
//!
//! ```bash
//! # Run with HTTP support
//! cargo run --features http
//!
//! # Run with all features
//! cargo run --features full
//! ```

// Import only basic module to avoid route conflicts
#[cfg(feature = "http")]
mod basic;

#[cfg(feature = "http")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("SDForge Examples Server");
    println!("========================");
    println!();
    println!("Starting HTTP server on http://0.0.0.0:3000");
    println!();
    println!("Available endpoints:");
    println!("  GET /api/v1/hello       - Basic greeting");
    println!("  GET /api/v1/users/:id   - Get user by ID");
    println!("  POST /api/v1/echo       - Echo request body");
    println!();

    let app = sdforge::http::build();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    sdforge::axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(not(feature = "http"))]
fn main() {
    println!("SDForge Examples");
    println!("================");
    println!();
    println!("To run the HTTP server, enable the 'http' feature:");
    println!("  cargo run --features http");
    println!();
    println!("Available modules:");
    println!("  - basic: Core API definitions and error handling");
    println!("  - http: HTTP protocol examples");
    println!("  - mcp: MCP protocol examples");
    println!("  - security: Authentication and authorization");
    println!("  - cache: Caching examples");
    println!("  - config: Configuration management");
    println!("  - streaming: SSE streaming examples");
    println!("  - websocket: WebSocket examples");
    println!("  - grpc: gRPC examples");
    println!("  - logging: Logging examples");
    println!("  - combined: Full example applications");
}
