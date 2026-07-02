// Copyright (c) 2026 Kirky.X
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
//! - `connection`: [`WebSocketConnection`], [`ConnectionManager`], `RateLimitConfig`,
//!   [`WebSocketConfig`], `AppState`
//! - `broadcast`: [`ConnectionManager::broadcast`] fan-out implementation
//! - `handler`: [`WebSocketHandler`] trait, `DefaultWebSocketHandler`,
//!   [`ValidatedWebSocketUpgrade`], [`websocket_upgrade`], `handle_socket`, [`build`]

#[cfg(feature = "websocket")]
mod broadcast;
#[cfg(feature = "websocket")]
mod connection;
#[cfg(feature = "websocket")]
mod handler;
#[cfg(feature = "websocket")]
mod message;

#[cfg(test)]
#[cfg(feature = "websocket")]
mod tests;

// Re-export public API. Order mirrors the original `mod.rs` declarations so
// downstream `use crate::websocket::*` continues to resolve every type.
#[cfg(feature = "websocket")]
pub use connection::{
    AppState, ConnectionManager, RateLimitConfig, WebSocketConfig, WebSocketConnection,
};
#[cfg(feature = "websocket")]
pub use handler::{
    build, websocket_upgrade, BoxFuture, DefaultWebSocketHandler, ValidatedWebSocketUpgrade,
    WebSocketHandler, WebSocketRoute,
};
#[cfg(feature = "websocket")]
pub use message::{
    calculate_value_depth, parse_websocket_message, WebSocketMessage, MAX_JSON_DEPTH,
    MAX_MESSAGE_SIZE,
};
// Test-only helpers from `message` module — re-exported under test cfg so the
// split test files can access them via `use crate::websocket::*`.
#[cfg(test)]
#[cfg(feature = "websocket")]
pub use message::{calculate_json_depth, MAX_STRING_LENGTH};
