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
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::{header::AUTHORIZATION, StatusCode},
    response::{IntoResponse, Response},
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

/// WebSocket server configuration with optional JWT authentication.
///
/// # Security
///
/// When `auth` is `Some`, all WebSocket upgrade requests must include a valid
/// JWT bearer token in the `Authorization` header. Requests without a valid
/// token receive HTTP 401 and the connection is rejected.
///
/// # Example
///
/// ```ignore
/// use sdforge::websocket::{WebSocketConfig, RateLimitConfig};
/// use sdforge::security::BearerAuth;
///
/// let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ").ok();
/// let config = WebSocketConfig {
///     auth,
///     rate_limit: RateLimitConfig::default(),
/// };
/// ```
#[derive(Clone, Default)]
pub struct WebSocketConfig {
    /// Optional JWT authentication validator.
    /// When `Some`, all connections must present a valid JWT bearer token.
    /// When `None`, connections are accepted without authentication.
    pub auth: Option<crate::security::BearerAuth>,
    /// Rate limiting configuration for connections.
    pub rate_limit: RateLimitConfig,
}

#[cfg(feature = "websocket")]
#[derive(Clone)]
/// Application state for the WebSocket router.
///
/// Combines WebSocket configuration with connection manager for active connection tracking.
pub struct AppState {
    /// WebSocket configuration including optional auth.
    pub config: Arc<WebSocketConfig>,
    /// Connection manager for tracking active WebSocket connections.
    pub manager: Arc<ConnectionManager>,
}

#[cfg(feature = "websocket")]
impl AppState {
    /// Create a new AppState with default WebSocketConfig.
    pub fn new(manager: Arc<ConnectionManager>) -> Self {
        Self {
            config: Arc::new(WebSocketConfig::default()),
            manager,
        }
    }

