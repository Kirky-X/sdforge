// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! WebSocket support for Axiom
//!
//! This module provides WebSocket protocol support alongside SSE.
//! It includes message types, connection management, and routing.
//!
//! # Features
//!
//! - Real-time bidirectional communication
//! - Automatic connection management
//! - Message serialization/deserialization
//! - Broadcast and point-to-point messaging
//!
//! # Example
//!
//! ```rust
//! use sdforge::websocket::{WebSocketMessage, ConnectionManager};
//!
//! let manager = ConnectionManager::new();
//! let message = WebSocketMessage::Request {
//!     id: "123".to_string(),
//!     method: "get_data".to_string(),
//!     params: serde_json::json!({"key": "value"}),
//! };
//! ```
//!
//! # Module Organization
//!
//! - `message`: [`WebSocketMessage`] enum, parsing, and depth/size limits
//! - `connection`: [`WebSocketConnection`], [`ConnectionManager`],
//!   [`WebSocketConfig`], `SdForgeState`
//! - `broadcast`: [`ConnectionManager::broadcast`] fan-out implementation
//! - `handler`: [`WebSocketHandler`] trait, `DefaultWebSocketHandler`,
//!   [`ValidatedWebSocketUpgrade`], [`websocket_upgrade`], `handle_socket`, [`build`]
//!
//! # Endpoint Lifecycle
//!
//! `#[forge(deprecated, sunset, successor)]` annotations travel on the
//! route's `ApiMetadata`, but the WebSocket protocol has no response-header
//! or metadata injection point (post-upgrade traffic is a bidirectional
//! message stream) — lifecycle declarations are an explicit no-op on ws
//! routes: the metadata is carried, nothing is stamped on responses. HTTP,
//! gRPC, MCP and OpenAPI consume the same declaration on their own channels.

mod broadcast;
mod connection;
mod handler;
mod message;

#[cfg(test)]
mod tests;

// Re-export public API. Order mirrors the original `mod.rs` declarations so
// downstream `use crate::websocket::*` continues to resolve every type.
pub use connection::{ConnectionManager, SdForgeState, WebSocketConfig, WebSocketConnection};
pub use handler::{
    BoxFuture, DefaultWebSocketHandler, ValidatedWebSocketUpgrade, WebSocketHandler,
    WebSocketRoute, build, websocket_upgrade,
};
// `MAX_STRING_LENGTH` 现已由 `parse_websocket_message` 强制执行，随正常 API 导出。
pub use message::{
    MAX_JSON_DEPTH, MAX_MESSAGE_SIZE, MAX_STRING_LENGTH, WebSocketMessage, calculate_value_depth,
    parse_websocket_message,
};
