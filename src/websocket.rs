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

#[cfg(feature = "websocket")]
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    Router,
};
#[cfg(feature = "websocket")]
use dashmap::DashMap;
#[cfg(feature = "websocket")]
use futures_util::SinkExt;
#[cfg(feature = "websocket")]
use futures_util::StreamExt;
#[cfg(feature = "websocket")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "websocket")]
use serde_json::Value;
#[cfg(feature = "websocket")]
use std::sync::Arc;

use crate::impl_default_new;

#[cfg(feature = "websocket")]
use std::time::Instant;

#[cfg(feature = "websocket")]
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[cfg(feature = "websocket")]
/// WebSocket message type
///
/// Represents different types of WebSocket messages exchanged between
/// client and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    /// Request message from client
    #[serde(rename = "request")]
    Request {
        /// Unique request identifier
        id: String,
        /// Method name to invoke
        method: String,
        /// Method parameters
        params: serde_json::Value,
    },
    /// Response message to client
    #[serde(rename = "response")]
    Response {
        /// Request identifier
        id: String,
        /// Result data
        result: serde_json::Value,
    },
    /// Error message to client
    #[serde(rename = "error")]
    Error {
        /// Request identifier
        id: String,
        /// Error message
        error: String,
    },
    /// Notification message to client
    #[serde(rename = "notification")]
    Notification {
        /// Event name
        event: String,
        /// Event data
        data: serde_json::Value,
    },
}

#[cfg(feature = "websocket")]
/// WebSocket handler trait
pub trait WebSocketHandler: Send + Sync {
    /// Handle a WebSocket message and return a response
    fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage>;
}

#[cfg(feature = "websocket")]
use std::pin::Pin;
#[cfg(feature = "websocket")]
/// Boxed future type for async WebSocket handling
pub type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

#[cfg(feature = "websocket")]
/// WebSocket connection
#[derive(Clone)]
pub struct WebSocketConnection {
    id: String,
    sender: tokio::sync::mpsc::UnboundedSender<WebSocketMessage>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    pub fn new(id: String) -> (Self, tokio::sync::mpsc::UnboundedReceiver<WebSocketMessage>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        (Self { id, sender }, receiver)
    }

    /// Get the connection ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Send a message to this connection
    pub async fn send(&self, message: WebSocketMessage) -> Result<(), Box<dyn std::error::Error>> {
        self.sender.send(message).map_err(|e| e.into())
    }
}

#[cfg(feature = "websocket")]
/// Connection manager for WebSocket connections with rate limiting
pub struct ConnectionManager {
    connections: Arc<DashMap<String, WebSocketConnection>>,
    /// Rate limiting: message count per connection per window
    message_counts: Arc<DashMap<String, AtomicU64>>,
    /// Rate limiting: connection count tracking
    connection_count: Arc<AtomicUsize>,
    /// Rate limiting: track messages per time window
    last_message_time: Arc<DashMap<String, AtomicU64>>,
}

/// Rate limiting configuration for WebSocket connections
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum messages per connection per second
    pub max_messages_per_second: u64,
    /// Maximum message size in bytes (1MB default)
    pub max_message_size: usize,
    /// Maximum connections allowed
    pub max_connections: usize,
    /// Time window in seconds for rate limiting
    pub rate_limit_window_seconds: u64,
}

impl RateLimitConfig {
    /// Validate the rate limit configuration
    /// Returns Err if configuration is invalid (could cause DoS or undefined behavior)
    pub fn validate(&self) -> Result<(), String> {
        if self.max_connections == 0 {
            return Err("max_connections must be greater than 0".to_string());
        }
        if self.max_connections > 100_000 {
            return Err("max_connections exceeds reasonable limit of 100,000".to_string());
        }
        if self.max_messages_per_second == 0 {
            return Err("max_messages_per_second must be greater than 0".to_string());
        }
        if self.max_messages_per_second > 1_000_000 {
            return Err(
                "max_messages_per_second exceeds reasonable limit of 1,000,000".to_string(),
            );
        }
        if self.max_message_size == 0 {
            return Err("max_message_size must be greater than 0".to_string());
        }
        if self.max_message_size > 100_000_000 {
            return Err("max_message_size exceeds reasonable limit of 100MB".to_string());
        }
        if self.rate_limit_window_seconds == 0 {
            return Err("rate_limit_window_seconds must be greater than 0".to_string());
        }
        if self.rate_limit_window_seconds > 86400 {
            return Err(
                "rate_limit_window_seconds exceeds reasonable limit of 86400 (24 hours)"
                    .to_string(),
            );
        }
        Ok(())
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_second: 100,
            max_message_size: 1_048_576, // 1MB
            max_connections: 1000,
            rate_limit_window_seconds: 1,
        }
    }
}

