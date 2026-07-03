// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! WebSocket connection management.
//!
//! Provides [`WebSocketConnection`] for individual connections, [`ConnectionManager`]
//! for tracking active connections with rate limiting, and the related
//! configuration types ([`RateLimitConfig`], [`WebSocketConfig`], [`AppState`]).
//!
//! Broadcast logic lives in [`crate::websocket::broadcast`].

#[cfg(feature = "websocket")]
use dashmap::DashMap;
#[cfg(feature = "websocket")]
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
#[cfg(feature = "websocket")]
use std::sync::Arc;

#[cfg(feature = "websocket")]
use crate::websocket::message::WebSocketMessage;

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
    /// Active connections keyed by ID.
    ///
    /// `pub(crate)` so the [`crate::websocket::broadcast`] module can iterate
    /// connections without re-implementing the dashmap iterator type path
    /// (which differs across dashmap versions).
    pub(crate) connections: Arc<DashMap<String, WebSocketConnection>>,
    /// Rate limiting: message count per connection per window.
    ///
    /// `pub(crate)` so unit tests in `tests/connection_tests.rs` can inspect
    /// rate-limit counters after exercising `check_and_record`.
    pub(crate) message_counts: Arc<DashMap<String, AtomicU64>>,
    /// Rate limiting: connection count tracking.
    ///
    /// `pub(crate)` so unit tests can assert on the live counter.
    pub(crate) connection_count: Arc<AtomicUsize>,
    /// Rate limiting: track messages per time window.
    ///
    /// `pub(crate)` so unit tests can inspect window-reset behavior.
    pub(crate) last_message_time: Arc<DashMap<String, AtomicU64>>,
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
    ///
    /// Only available when the `security` feature is enabled; omitted
    /// otherwise so that `http,websocket` (without `security`) compiles.
    #[cfg(feature = "security")]
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
    ///
    /// Note: This method checks per-connection message rate only.
    /// Connection count is managed by `add_connection`/`remove_connection`.
    pub fn check_and_record(&self, conn_id: &str, config: &RateLimitConfig) -> bool {
        // Use SystemTime (unix timestamp) for rate limit window calculation.
        // Instant::now().elapsed() is always ~0 and was a bug that made rate limiting ineffective.
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Check max_connections as a read-only pre-check (increment is in add_connection)
        if self.connection_count.load(Ordering::SeqCst) >= config.max_connections {
            return true;
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
                if current_time.saturating_sub(last_time) >= config.rate_limit_window_seconds {
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
