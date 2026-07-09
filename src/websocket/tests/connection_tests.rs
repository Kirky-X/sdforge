// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::websocket::connection::*;
use crate::websocket::message::*;
use futures_util::FutureExt;
use std::sync::Arc;

/// Test WebSocketConnection creation
#[test]
fn test_websocket_connection_new() {
    let (conn, mut receiver) = WebSocketConnection::new("conn-001".to_string());
    assert_eq!(conn.id(), "conn-001");
    assert!(!conn.id().is_empty());
    // Receiver should be ready to receive
    assert!(receiver.recv().now_or_never().is_none());
}

/// Test WebSocketConfig default values.
///
/// Replaces the old `test_rate_limit_config_default` (R-websocket-003):
/// `max_message_size` is now a top-level field (default 1 MiB), and
/// `rate_limit` (when `ratelimit` feature is on) is a `FlowControlConfig`.
#[test]
fn test_websocket_config_default() {
    let config = WebSocketConfig::default();
    assert_eq!(config.max_message_size, 1_048_576);
    #[cfg(feature = "ratelimit")]
    {
        // FlowControlConfig::default() has an empty rules vec; we only
        // assert that the field exists and is the default value.
        let expected = limiteron::config::FlowControlConfig::default();
        assert_eq!(config.rate_limit.rules.len(), expected.rules.len());
    }
}

/// Test ConnectionManager creation
#[test]
fn test_connection_manager_new() {
    let manager = ConnectionManager::new();
    // Just verify it can be created without panic
    let _ = manager;
}

/// Test WebSocketConfig default has no auth configured
#[cfg(feature = "security")]
#[test]
fn test_websocket_config_default_no_auth() {
    let config = WebSocketConfig::default();
    assert!(config.auth.is_none());
    assert_eq!(config.max_message_size, 1_048_576);
}

/// Test WebSocketConfig with BearerAuth configured
#[cfg(feature = "security")]
#[test]
fn test_websocket_config_with_auth() {
    let auth = crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .expect("valid secret");
    let config = WebSocketConfig {
        auth: Some(auth),
        ..Default::default()
    };
    assert!(config.auth.is_some());
}