#[cfg(feature = "websocket")]
impl ConnectionManager {
    /// Create a new connection manager with rate limiting
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            message_counts: Arc::new(DashMap::new()),
            connection_count: Arc::new(AtomicUsize::new(0)),
            last_message_time: Arc::new(DashMap::new()),
        }
    }

    /// Check and record a message atomically for rate limiting
    /// Returns true if rate limited (should disconnect), false otherwise
    pub fn check_and_record(&self, conn_id: &str, config: &RateLimitConfig) -> bool {
        let now = Instant::now();
        let current_time = now.elapsed().as_secs();

        // 使用 compare_exchange 实现原子检查和设置，避免竞态窗口
        let mut current = self.connection_count.load(Ordering::SeqCst);
        loop {
            if current >= config.max_connections {
                #[cfg(feature = "logging")]
                tracing::warn!(target: "websocket", "Max connections reached, rejecting new connection");
                return true;
            }
            match self.connection_count.compare_exchange_weak(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(new_current) => current = new_current,
            }
        }

        // Check per-connection rate limit
        let mut should_disconnect = false;

        let entry = self.message_counts.entry(conn_id.to_string());

        match entry {
            dashmap::mapref::entry::Entry::Occupied(count_entry) => {
                let count = count_entry.get();
                let last_time = self
                    .last_message_time
                    .get(conn_id)
                    .map(|t| t.value().load(Ordering::Relaxed))
                    .unwrap_or(0);

                // Reset counter if window has passed
                if current_time - last_time >= config.rate_limit_window_seconds {
                    count.store(0, Ordering::Relaxed);
                    if let Some(time_entry) = self.last_message_time.get_mut(conn_id) {
                        time_entry.value().store(current_time, Ordering::Relaxed);
                    }
                } else if count.load(Ordering::Relaxed) >= config.max_messages_per_second {
                    should_disconnect = true;
                } else {
                    count.fetch_add(1, Ordering::Relaxed);
                }
            }
            dashmap::mapref::entry::Entry::Vacant(_) => {
                drop(entry);
                self.message_counts
                    .insert(conn_id.to_string(), AtomicU64::new(1));
                self.last_message_time
                    .insert(conn_id.to_string(), AtomicU64::new(current_time));
            }
        }

        if should_disconnect {
            self.connection_count.fetch_sub(1, Ordering::SeqCst);
            #[cfg(feature = "logging")]
            tracing::warn!(target: "websocket",
                conn_id = %conn_id,
                "Rate limit exceeded, disconnecting"
            );
        }

        should_disconnect
    }

    /// Add a connection to the manager
    pub async fn add_connection(&self, id: String, conn: WebSocketConnection) {
        self.connections.insert(id.clone(), conn);
        self.connection_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove a connection from the manager
    pub async fn remove_connection(&self, id: &str) {
        self.connections.remove(id);
        self.connection_count.fetch_sub(1, Ordering::Relaxed);
        // Clean up rate limiting data
        self.message_counts.remove(id);
        self.last_message_time.remove(id);
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: &str) -> Option<WebSocketConnection> {
        self.connections.get(id).map(|conn| conn.clone())
    }

    /// Broadcast a message to all connections (optimized with Arc)
    /// Security fix: Handle broadcast errors properly and clean up failed connections
    pub async fn broadcast(&self, message: &Arc<WebSocketMessage>) {
        let mut failed_connections: Vec<String> = Vec::new();

        for conn in self.connections.iter() {
            if let Err(_e) = conn.send(message.as_ref().clone()).await {
                // Track failed connections for cleanup
                // Don't log every failure to avoid log spam
                failed_connections.push(conn.id().to_string());
            }
        }

        // Clean up failed connections
        for conn_id in failed_connections {
            self.remove_connection(&conn_id).await;
        }
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connection_count.load(Ordering::Relaxed)
    }
}

