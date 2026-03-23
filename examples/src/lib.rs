//! SDForge Examples Library
//!
//! This library contains example modules demonstrating SDForge framework functionality.
//!
//! # Modules
//!
//! - `basic`: Core API definitions and error handling
//! - `http`: HTTP protocol examples
//! - `mcp`: MCP protocol examples
//! - `security`: Authentication and authorization
//! - `cache`: Caching examples
//! - `config`: Configuration management
//! - `streaming`: SSE streaming examples
//! - `websocket`: WebSocket examples
//! - `grpc`: gRPC examples
//! - `logging`: Logging examples
//! - `combined`: Full example applications

pub mod basic;
pub mod cache;
pub mod combined;
pub mod config;
pub mod grpc;
pub mod http;
pub mod logging;
pub mod mcp;
pub mod security;
pub mod streaming;
pub mod websocket;

pub use sdforge::prelude::*;