/// Test AppState creation with custom config
#[cfg(feature = "security")]
#[test]
fn test_app_state_with_config() {
    use std::sync::Arc;
    let manager = Arc::new(ConnectionManager::new());
    let auth = crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .expect("valid secret");
    let config = WebSocketConfig {
        auth: Some(auth),
        ..Default::default()
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

/// Test WebSocketConfig clone preserves `max_message_size` and (when
/// `ratelimit` is on) the `rate_limit` field.
#[cfg(feature = "security")]
#[test]
fn websocket_config_clone() {
    let config = WebSocketConfig::default();
    let cloned = config.clone();
    assert_eq!(config.auth.is_some(), cloned.auth.is_some());
    assert_eq!(config.max_message_size, cloned.max_message_size);
}

/// Test WebSocketConfig clone preserves `max_message_size` without `security`.
#[test]
fn websocket_config_clone_max_message_size() {
    let config = WebSocketConfig::default();
    let cloned = config.clone();
    assert_eq!(config.max_message_size, cloned.max_message_size);
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

#[cfg(feature = "security")]
#[test]
fn app_state_new_default_config() {
    let manager = Arc::new(ConnectionManager::new());
    let state = AppState::new(manager);
    assert!(state.config.auth.is_none());
}

#[cfg(feature = "security")]
#[test]
fn app_state_clone() {
    let manager = Arc::new(ConnectionManager::new());
    let state = AppState::new(manager);
    let cloned = state.clone();
    assert!(cloned.config.auth.is_none());
}

/// Test ConnectionManager::broadcast with empty connections
#[tokio::test]
async fn connection_manager_broadcast_empty() {
    let manager = ConnectionManager::new();
    let msg = Arc::new(WebSocketMessage::Notification {
        event: "test".to_string(),
        data: serde_json::json!({}),
    });
    manager.broadcast(&msg).await;
    assert_eq!(manager.connection_count().await, 0);
}

/// Test ConnectionManager::broadcast with single connection
#[tokio::test]
async fn connection_manager_broadcast_single() {
    let manager = ConnectionManager::new();
    let (conn, mut rx) = WebSocketConnection::new("single-broadcast".to_string());
    manager
        .add_connection("single-broadcast".to_string(), conn)
        .await;
    let msg = Arc::new(WebSocketMessage::Notification {
        event: "single".to_string(),
        data: serde_json::json!({"value": 1}),
    });
    manager.broadcast(&msg).await;
    let received = rx.recv().await;
    assert!(received.is_some());
    if let Some(WebSocketMessage::Notification { event, data }) = received {
        assert_eq!(event, "single");
        assert_eq!(data["value"], 1);
    } else {
        panic!("Expected Notification");
    }
}

/// Test AppState::with_config preserves custom `max_message_size`.
///
/// Replaces the old `app_state_with_config_preserves_rate_limit` test
/// (R-websocket-003): the `RateLimitConfig` struct is gone; we now verify
/// that the migrated `max_message_size` field survives the round-trip
/// through `AppState::with_config`.
#[test]
fn app_state_with_config_preserves_max_message_size() {
    let manager = Arc::new(ConnectionManager::new());
    let config = WebSocketConfig {
        max_message_size: 2048,
        ..Default::default()
    };
    let state = AppState::with_config(config, manager.clone());
    assert_eq!(state.config.max_message_size, 2048);
}

/// Test AppState::with_config preserves custom config (with auth)
#[cfg(feature = "security")]
#[test]
fn app_state_with_config_preserves_settings() {
    let manager = Arc::new(ConnectionManager::new());
    let config = WebSocketConfig {
        auth: None,
        max_message_size: 2048,
        ..Default::default()
    };
    let state = AppState::with_config(config, manager.clone());
    assert_eq!(state.config.max_message_size, 2048);
}

/// Test AppState clone shares underlying data
#[test]
fn app_state_clone_shares_data() {
    let manager = Arc::new(ConnectionManager::new());
    let state = AppState::new(manager.clone());
    let cloned = state.clone();
    assert!(Arc::ptr_eq(&state.manager, &cloned.manager));
    assert!(Arc::ptr_eq(&state.config, &cloned.config));
}

/// Test AppState full config with auth
#[cfg(feature = "security")]
#[test]
fn app_state_full_config() {
    let manager = Arc::new(ConnectionManager::new());
    let auth = crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .expect("valid secret");
    let config = WebSocketConfig {
        auth: Some(auth),
        max_message_size: 2_097_152,
        ..Default::default()
    };
    let state = AppState::with_config(config, manager);
    assert!(state.config.auth.is_some());
    assert_eq!(state.config.max_message_size, 2_097_152);
}

/// Test WebSocketConnection send with different message types
#[tokio::test]
async fn websocket_connection_send_request() {
    let (conn, mut receiver) = WebSocketConnection::new("send-req".to_string());
    let msg = WebSocketMessage::Request {
        id: "send-req".to_string(),
        method: "test".to_string(),
        params: serde_json::json!({}),
    };
    conn.send(msg).await.unwrap();
    let received = receiver.recv().await.unwrap();
    assert!(matches!(received, WebSocketMessage::Request { .. }));
}

#[tokio::test]
async fn websocket_connection_send_response() {
    let (conn, mut receiver) = WebSocketConnection::new("send-resp".to_string());
    let msg = WebSocketMessage::Response {
        id: "send-resp".to_string(),
        result: serde_json::json!({"ok": true}),
    };
    conn.send(msg).await.unwrap();
    let received = receiver.recv().await.unwrap();
    assert!(matches!(received, WebSocketMessage::Response { .. }));
}

#[tokio::test]
async fn websocket_connection_send_error() {
    let (conn, mut receiver) = WebSocketConnection::new("send-err".to_string());
    let msg = WebSocketMessage::Error {
        id: "send-err".to_string(),
        error: "test error".to_string(),
    };
    conn.send(msg).await.unwrap();
    let received = receiver.recv().await.unwrap();
    assert!(matches!(received, WebSocketMessage::Error { .. }));
}

/// Test multiple sends on same connection
#[tokio::test]
async fn websocket_connection_multiple_sends() {
    let (conn, mut receiver) = WebSocketConnection::new("multi-send".to_string());
    for i in 0..5 {
        let msg = WebSocketMessage::Notification {
            event: format!("event-{}", i),
            data: serde_json::json!({"index": i}),
        };
        conn.send(msg).await.unwrap();
    }
    for i in 0..5 {
        let received = receiver.recv().await.unwrap();
        if let WebSocketMessage::Notification { event, data } = received {
            assert_eq!(event, format!("event-{}", i));
            assert_eq!(data["index"], i);
        } else {
            panic!("Expected Notification");
        }
    }
}

/// Test ConnectionManager::get_connection returns None for removed connection
#[tokio::test]
async fn connection_manager_get_removed_connection() {
    let manager = ConnectionManager::new();
    let (conn, _) = WebSocketConnection::new("removed-test".to_string());
    manager
        .add_connection("removed-test".to_string(), conn)
        .await;
    manager.remove_connection("removed-test").await;
    assert!(manager.get_connection("removed-test").await.is_none());
}

// ============================================================================
// broadcast() error cleanup path
// ============================================================================

/// Test broadcast cleans up connections whose receiver has been dropped.
///
/// Covers the `failed_connections` cleanup path in `broadcast()`: when
/// `conn.send()` returns an error (because the receiver was dropped),
/// the failed connection is removed from the manager.
#[tokio::test]
async fn broadcast_cleans_up_failed_connections() {
    let manager = ConnectionManager::new();
    let (doomed, rx) = WebSocketConnection::new("doomed-conn".to_string());
    let (healthy, mut rx2) = WebSocketConnection::new("healthy-conn".to_string());
    manager
        .add_connection("doomed-conn".to_string(), doomed)
        .await;
    manager
        .add_connection("healthy-conn".to_string(), healthy)
        .await;
    assert_eq!(manager.connection_count().await, 2);

    // Drop the receiver so sends to "doomed-conn" fail.
    drop(rx);

    let msg = Arc::new(WebSocketMessage::Notification {
        event: "cleanup-test".to_string(),
        data: serde_json::json!({}),
    });
    manager.broadcast(&msg).await;

    // The healthy connection should still receive the message.
    assert!(rx2.recv().await.is_some());
    // The doomed connection should have been cleaned up.
    assert_eq!(manager.connection_count().await, 1);
    assert!(manager.get_connection("doomed-conn").await.is_none());
    assert!(manager.get_connection("healthy-conn").await.is_some());
}

// ============================================================================
// RwLock poison branch coverage (lines 196, 212, 229-230)
//
// ConnectionManager::add_connection / remove_connection / get_connection each
// have an `else` branch that handles a poisoned RwLock. These branches were
// previously uncovered because no test poisons the `connections` RwLock.
// ============================================================================

/// Cover the RwLock poison branch in `add_connection` (line 196).
#[tokio::test]
async fn add_connection_handles_poisoned_lock() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let manager = ConnectionManager::new();
    let connections = manager.connections.clone();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = connections.write().unwrap();
        panic!("intentional panic to poison RwLock");
    }));
    let (conn, _) = WebSocketConnection::new("poison-test".to_string());
    manager
        .add_connection("poison-test".to_string(), conn)
        .await;
    assert_eq!(manager.connection_count().await, 0);
}

/// Cover the RwLock poison branch in `remove_connection` (line 212).
#[tokio::test]
async fn remove_connection_handles_poisoned_lock() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let manager = ConnectionManager::new();
    let connections = manager.connections.clone();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = connections.write().unwrap();
        panic!("intentional panic to poison RwLock");
    }));
    manager.remove_connection("any-id").await;
    assert_eq!(manager.connection_count().await, 0);
}

/// Cover the RwLock poison branch in `get_connection` (lines 229-230).
#[tokio::test]
async fn get_connection_handles_poisoned_lock() {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    let manager = ConnectionManager::new();
    let connections = manager.connections.clone();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _guard = connections.write().unwrap();
        panic!("intentional panic to poison RwLock");
    }));
    let result = manager.get_connection("any-id").await;
    assert!(result.is_none());
}
