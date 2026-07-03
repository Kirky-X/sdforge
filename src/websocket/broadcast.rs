// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! WebSocket broadcast logic.
//!
//! Implements the [`ConnectionManager::broadcast`] method that sends a single
//! message to every active connection. Failed sends are tracked and the
//! affected connections are pruned from the manager to avoid leaking stale
//! entries.

#[cfg(feature = "websocket")]
use std::sync::Arc;

#[cfg(feature = "websocket")]
use crate::websocket::connection::ConnectionManager;
#[cfg(feature = "websocket")]
use crate::websocket::message::WebSocketMessage;

#[cfg(feature = "websocket")]
impl ConnectionManager {
    /// Broadcast a message to all connections (optimized with Arc)
    ///
    /// Security fix: Handle broadcast errors properly and clean up failed connections.
    /// Connections whose receiver has been dropped are removed from the manager
    /// so subsequent broadcasts do not repeatedly attempt to deliver to dead channels.
    pub async fn broadcast(&self, message: &Arc<WebSocketMessage>) {
        let mut failed_connections: Vec<String> = Vec::new();

        for conn in self.connections.iter() {
            if conn.send(message.as_ref().clone()).await.is_err() {
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
}