#[cfg(feature = "websocket")]
impl_default_new!(ConnectionManager);

#[cfg(feature = "websocket")]
/// Default implementation of WebSocketHandler
pub struct DefaultWebSocketHandler;

#[cfg(feature = "websocket")]
impl WebSocketHandler for DefaultWebSocketHandler {
    fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
        Box::pin(async move {
            match message {
                WebSocketMessage::Request { id, method, .. } => WebSocketMessage::Response {
                    id,
                    result: serde_json::json!({"status": "ok", "method": method}),
                },
                _ => message,
            }
        })
    }
}

#[cfg(feature = "websocket")]
/// WebSocket route registration
pub struct WebSocketRoute {
    /// The WebSocket route path
    pub path: String,
    /// The WebSocket handler for this route
    pub handler: Arc<dyn WebSocketHandler>,
}

#[cfg(feature = "websocket")]
inventory::collect!(WebSocketRoute);

#[cfg(feature = "websocket")]
/// WebSocket upgrade handler
pub async fn websocket_upgrade(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<ConnectionManager>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, manager))
}

#[cfg(feature = "websocket")]
/// Maximum message size in bytes (1MB)
const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// Maximum nesting depth for JSON parsing (prevents stack overflow from deeply nested JSON)
const MAX_JSON_DEPTH: usize = 16;

/// Maximum length for string fields in WebSocket messages
#[allow(dead_code)]
const MAX_STRING_LENGTH: usize = 64 * 1024; // 64KB

#[cfg(feature = "websocket")]
fn parse_websocket_message(text: &str) -> Result<WebSocketMessage, String> {
    // First, check basic size limit
    if text.len() > MAX_MESSAGE_SIZE {
        return Err(format!(
            "Message too large: {} bytes (max: {} bytes)",
            text.len(),
            MAX_MESSAGE_SIZE
        ));
    }

    // Security fix: Use serde_json's streaming parser to parse and validate depth
    // This provides more accurate depth checking than manual bracket counting
    use serde_json::{Deserializer, Value};

    // Parse and validate depth
    let mut max_depth = 0;
    let mut current_depth = 0;

    let deserializer = Deserializer::from_str(text);
    for result in deserializer.into_iter::<Value>() {
        match result {
            Ok(value) => {
                // Calculate actual depth of the parsed value
                let depth = calculate_value_depth(&value, &mut current_depth);
                max_depth = max_depth.max(depth);

                if max_depth > MAX_JSON_DEPTH {
                    return Err(format!(
                        "JSON nesting too deep: depth {} (max: {})",
                        max_depth, MAX_JSON_DEPTH
                    ));
                }
            }
            Err(e) => {
                return Err(format!("Invalid JSON: {}", e));
            }
        }
    }

    // Parse the actual WebSocket message
    serde_json::from_str::<WebSocketMessage>(text).map_err(|e| format!("Invalid JSON: {}", e))
}

/// Calculate depth of a JSON value recursively
fn calculate_value_depth(value: &serde_json::Value, current_depth: &mut usize) -> usize {
    match value {
        Value::Object(map) => {
            *current_depth += 1;
            let max_child_depth = map
                .values()
                .map(|v| calculate_value_depth(v, current_depth))
                .max()
                .unwrap_or(0);
            *current_depth -= 1;
            max_child_depth
        }
        Value::Array(arr) => {
            *current_depth += 1;
            let max_child_depth = arr
                .iter()
                .map(|v| calculate_value_depth(v, current_depth))
                .max()
                .unwrap_or(0);
            *current_depth -= 1;
            max_child_depth
        }
        _ => *current_depth,
    }
}

/// Calculate actual JSON nesting depth by parsing the structure
/// Returns the maximum nesting level encountered
#[allow(dead_code)]
fn calculate_json_depth(text: &str) -> usize {
    let mut depth = 0;
    let mut max_depth = 0;
    let mut in_string = false;
    let mut escaped = false;

    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
        } else if c == '"' {
            in_string = true;
            escaped = false;
        } else if c == '{' || c == '[' {
            depth += 1;
            max_depth = max_depth.max(depth);
        } else if (c == '}' || c == ']') && depth > 0 {
            depth -= 1;
        }
    }

    max_depth
}

