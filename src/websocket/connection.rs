// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! WebSocket connection management.
//!
//! Provides [`WebSocketConnection`] for individual connections, [`ConnectionManager`]
//! for tracking active connections, and the related configuration types
//! ([`WebSocketConfig`], [`AppState`]).
//!
//! Broadcast logic lives in [`crate::websocket::broadcast`].

#[cfg(feature = "websocket")]
use std::collections::HashMap;
#[cfg(feature = "websocket")]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "websocket")]
use std::sync::{Arc, RwLock};

#[cfg(feature = "ratelimit")]
use limiteron::config::FlowControlConfig;

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
/// Connection manager for WebSocket connections.
///
/// Connection-level rate limiting (message counting, window-based throttling)
/// was removed in the `integrate-limiteron-ratelimit` change. HTTP-level
/// rate limiting is now provided by `crate::security::ratelimit` (Tower
/// middleware backed by limiteron). Connection-level enforcement via a
/// limiteron `Governor` is a future follow-up (see `design.md` D6
/// "Out of Scope").
pub struct ConnectionManager {
    /// Active connections keyed by ID.
    ///
    /// `pub(crate)` so the [`crate::websocket::broadcast`] module can iterate
    /// connections without re-implementing the underlying iterator type path.
    pub(crate) connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    /// Active connection count.
    ///
    /// `pub(crate)` so unit tests can assert on the live counter.
    pub(crate) connection_count: Arc<AtomicUsize>,
}

/// WebSocket server configuration with optional JWT authentication.
///
/// # Security
///
/// When `auth` is `Some`, all WebSocket upgrade requests must include a valid
/// JWT bearer token in the `Authorization` header. Requests without a valid
/// token receive HTTP 401 and the connection is rejected.
///
/// # Rate Limiting
///
/// When the `ratelimit` feature is enabled, `rate_limit` holds a
/// `limiteron::config::FlowControlConfig` that can be applied by a limiteron
/// `Governor` at connection establishment (future follow-up, see `design.md`
/// D6 "Out of Scope"). When `ratelimit` is off, the field does not exist.
///
/// `max_message_size` is always present (independent of features) because it
/// is a simple byte-size cap, not a flow-control algorithm.
///
/// # Example
///
/// ```ignore
/// use sdforge::websocket::WebSocketConfig;
/// use sdforge::security::BearerAuth;
///
/// let auth = BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ").ok();
/// let config = WebSocketConfig {
///     auth,
///     max_message_size: 1_048_576,
///     #[cfg(feature = "ratelimit")]
///     rate_limit: limiteron::config::FlowControlConfig::default(),
/// };
/// ```
#[derive(Clone)]
pub struct WebSocketConfig {
    /// Optional JWT authentication validator.
    /// When `Some`, all connections must present a valid JWT bearer token.
    /// When `None`, connections are accepted without authentication.
    ///
    /// Only available when the `security` feature is enabled; omitted
    /// otherwise so that `http,websocket` (without `security`) compiles.
    ///
    /// # Security Warning (LOW-003)
    ///
    /// When the `security` feature is NOT enabled, this field does not exist
    /// and **all WebSocket upgrade requests are accepted without authentication**.
    /// To require JWT authentication, enable both `websocket` and `security` features:
    /// ```toml
    /// sdforge = { features = ["websocket", "security"] }
    /// ```
    #[cfg(feature = "security")]
    pub auth: Option<crate::security::BearerAuth>,
    /// Maximum message size in bytes. Default 1 MiB.
    ///
    /// Migrated out of the deleted `RateLimitConfig` (the old 4-field config
    /// struct). Kept as a top-level field because `FlowControlConfig` does not
    /// model message-size limits.
    pub max_message_size: usize,
    /// Flow-control configuration (limiteron).
    ///
    /// Only present when the `ratelimit` feature is enabled. When `ratelimit`
    /// is off, WebSocket has no rate-limit configuration.
    #[cfg(feature = "ratelimit")]
    pub rate_limit: FlowControlConfig,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "security")]
            auth: None,
            max_message_size: 1_048_576, // 1 MiB
            #[cfg(feature = "ratelimit")]
            rate_limit: FlowControlConfig::default(),
        }
    }
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
    /// Create a new connection manager.
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Add a connection to the manager
    pub async fn add_connection(&self, id: String, conn: WebSocketConnection) {
        {
            if let Ok(mut map) = self.connections.write() {
                map.insert(id, conn);
            } else {
                log::warn!("websocket connections lock poisoned; add_connection skipped");
                return;
            }
        }
        self.connection_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove a connection from the manager.
    ///
    /// BUG-1 修复: 仅在确实移除连接时才递减 `connection_count`，
    /// 避免并发 remove 同一 id 导致 usize 下溢为 `usize::MAX`。
    pub async fn remove_connection(&self, id: &str) {
        let existed = {
            if let Ok(mut map) = self.connections.write() {
                map.remove(id).is_some()
            } else {
                log::warn!("websocket connections lock poisoned; remove_connection skipped");
                return;
            }
        };
        if !existed {
            // 连接已被另一路径移除（如 broadcast 清理 + handler 断开同时触发），
            // 不递减计数器，避免下溢。
            return;
        }
        self.connection_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get a connection by ID
    pub async fn get_connection(&self, id: &str) -> Option<WebSocketConnection> {
        let map = match self.connections.read() {
            Ok(g) => g,
            Err(_) => {
                log::warn!("websocket connections lock poisoned; get_connection returned None");
                return None;
            }
        };
        map.get(id).cloned()
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
