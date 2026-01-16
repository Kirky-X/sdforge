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
//! use axiom::websocket::{WebSocketMessage, ConnectionManager};
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

#[cfg(feature = "websocket")]
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

    /// Check if connection is rate limited
    /// Returns true if rate limited (should disconnect)
    pub fn check_rate_limit(&self, conn_id: &str, config: &RateLimitConfig) -> bool {
        let now = Instant::now();
        let current_time = now.elapsed().as_secs();

        // Check total connection limit
        if self.connection_count.load(Ordering::Relaxed) >= config.max_connections {
            #[cfg(feature = "logging")]
            tracing::warn!(target: "websocket", "Max connections reached, rejecting new connection");
            return true;
        }

        // Check per-connection rate limit
        if let Some(count_ref) = self.message_counts.get(conn_id) {
            let last_time = self
                .last_message_time
                .get(conn_id)
                .map(|t| t.value().load(Ordering::Relaxed))
                .unwrap_or(0);

            // Reset counter if window has passed
            if current_time - last_time >= config.rate_limit_window_seconds {
                count_ref.value().store(0, Ordering::Relaxed);
                if let Some(time_entry) = self.last_message_time.get_mut(conn_id) {
                    time_entry.value().store(current_time, Ordering::Relaxed);
                }
                return false;
            }

            // Check if over rate limit
            if count_ref.value().load(Ordering::Relaxed) >= config.max_messages_per_second {
                #[cfg(feature = "logging")]
                tracing::warn!(target: "websocket",
                    conn_id = %conn_id,
                    msg_count = %count_ref.value().load(Ordering::Relaxed),
                    "Rate limit exceeded, disconnecting"
                );
                return true;
            }

            // Increment counter
            count_ref.value().fetch_add(1, Ordering::Relaxed);
        } else {
            // New connection, initialize counter
            self.message_counts
                .insert(conn_id.to_string(), AtomicU64::new(1));
            self.last_message_time
                .insert(conn_id.to_string(), AtomicU64::new(current_time));
        }

        false
    }

    /// Record a message for rate limiting
    pub fn record_message(&self, conn_id: &str, config: &RateLimitConfig) {
        let now = Instant::now();
        let current_time = now.elapsed().as_secs();

        if let Some(count_ref) = self.message_counts.get(conn_id) {
            let last_time = self
                .last_message_time
                .get(conn_id)
                .map(|t| t.value().load(Ordering::Relaxed))
                .unwrap_or(0);

            // Reset counter if window has passed
            if current_time - last_time >= config.rate_limit_window_seconds {
                count_ref.value().store(0, Ordering::Relaxed);
                if let Some(time_entry) = self.last_message_time.get_mut(conn_id) {
                    time_entry.value().store(current_time, Ordering::Relaxed);
                }
            } else {
                count_ref.value().fetch_add(1, Ordering::Relaxed);
            }
        }
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
    pub async fn broadcast(&self, message: &Arc<WebSocketMessage>) {
        for conn in self.connections.iter() {
            let _ = conn.send(message.as_ref().clone()).await;
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
const MAX_JSON_DEPTH: usize = 32;

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

    // Check for obviously malformed JSON that could cause excessive parsing
    let depth_estimate = text.bytes().filter(|&b| b == b'{' || b == b'[').count();
    if depth_estimate > MAX_JSON_DEPTH {
        return Err(format!(
            "JSON nesting too deep: estimated depth {} (max: {})",
            depth_estimate, MAX_JSON_DEPTH
        ));
    }

    // Use serde_json with custom limit to prevent DoS from deeply nested structures
    // The depth_estimate check above already limits nesting, so we just parse normally
    serde_json::from_str::<WebSocketMessage>(text).map_err(|e| format!("Invalid JSON: {}", e))
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

#[cfg(feature = "websocket")]
/// Build WebSocket router with custom connection manager
///
/// This function is similar to `build()` but allows providing a custom
/// connection manager for advanced use cases like connection sharing
/// across multiple routers or custom connection handling.
///
/// # Arguments
/// * `manager` - A shared connection manager for handling WebSocket connections
///
/// # Returns
/// A configured Axum Router using the provided connection manager
pub fn build_with_manager(manager: Arc<ConnectionManager>) -> Router {
    let mut router = Router::new();

    for route in inventory::iter::<WebSocketRoute> {
        router = router.route(
            &route.path,
            axum::routing::get(websocket_upgrade).with_state(manager.clone()),
        );
    }

    router
}
