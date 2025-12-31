//! README.md - Axiom Multi-Protocol SDK Framework
//!
//! # Axiom
//!
//! **Axiom** is a Rust-based declarative SDK framework that uses procedural macros
//! to automatically generate multi-protocol service interfaces (HTTP + MCP) from
//! unified function annotations. The key innovation is compile-time protocol selection
//! via Cargo features—unused protocols produce zero compiled code.
//!
//! ## Features
//!
//! - **Unified Interface Definition**: Single macro configuration for both HTTP and MCP
//! - **Compile-Time Protocol Selection**: Feature-gated code generation
//! - **Zero Runtime Overhead**: Unused protocols don't exist in the binary
//! - **Type Safety**: Compile-time validation of interface definitions
//! - **Easy Integration**: Works as a library in any Rust project
//!
//! ## Quick Start
//!
//! Add Axiom to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! axiom = { version = "0.1", features = ["http"] }
//! ```
//!
//! Define your API with a single macro:
//!
//! ```rust
//! use axiom::prelude::*;
//!
//! #[service_api(
//!     name = "get_user",
//!     version = "v1",
//!     path = "/users/:id",
//!     method = "GET",
//!     tool_name = "get_user",
//!     description = "Get a user by ID"
//! )]
//! async fn get_user(id: u64) -> Result<User, ApiError> {
//!     // Your implementation
//!     Ok(User { id, name: "Test".into() })
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let app = axiom::http::build();
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ## Feature System
//!
//! | Feature    | Description                          | Dependencies          |
//! |------------|--------------------------------------|-----------------------|
//! | `http`     | HTTP server (Axum 0.8.8)            | axum, tower, tower-http |
//! | `mcp`      | MCP protocol (mcp-sdk 0.0.3)        | mcp-sdk               |
//! | `streaming`| SSE streaming support                | tokio-stream, futures |
//! | `timestamp`| Auto-add timestamp to responses      | chrono                |
//! | `logging`  | Structured request logging           | tracing, tracing-subscriber |
//! | `full`     | All features enabled                 | -                     |
//!
//! ## Usage Examples
//!
//! ### HTTP Only
//!
//! ```toml
//! axiom = { version = "0.1", features = ["http"] }
//! ```
//!
//! ### MCP Only (for AI tools)
//!
//! ```toml
//! axiom = { version = "0.1", features = ["mcp"] }
//! ```
//!
//! ### Both Protocols
//!
//! ```toml
//! axiom = { version = "0.1", features = ["http", "mcp"] }
//! ```
//!
//! ### Full Features
//!
//! ```toml
//! axiom = { version = "0.1", features = ["full"] }
//! ```
//!
//! ## Module Prefixes
//!
//! Group related APIs with module prefixes:
//!
//! ```rust
//! #[service_module(prefix = "/auth")]
//! mod auth_api {
//!     use super::*;
//!
//!     #[service_api(
//!         name = "login",
//!         version = "v1",
//!         path = "/login",
//!         method = "POST"
//!     )]
//!     async fn login(credentials: Credentials) -> Result<Token, ApiError> {
//!         // Implementation
//!     }
//! }
//! ```
//!
//! This creates the endpoint: `/auth/api/v1/login`
//!
//! ## Error Handling
//!
//! Define your error types and convert them to `ServiceError`:
//!
//! ```rust
//! #[derive(Debug, thiserror::Error)]
//! pub enum MyError {
//!     #[error("Not found: {resource}")]
//!     NotFound { resource: String },
//! }
//!
//! impl From<MyError> for ServiceError {
//!     fn from(err: MyError) -> Self {
//!         match err {
//!             MyError::NotFound { resource } => ServiceError::with_details(
//!                 "NOT_FOUND",
//!                 format!("Resource not found: {}", resource),
//!                 serde_json::json!({ "resource": resource }),
//!                 404,
//!             ),
//!         }
//!     }
//! }
//! ```
//!
//! ## Path Parameters
//!
//! Extract path parameters using Rust naming conventions:
//!
//! ```rust
//! #[service_api(
//!     name = "get_user",
//!     version = "v1",
//!     path = "/users/:id",
//!     method = "GET"
//! )]
//! async fn get_user(id: u64) -> Result<User, ApiError> {
//!     // id is automatically extracted from the path
//! }
//!
//! #[service_api(
//!     name = "get_post_comments",
//!     version = "v1",
//!     path = "/posts/:post_id/comments/:comment_id",
//!     method = "GET"
//! )]
//! async fn get_comment(
//!     post_id: u64,
//!     comment_id: u64
//! ) -> Result<Comment, ApiError> {
//!     // Both parameters are extracted from the path
//! }
//! ```
//!
//! ## Version Management
//!
//! Support multiple API versions:
//!
//! ```rust
//! #[service_api(
//!     name = "get_user",
//!     version = "v1",
//!     path = "/users/:id",
//!     method = "GET",
//!     tool_name = "get_user_v1"
//! )]
//! async fn get_user_v1(id: u64) -> Result<UserV1, ApiError> {
//!     // V1 implementation
//! }
//!
//! #[service_api(
//!     name = "get_user",
//!     version = "v2",
//!     path = "/users/:id",
//!     method = "GET",
//!     tool_name = "get_user_v2"
//! )]
//! async fn get_user_v2(id: u64) -> Result<UserV2, ApiError> {
//!     // V2 implementation
//! }
//! ```
//!
//! ## Building
//!
//! ```bash
//! # Build with HTTP only
//! cargo build --features http
//!
//! # Build with MCP only
//! cargo build --features mcp
//!
//! # Build with all features
//! cargo build --features full
//!
//! # Run tests
//! cargo test --features http
//! cargo test --features mcp
//! cargo test --features "http,mcp"
//! ```
//!
//! ## Documentation
//!
//! - [API Documentation](https://docs.rs/axiom)
//! - [Examples](./examples/)
//! - [Tests](./tests/)
//!
//! ## License
//!
//! Licensed under either of:
//!
//! - Apache License, Version 2.0
//! - MIT License
//!
//! at your option.
//!
//! ## Contributing
//!
//! Contributions are welcome! Please read our contributing guidelines before submitting PRs.