#[cfg(feature = "websocket")]
async fn handle_socket(mut socket: WebSocket, manager: Arc<ConnectionManager>) {
    let conn_id = uuid::Uuid::new_v4().to_string();
    let (conn, _receiver) = WebSocketConnection::new(conn_id.clone());
    manager.add_connection(conn_id.clone(), conn).await;

    // Handle incoming messages
    while let Some(result) = socket.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_text() {
                    // Check message size early
                    if text.len() > MAX_MESSAGE_SIZE {
                        #[cfg(feature = "logging")]
                        tracing::warn!(target: "websocket",
                            conn_id = %conn_id,
                            msg_size = %text.len(),
                            max_size = %MAX_MESSAGE_SIZE,
                            "Message size exceeded limit, closing connection"
                        );
                        // Close connection immediately to prevent DoS
                        let _ = socket.close().await;
                        return;
                    }

                    match parse_websocket_message(text) {
                        Ok(ws_msg) => {
                            // Handle message with default handler
                            let handler = DefaultWebSocketHandler;
                            let response = handler.handle(ws_msg).await;
                            // Use map_err to convert serialization errors to error messages
                            let response_json = match serde_json::to_string(&response) {
                                Ok(json) => json,
                                Err(e) => {
                                    #[cfg(feature = "logging")]
                                    tracing::error!(target: "websocket",
                                        conn_id = %conn_id,
                                        error = %e,
                                        "Failed to serialize response"
                                    );
                                    // Send a generic error to the client
                                    let error_response = WebSocketMessage::Error {
                                        id: String::new(),
                                        error: "Internal serialization error".to_string(),
                                    };
                                    if let Ok(json) = serde_json::to_string(&error_response) {
                                        json
                                    } else {
                                        // If even the error message can't be serialized, send a hardcoded fallback
                                        r#"{"type":"error","id":"","error":"Internal error"}"#
                                            .to_string()
                                    }
                                }
                            };
                            let _ = socket
                                .send(axum::extract::ws::Message::Text(response_json.into()))
                                .await;
                        }
                        Err(e) => {
                            let error_msg = WebSocketMessage::Error {
                                id: String::new(),
                                error: e,
                            };
                            // Use match instead of expect to handle serialization errors gracefully
                            let response_json = match serde_json::to_string(&error_msg) {
                                Ok(json) => json,
                                Err(e) => {
                                    #[cfg(feature = "logging")]
                                    tracing::error!(target: "websocket",
                                        conn_id = %conn_id,
                                        error = %e,
                                        "Failed to serialize error message"
                                    );
                                    // Send a hardcoded fallback error message
                                    r#"{"type":"error","id":"","error":"Internal error processing request"}"#.to_string()
                                }
                            };
                            let _ = socket
                                .send(axum::extract::ws::Message::Text(response_json.into()))
                                .await;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket error: {:?}", e);
                break;
            }
        }
    }

    // Cleanup
    manager.remove_connection(&conn_id).await;
}