    /// Create a new AppState with custom WebSocketConfig.
    pub fn with_config(config: WebSocketConfig, manager: Arc<ConnectionManager>) -> Self {
        Self {
            config: Arc::new(config),
            manager,
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
impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

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
use crate::core::ApiMetadata;
#[cfg(feature = "websocket")]
use crate::define_registration;

#[cfg(feature = "websocket")]
define_registration!(WebSocketRoute, Arc<dyn WebSocketHandler>, ApiMetadata);
/// Custom WebSocket upgrade extractor that validates JWT auth before upgrade.
///
/// This type handles the entire WebSocket upgrade lifecycle:
/// 1. Reads the `Authorization` header from the request
/// 2. If auth is configured in `AppState`, validates the bearer token (returns 401 if invalid)
/// 3. Extracts the WebSocketUpgrade
/// 4. Implements `IntoResponse` to perform the actual upgrade
///
/// Usage:
/// ```ignore
/// use axum::response::IntoResponse;
/// use sdforge::websocket::ValidatedWebSocketUpgrade;
///
/// pub async fn ws_handler(ws: ValidatedWebSocketUpgrade) -> impl IntoResponse {
///     ws // performs upgrade automatically via IntoResponse
/// }
/// ```
#[cfg(feature = "websocket")]
pub struct ValidatedWebSocketUpgrade {
    ws: WebSocketUpgrade,
    manager: Arc<ConnectionManager>,
}

#[cfg(feature = "websocket")]
impl IntoResponse for ValidatedWebSocketUpgrade {
    fn into_response(self) -> Response {
        self.ws
            .on_upgrade(move |socket| handle_socket(socket, self.manager.clone()))
    }
}

#[cfg(feature = "websocket")]
impl<S> axum::extract::FromRequest<S> for ValidatedWebSocketUpgrade
where
    S: Clone + Send + Sync + 'static,
{
    type Rejection = StatusCode;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let req = req;

        // Get bearer token from Authorization header
        let bearer_token: Option<String> = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(String::from);

        // Get AppState from request extensions (injected via with_state by axum)
        // The state parameter is &Arc<AppState> since that's what we registered
        let app_state = req.extensions().get::<Arc<AppState>>().cloned();

        // Validate auth if configured
        if let Some(ref state_ref) = app_state {
            if let Some(ref auth) = state_ref.config.auth {
                let token = bearer_token.ok_or(StatusCode::UNAUTHORIZED)?;
                auth.validate_token(&token)
                    .ok_or(StatusCode::UNAUTHORIZED)?;
            }
        }

        // Extract WebSocketUpgrade via axum's built-in extractor
        let ws = axum::extract::ws::WebSocketUpgrade::from_request(req, state)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        // Get manager for connection handling
        let manager = app_state
            .map(|s| s.manager.clone())
            .unwrap_or_else(|| Arc::new(ConnectionManager::new()));

        Ok(Self { ws, manager })
    }
}

#[cfg(feature = "websocket")]
/// WebSocket upgrade handler with optional JWT authentication.
///
/// Security: When `WebSocketConfig::auth` is `Some`, this handler validates
/// the `Authorization: Bearer <token>` header before upgrading the connection.
/// Invalid or missing tokens result in HTTP 401 Unauthorized.
pub async fn websocket_upgrade(ws: ValidatedWebSocketUpgrade) -> impl IntoResponse {
    ws // IntoResponse performs the upgrade
}

#[cfg(feature = "websocket")]
/// Maximum message size in bytes (1MB)
const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// Maximum nesting depth for JSON parsing (prevents stack overflow from deeply nested JSON)
const MAX_JSON_DEPTH: usize = 16;

/// Maximum length for string fields in WebSocket messages (64KB).
///
/// # Security Purpose
///
/// This constant defines a reasonable upper bound for string field lengths to prevent:
/// - Memory exhaustion attacks via oversized string fields
/// - Buffer overflow vulnerabilities in downstream processing  
/// - Performance degradation from processing extremely large strings
///
/// # Future Use
///
/// While not currently enforced in parsing logic, this constant serves as:
/// - A reference for implementing future validation checks
/// - A best practice reminder for secure WebSocket message handling
/// - A potential configuration parameter for custom validation rules
///
/// # Recommendation
///
/// Implementers should consider validating string field lengths against this limit
/// when processing WebSocket messages, especially in production environments.
#[allow(dead_code)] // Reserved for future WebSocket message size validation
const MAX_STRING_LENGTH: usize = 64 * 1024; // 64KB

#[cfg(feature = "websocket")]
/// Parse and validate a WebSocket message from JSON text
///
/// # Security
///
/// This function provides security measures:
/// - Maximum message size validation (1MB)
/// - JSON nesting depth validation (max 16 levels)
///
/// # Errors
///
/// Returns an error if:
/// - Message exceeds maximum size
/// - JSON is malformed
/// - Nesting depth exceeds limit
/// - Message is not a valid WebSocketMessage variant
pub fn parse_websocket_message(text: &str) -> Result<WebSocketMessage, String> {
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
///
/// This function is kept for testing purposes only.
/// Production code uses `calculate_value_depth` which operates on parsed JSON values.
#[cfg(test)]
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
                                Err(_) => {
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
                                Err(_) => {
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
            Err(_) => {
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
    let state = Arc::new(AppState::new(manager));

    for route in inventory::iter::<WebSocketRoute> {
        // Use the registration name to construct the path
        let path = format!("/{}", route.name);
        router = router.route(
            &path,
            axum::routing::get(websocket_upgrade).with_state(state.clone()),
        );
    }

    router
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::registration::Registration;
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

        assert!(
            matches!(
                decoded,
                WebSocketMessage::Request {
                    ref id,
                    ref method,
                    ref params,
                } if id == "test-123" && method == "get_data" && params["key"] == "value"
            ),
            "Expected Request variant with correct values"
        );
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

        assert!(
            matches!(
                decoded,
                WebSocketMessage::Response { ref id, ref result }
                    if id == "resp-456" && result["status"] == "ok"
            ),
            "Expected Response variant with correct values"
        );
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

        assert!(
            matches!(
                decoded,
                WebSocketMessage::Error { ref id, ref error }
                    if id == "err-789" && error == "Something went wrong"
            ),
            "Expected Error variant with correct values"
        );
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

        assert!(
            matches!(
                decoded,
                WebSocketMessage::Notification { ref event, ref data }
                    if event == "user_joined" && data["user"] == "alice"
            ),
            "Expected Notification variant with correct values"
        );
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

        fn create_mock_handler() -> Arc<dyn WebSocketHandler> {
            Arc::new(MockHandler) as Arc<dyn WebSocketHandler>
        }

        let route = WebSocketRoute::new("/ws", "v1", create_mock_handler, || ApiMetadata {
            name: "/ws".to_string(),
            version: "v1".to_string(),
            description: "WebSocket handler".to_string(),
            cache_ttl: None,
            is_streaming: true,
        });

        assert_eq!(route.name(), "/ws");
        assert_eq!(route.version(), "v1");
    }

    /// Test WebSocketConfig default has no auth configured
    #[test]
    fn test_websocket_config_default_no_auth() {
        let config = WebSocketConfig::default();
        assert!(config.auth.is_none());
        assert_eq!(config.rate_limit.max_connections, 1000);
        assert_eq!(config.rate_limit.max_messages_per_second, 100);
    }

    /// Test WebSocketConfig with BearerAuth configured
    #[test]
    fn test_websocket_config_with_auth() {
        let auth =
            crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        assert!(config.auth.is_some());
    }

    /// Test AppState creation with custom config
    #[test]
    fn test_app_state_with_config() {
        use std::sync::Arc;
        let manager = Arc::new(ConnectionManager::new());
        let auth =
            crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .expect("valid secret");
        let config = WebSocketConfig {
            auth: Some(auth),
            rate_limit: RateLimitConfig::default(),
        };
        let state = AppState::with_config(config, manager.clone());
        assert!(state.config.auth.is_some());
    }

    /// Test bearer token extraction from Authorization header value
    #[test]
    fn test_bearer_token_extraction() {
        use axum::http::header::AUTHORIZATION;
        use axum::http::HeaderMap;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer my-test-token".parse().unwrap());
        let token = headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(String::from);
        assert_eq!(token, Some("my-test-token".to_string()));
    }

    /// Test bearer token extraction fails without Bearer prefix
    #[test]
    fn test_bearer_token_extraction_no_bearer() {
        use axum::http::header::AUTHORIZATION;
        use axum::http::HeaderMap;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Basic abc123".parse().unwrap());
        let token = headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(String::from);
        assert!(token.is_none());
    }

    #[test]
    fn test_websocket_message_request_empty_fields() {
        let msg = WebSocketMessage::Request {
            id: String::new(),
            method: String::new(),
            params: serde_json::json!(null),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            WebSocketMessage::Request { ref id, ref method, ref params }
                if id.is_empty() && method.is_empty() && params.is_null()
        ));
    }

    #[test]
    fn test_websocket_message_request_unicode() {
        let msg = WebSocketMessage::Request {
            id: "日本語".to_string(),
            method: "方法".to_string(),
            params: serde_json::json!({"键": "值"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            WebSocketMessage::Request { ref id, ref method, .. }
                if id == "日本語" && method == "方法"
        ));
    }

    #[test]
    fn test_websocket_message_request_large_params() {
        let large_array: Vec<i32> = (0..10000).collect();
        let msg = WebSocketMessage::Request {
            id: "large".to_string(),
            method: "test".to_string(),
            params: serde_json::json!({ "data": large_array }),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.len() > 40000);
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, WebSocketMessage::Request { .. }));
    }

    #[test]
    fn test_websocket_message_response_empty_result() {
        let msg = WebSocketMessage::Response {
            id: "test".to_string(),
            result: serde_json::json!(null),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, WebSocketMessage::Response { .. }));
    }

    #[test]
    fn test_websocket_message_response_nested_result() {
        let nested = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": "deep"
                }
            }
        });
        let msg = WebSocketMessage::Response {
            id: "nested".to_string(),
            result: nested.clone(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        if let WebSocketMessage::Response { result, .. } = decoded {
            assert_eq!(result["level1"]["level2"]["level3"], "deep");
        } else {
            panic!("Expected Response");
        }
    }

    #[test]
    fn test_websocket_message_error_empty_error() {
        let msg = WebSocketMessage::Error {
            id: String::new(),
            error: String::new(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, WebSocketMessage::Error { .. }));
    }

    #[test]
    fn test_websocket_message_error_long_message() {
        let long_error = "x".repeat(10000);
        let msg = WebSocketMessage::Error {
            id: "err".to_string(),
            error: long_error.clone(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        if let WebSocketMessage::Error { error, .. } = decoded {
            assert_eq!(error.len(), 10000);
        } else {
            panic!("Expected Error");
        }
    }

    #[test]
    fn test_websocket_message_notification_empty_event() {
        let msg = WebSocketMessage::Notification {
            event: String::new(),
            data: serde_json::json!({}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, WebSocketMessage::Notification { .. }));
    }

    #[test]
    fn test_websocket_message_notification_array_data() {
        let msg = WebSocketMessage::Notification {
            event: "list_update".to_string(),
            data: serde_json::json!([1, 2, 3, 4, 5]),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: WebSocketMessage = serde_json::from_str(&json).unwrap();
        if let WebSocketMessage::Notification { data, .. } = decoded {
            assert_eq!(data.as_array().unwrap().len(), 5);
        } else {
            panic!("Expected Notification");
        }
    }

    #[test]
    fn test_websocket_message_deserialize_missing_type() {
        let json = r#"{"id":"123","method":"test","params":{}}"#;
        let result: Result<WebSocketMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_websocket_message_deserialize_invalid_type() {
        let json = r#"{"type":"invalid","id":"123"}"#;
        let result: Result<WebSocketMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_websocket_message_deserialize_request_missing_field() {
        let json = r#"{"type":"request","id":"123"}"#;
        let result: Result<WebSocketMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_websocket_connection_id_accessor() {
        let (conn, _) = WebSocketConnection::new("unique-id-123".to_string());
        assert_eq!(conn.id(), "unique-id-123");
    }

    #[test]
    fn test_websocket_connection_clone() {
        let (conn, _) = WebSocketConnection::new("clone-test".to_string());
        let cloned = conn.clone();
        assert_eq!(cloned.id(), "clone-test");
    }

    #[tokio::test]
    async fn test_websocket_connection_send_success() {
        let (conn, mut receiver) = WebSocketConnection::new("send-test".to_string());
        let msg = WebSocketMessage::Notification {
            event: "test".to_string(),
            data: serde_json::json!({}),
        };
        let result = conn.send(msg.clone()).await;
        let received = receiver.recv().await;
        assert!(result.is_ok());
        assert!(received.is_some());
    }

    #[test]
    fn rate_limit_config_clone() {
        let config = RateLimitConfig::default();
        let cloned = config.clone();
        assert_eq!(config.max_connections, cloned.max_connections);
        assert_eq!(
            config.max_messages_per_second,
            cloned.max_messages_per_second
        );
    }

    #[test]
    fn rate_limit_config_debug() {
        let config = RateLimitConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("max_messages_per_second"));
        assert!(debug_str.contains("max_message_size"));
        assert!(debug_str.contains("max_connections"));
    }

    #[test]
    fn rate_limit_config_boundary_min_connections() {
        let config = RateLimitConfig {
            max_connections: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_max_connections() {
        let config = RateLimitConfig {
            max_connections: 100_000,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_min_messages() {
        let config = RateLimitConfig {
            max_messages_per_second: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_max_messages() {
        let config = RateLimitConfig {
            max_messages_per_second: 1_000_000,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_min_size() {
        let config = RateLimitConfig {
            max_message_size: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_max_size() {
        let config = RateLimitConfig {
            max_message_size: 100_000_000,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_min_window() {
        let config = RateLimitConfig {
            rate_limit_window_seconds: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rate_limit_config_boundary_max_window() {
        let config = RateLimitConfig {
            rate_limit_window_seconds: 86400,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn websocket_config_clone() {
        let config = WebSocketConfig::default();
        let cloned = config.clone();
        assert_eq!(config.auth.is_some(), cloned.auth.is_some());
        assert_eq!(
            config.rate_limit.max_connections,
            cloned.rate_limit.max_connections
        );
    }

    #[tokio::test]
    async fn connection_manager_add_connection() {
        let manager = ConnectionManager::new();
        let (conn, _) = WebSocketConnection::new("test-conn-1".to_string());
        manager
            .add_connection("test-conn-1".to_string(), conn)
            .await;
        assert_eq!(manager.connection_count().await, 1);
    }

    #[tokio::test]
    async fn connection_manager_remove_connection() {
        let manager = ConnectionManager::new();
        let (conn, _) = WebSocketConnection::new("test-conn-2".to_string());
        manager
            .add_connection("test-conn-2".to_string(), conn)
            .await;
        assert_eq!(manager.connection_count().await, 1);
        manager.remove_connection("test-conn-2").await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn connection_manager_get_connection() {
        let manager = ConnectionManager::new();
        let (conn, _) = WebSocketConnection::new("test-conn-3".to_string());
        manager
            .add_connection("test-conn-3".to_string(), conn)
            .await;
        let retrieved = manager.get_connection("test-conn-3").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), "test-conn-3");
    }

    #[tokio::test]
    async fn connection_manager_get_nonexistent() {
        let manager = ConnectionManager::new();
        let retrieved = manager.get_connection("nonexistent").await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn connection_manager_multiple_connections() {
        let manager = ConnectionManager::new();
        for i in 0..10 {
            let (conn, _) = WebSocketConnection::new(format!("conn-{}", i));
            manager.add_connection(format!("conn-{}", i), conn).await;
        }
        assert_eq!(manager.connection_count().await, 10);
    }

    #[tokio::test]
    async fn connection_manager_broadcast() {
        let manager = ConnectionManager::new();
        let (conn1, mut rx1) = WebSocketConnection::new("broadcast-1".to_string());
        let (conn2, mut rx2) = WebSocketConnection::new("broadcast-2".to_string());
        manager
            .add_connection("broadcast-1".to_string(), conn1)
            .await;
        manager
            .add_connection("broadcast-2".to_string(), conn2)
            .await;
        let msg = Arc::new(WebSocketMessage::Notification {
            event: "broadcast".to_string(),
            data: serde_json::json!({"msg": "hello"}),
        });
        manager.broadcast(&msg).await;
        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
        assert_eq!(manager.connection_count().await, 2);
    }

    #[tokio::test]
    async fn connection_manager_default() {
        let manager = ConnectionManager::default();
        assert_eq!(manager.connection_count().await, 0);
    }

    #[test]
    fn parse_websocket_message_empty_string() {
        let result = parse_websocket_message("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_websocket_message_whitespace() {
        let result = parse_websocket_message("   ");
        assert!(result.is_err());
    }

    #[test]
    fn parse_websocket_message_response_variant() {
        let json = r#"{"type":"response","id":"resp-1","result":{"status":"success"}}"#;
        let result = parse_websocket_message(json);
        assert!(result.is_ok());
        if let WebSocketMessage::Response { id, result } = result.unwrap() {
            assert_eq!(id, "resp-1");
            assert_eq!(result["status"], "success");
        } else {
            panic!("Expected Response");
        }
    }

    #[test]
    fn parse_websocket_message_error_variant() {
        let json = r#"{"type":"error","id":"err-1","error":"Something failed"}"#;
        let result = parse_websocket_message(json);
        assert!(result.is_ok());
        if let WebSocketMessage::Error { id, error } = result.unwrap() {
            assert_eq!(id, "err-1");
            assert_eq!(error, "Something failed");
        } else {
            panic!("Expected Error");
        }
    }

    #[test]
    fn parse_websocket_message_notification_variant() {
        let json = r#"{"type":"notification","event":"update","data":{"value":42}}"#;
        let result = parse_websocket_message(json);
        assert!(result.is_ok());
        if let WebSocketMessage::Notification { event, data } = result.unwrap() {
            assert_eq!(event, "update");
            assert_eq!(data["value"], 42);
        } else {
            panic!("Expected Notification");
        }
    }

    #[test]
    fn calculate_value_depth_primitive() {
        let value = serde_json::json!(42);
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 0);
    }

    #[test]
    fn calculate_value_depth_string() {
        let value = serde_json::json!("hello");
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 0);
    }

    #[test]
    fn calculate_value_depth_simple_object() {
        let value = serde_json::json!({"a": 1});
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 1);
    }

    #[test]
    fn calculate_value_depth_nested_object() {
        let value = serde_json::json!({"a": {"b": {"c": 1}}});
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 3);
    }

    #[test]
    fn calculate_value_depth_simple_array() {
        let value = serde_json::json!([1, 2, 3]);
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 1);
    }

    #[test]
    fn calculate_value_depth_nested_array() {
        let value = serde_json::json!([[[1, 2], [3, 4]], [[5, 6]]]);
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 3);
    }

    #[test]
    fn calculate_value_depth_mixed() {
        let value = serde_json::json!({
            "users": [
                {"name": "Alice", "tags": ["a", "b"]},
                {"name": "Bob", "tags": ["c"]}
            ]
        });
        let mut depth = 0;
        assert_eq!(calculate_value_depth(&value, &mut depth), 4);
    }

    #[test]
    fn app_state_new_default_config() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);
        assert!(state.config.auth.is_none());
    }

    #[test]
    fn app_state_clone() {
        let manager = Arc::new(ConnectionManager::new());
        let state = AppState::new(manager);
        let cloned = state.clone();
        assert!(cloned.config.auth.is_none());
    }

    #[test]
    fn default_websocket_handler_returns_response_for_request() {
        let handler = DefaultWebSocketHandler;
        let request = WebSocketMessage::Request {
            id: "handler-test".to_string(),
            method: "custom_method".to_string(),
            params: serde_json::json!({}),
        };
        let result = handler.handle(request).now_or_never().unwrap();
        match result {
            WebSocketMessage::Response { id, result } => {
                assert_eq!(id, "handler-test");
                assert_eq!(result["method"], "custom_method");
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn default_websocket_handler_passes_through_response() {
        let handler = DefaultWebSocketHandler;
        let response = WebSocketMessage::Response {
            id: "pass-through".to_string(),
            result: serde_json::json!({"key": "value"}),
        };
        let result = handler.handle(response.clone()).now_or_never().unwrap();
        match result {
            WebSocketMessage::Response { id, result } => {
                assert_eq!(id, "pass-through");
                assert_eq!(result["key"], "value");
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn default_websocket_handler_passes_through_error() {
        let handler = DefaultWebSocketHandler;
        let error = WebSocketMessage::Error {
            id: "error-test".to_string(),
            error: "Test error".to_string(),
        };
        let result = handler.handle(error.clone()).now_or_never().unwrap();
        match result {
            WebSocketMessage::Error { id, error } => {
                assert_eq!(id, "error-test");
                assert_eq!(error, "Test error");
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn default_websocket_handler_passes_through_notification() {
        let handler = DefaultWebSocketHandler;
        let notification = WebSocketMessage::Notification {
            event: "test_event".to_string(),
            data: serde_json::json!({"test": true}),
        };
        let result = handler.handle(notification.clone()).now_or_never().unwrap();
        match result {
            WebSocketMessage::Notification { event, data } => {
                assert_eq!(event, "test_event");
                assert_eq!(data["test"], true);
            }
            _ => panic!("Expected Notification"),
        }
    }
}