#[cfg(feature = "websocket")]
/// Build WebSocket router with default connection manager
///
/// This function collects all WebSocket routes registered via `inventory::submit!`
/// and builds an Axum router for handling WebSocket connections.
///
/// Routes are automatically registered with the WebSocket upgrade handler
/// and connection management state.
///
/// # Returns
/// A configured Axum Router ready to handle WebSocket connections
pub fn build() -> Router {
    let mut router = Router::new();
    let manager = Arc::new(ConnectionManager::new());

    for route in inventory::iter::<WebSocketRoute> {
        router = router.route(
            &route.path,
            axum::routing::get(websocket_upgrade).with_state(manager.clone()),
        );
    }

    router
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;

    /// Test WebSocketMessage serialization and deserialization
    #[test]
    fn test_websocket_message_request() {
        let msg = WebSocketMessage::Request {
            id: "test-123".to_string(),
            method: "get_data".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        // Test serialization
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"request\""));
        assert!(json.contains("\"id\":\"test-123\""));
        assert!(json.contains("\"method\":\"get_data\""));

        // Test deserialization
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            WebSocketMessage::Request { id, method, params } => {
                assert_eq!(id, "test-123");
                assert_eq!(method, "get_data");
                assert_eq!(params["key"], "value");
            }
            _ => panic!("Expected Request variant"),
        }
    }

    /// Test WebSocketMessage Response variant
    #[test]
    fn test_websocket_message_response() {
        let msg = WebSocketMessage::Response {
            id: "resp-456".to_string(),
            result: serde_json::json!({"status": "ok"}),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"response\""));

        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            WebSocketMessage::Response { id, result } => {
                assert_eq!(id, "resp-456");
                assert_eq!(result["status"], "ok");
            }
            _ => panic!("Expected Response variant"),
        }
    }

    /// Test WebSocketMessage Error variant
    #[test]
    fn test_websocket_message_error() {
        let msg = WebSocketMessage::Error {
            id: "err-789".to_string(),
            error: "Something went wrong".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"error\""));

        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            WebSocketMessage::Error { id, error } => {
                assert_eq!(id, "err-789");
                assert_eq!(error, "Something went wrong");
            }
            _ => panic!("Expected Error variant"),
        }
    }

    /// Test WebSocketMessage Notification variant
    #[test]
    fn test_websocket_message_notification() {
        let msg = WebSocketMessage::Notification {
            event: "user_joined".to_string(),
            data: serde_json::json!({"user": "alice"}),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"notification\""));

        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            WebSocketMessage::Notification { event, data } => {
                assert_eq!(event, "user_joined");
                assert_eq!(data["user"], "alice");
            }
            _ => panic!("Expected Notification variant"),
        }
    }

    /// Test WebSocketConnection creation
    #[test]
    fn test_websocket_connection_new() {
        let (conn, mut receiver) = WebSocketConnection::new("conn-001".to_string());
        assert_eq!(conn.id(), "conn-001");
        assert!(!conn.id().is_empty());
        // Receiver should be ready to receive
        assert!(receiver.recv().now_or_never().is_none());
    }

    /// Test RateLimitConfig default values
    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_messages_per_second, 100);
        assert_eq!(config.max_message_size, 1_048_576);
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.rate_limit_window_seconds, 1);
    }

    /// Test RateLimitConfig validation - valid config
    #[test]
    fn test_rate_limit_config_valid() {
        let config = RateLimitConfig {
            max_messages_per_second: 50,
            max_message_size: 1024,
            max_connections: 100,
            rate_limit_window_seconds: 60,
        };
        assert!(config.validate().is_ok());
    }

    /// Test RateLimitConfig validation - invalid max_connections
    #[test]
    fn test_rate_limit_config_invalid_connections() {
        let config = RateLimitConfig {
            max_connections: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("max_connections"));
    }

    /// Test RateLimitConfig validation - exceeds max connections
    #[test]
    fn test_rate_limit_config_exceeds_connections() {
        let config = RateLimitConfig {
            max_connections: 100_001,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("100,000"));
    }

    /// Test RateLimitConfig validation - invalid messages per second
    #[test]
    fn test_rate_limit_config_invalid_messages() {
        let config = RateLimitConfig {
            max_messages_per_second: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("max_messages_per_second"));
    }

    /// Test RateLimitConfig validation - exceeds max messages
    #[test]
    fn test_rate_limit_config_exceeds_messages() {
        let config = RateLimitConfig {
            max_messages_per_second: 1_000_001,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("1,000,000"));
    }

    /// Test RateLimitConfig validation - invalid message size
    #[test]
    fn test_rate_limit_config_invalid_size() {
        let config = RateLimitConfig {
            max_message_size: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("max_message_size"));
    }

    /// Test RateLimitConfig validation - exceeds max size
    #[test]
    fn test_rate_limit_config_exceeds_size() {
        let config = RateLimitConfig {
            max_message_size: 100_000_001,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("100MB"));
    }

    /// Test RateLimitConfig validation - invalid window
    #[test]
    fn test_rate_limit_config_invalid_window() {
        let config = RateLimitConfig {
            rate_limit_window_seconds: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config
            .validate()
            .unwrap_err()
            .contains("rate_limit_window_seconds"));
    }

    /// Test RateLimitConfig validation - exceeds max window
    #[test]
    fn test_rate_limit_config_exceeds_window() {
        let config = RateLimitConfig {
            rate_limit_window_seconds: 86401,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("24 hours"));
    }

    /// Test ConnectionManager creation
    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        // Just verify it can be created without panic
        let _ = manager;
    }

    /// Test calculate_json_depth function
    #[test]
    fn test_calculate_json_depth_empty() {
        assert_eq!(calculate_json_depth(""), 0);
    }

    #[test]
    fn test_calculate_json_depth_simple() {
        assert_eq!(calculate_json_depth("{}"), 1);
        assert_eq!(calculate_json_depth("[]"), 1);
    }

    #[test]
    fn test_calculate_json_depth_nested() {
        // The function counts maximum nesting depth of braces/brackets
        // {"a":{"b":{"c":1}}} returns 3 as the max depth
        assert_eq!(calculate_json_depth(r#"{"a":{"b":{"c":1}}}"#), 3);
        // [{"a":[{"b":1}]}] starts with [ so returns 4
        assert_eq!(calculate_json_depth(r#"[{"a":[{"b":1}]}]"#), 4);
    }

    #[test]
    fn test_calculate_json_depth_with_strings() {
        // Strings should not count toward depth
        assert_eq!(calculate_json_depth(r#"{"a":"{"}"}"#), 1);
    }

    #[test]
    fn test_calculate_json_depth_array_nesting() {
        assert_eq!(calculate_json_depth("[[[[1]]]]"), 4);
    }

    /// Test parse_websocket_message with valid JSON
    #[test]
    fn test_parse_websocket_message_valid() {
        let valid_json = r#"{"type":"request","id":"123","method":"test","params":{}}"#;
        let result = parse_websocket_message(valid_json);
        assert!(result.is_ok());
        match result.unwrap() {
            WebSocketMessage::Request { id, method, .. } => {
                assert_eq!(id, "123");
                assert_eq!(method, "test");
            }
            _ => panic!("Expected Request"),
        }
    }

    /// Test parse_websocket_message with invalid JSON
    #[test]
    fn test_parse_websocket_message_invalid() {
        let invalid_json = "not valid json";
        let result = parse_websocket_message(invalid_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid JSON"));
    }

    /// Test parse_websocket_message with too large message
    #[test]
    fn test_parse_websocket_message_too_large() {
        let large_json = "x".repeat(MAX_MESSAGE_SIZE + 1);
        let result = parse_websocket_message(&large_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Message too large"));
    }

    /// Test parse_websocket_message with deeply nested JSON
    #[test]
    fn test_parse_websocket_message_too_deep() {
        // Create a valid deeply nested JSON structure
        let mut deep_json = String::from("0");
        for _ in 0..=MAX_JSON_DEPTH {
            deep_json = format!(r#"{{"a":{}}}"#, deep_json);
        }

        let result = parse_websocket_message(&deep_json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nesting too deep"));
    }

    /// Test DefaultWebSocketHandler
    #[test]
    fn test_default_websocket_handler() {
        let handler = DefaultWebSocketHandler;

        // Test Request handling
        let request = WebSocketMessage::Request {
            id: "test-id".to_string(),
            method: "test_method".to_string(),
            params: serde_json::json!({"test": true}),
        };

        // Handler is async, but we can verify it compiles
        // Full async test would require runtime
        let result = handler.handle(request).now_or_never().unwrap();
        match result {
            WebSocketMessage::Response { id, .. } => assert_eq!(id, "test-id"),
            _ => panic!("Expected Response variant"),
        }
    }

    /// Test WebSocketRoute structure
    #[test]
    fn test_websocket_route_structure() {
        use std::sync::Arc;

        struct MockHandler;
        impl WebSocketHandler for MockHandler {
            fn handle(&self, _message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
                Box::pin(async {
                    WebSocketMessage::Response {
                        id: String::new(),
                        result: serde_json::json!({}),
                    }
                })
            }
        }

        let route = WebSocketRoute {
            path: "/ws".to_string(),
            handler: Arc::new(MockHandler) as Arc<dyn WebSocketHandler>,
        };

        assert_eq!(route.path, "/ws");
    }
}
