// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::websocket::connection::*;
use crate::websocket::message::*;
use futures_util::FutureExt;
use std::sync::atomic::Ordering;
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

/// Test WebSocketConfig default has no auth configured
#[cfg(feature = "security")]
#[test]
fn test_websocket_config_default_no_auth() {
    let config = WebSocketConfig::default();
    assert!(config.auth.is_none());
    assert_eq!(config.rate_limit.max_connections, 1000);
    assert_eq!(config.rate_limit.max_messages_per_second, 100);
}

/// Test WebSocketConfig with BearerAuth configured
#[cfg(feature = "security")]
#[test]
fn test_websocket_config_with_auth() {
    let auth = crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .expect("valid secret");
    let config = WebSocketConfig {
        auth: Some(auth),
        rate_limit: RateLimitConfig::default(),
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

#[cfg(feature = "security")]
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

#[test]
fn websocket_config_clone_rate_limit() {
    let config = WebSocketConfig::default();
    let cloned = config.clone();
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

/// Test check_and_record allows first message within limits
#[test]
fn check_and_record_first_message_allowed() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 10,
        max_connections: 100,
        rate_limit_window_seconds: 1,
        ..Default::default()
    };
    assert!(!manager.check_and_record("conn-1", &config));
}

/// Test check_and_record triggers rate limiting after exceeding limit
#[test]
fn check_and_record_exceeds_rate_limit() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 2,
        max_connections: 100,
        rate_limit_window_seconds: 10,
        ..Default::default()
    };
    assert!(!manager.check_and_record("conn-rate", &config));
    assert!(!manager.check_and_record("conn-rate", &config));
    assert!(manager.check_and_record("conn-rate", &config));
}

/// Test check_and_record respects connection limit
///
/// Note: `check_and_record` is a read-only pre-check; `connection_count` is
/// incremented by `add_connection`. This test simulates active connections
/// by directly setting `connection_count` to verify the limit check.
#[test]
fn check_and_record_exceeds_connection_limit() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 100,
        max_connections: 3,
        rate_limit_window_seconds: 10,
        ..Default::default()
    };
    // Under the limit: not rate limited
    manager.connection_count.fetch_add(2, Ordering::SeqCst);
    assert!(!manager.check_and_record("conn-a", &config));
    // At the limit: new connection check rejected
    manager.connection_count.fetch_add(1, Ordering::SeqCst);
    assert!(manager.check_and_record("conn-d", &config));
}

/// Test check_and_record tracks independent connections
#[test]
fn check_and_record_independent_connections() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 1,
        max_connections: 100,
        rate_limit_window_seconds: 10,
        ..Default::default()
    };
    assert!(!manager.check_and_record("conn-x", &config));
    assert!(!manager.check_and_record("conn-y", &config));
    assert!(manager.check_and_record("conn-x", &config));
    assert!(manager.check_and_record("conn-y", &config));
}

/// Test check_and_record with exact connection boundary
///
/// Note: `check_and_record` is a read-only pre-check; `connection_count` is
/// incremented by `add_connection`. This test simulates active connections
/// by directly setting `connection_count` to verify the boundary check.
#[test]
fn check_and_record_exact_connection_boundary() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 100,
        max_connections: 2,
        rate_limit_window_seconds: 10,
        ..Default::default()
    };
    // Under the limit: not rate limited
    manager.connection_count.fetch_add(1, Ordering::SeqCst);
    assert!(!manager.check_and_record("conn-1", &config));
    // At the limit: new connection check rejected
    manager.connection_count.fetch_add(1, Ordering::SeqCst);
    assert!(manager.check_and_record("conn-3", &config));
}

/// Test check_and_record with exact message rate boundary
#[test]
fn check_and_record_exact_message_boundary() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 3,
        max_connections: 100,
        rate_limit_window_seconds: 10,
        ..Default::default()
    };
    assert!(!manager.check_and_record("conn-msg", &config));
    assert!(!manager.check_and_record("conn-msg", &config));
    assert!(!manager.check_and_record("conn-msg", &config));
    assert!(manager.check_and_record("conn-msg", &config));
}

/// Test check_and_record resets the counter when the rate limit window elapses.
///
/// Covers the window-reset branch (lines 313-315): when
/// `current_time - last_time >= rate_limit_window_seconds`, the message count
/// is reset to 0 and the last-message timestamp is refreshed, allowing a
/// previously rate-limited connection to send again.
#[test]
fn check_and_record_resets_after_window_elapsed() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig {
        max_messages_per_second: 2,
        max_connections: 100,
        rate_limit_window_seconds: 10,
        ..Default::default()
    };

    // Exhaust the per-connection message budget.
    assert!(!manager.check_and_record("conn-reset", &config));
    assert!(!manager.check_and_record("conn-reset", &config));
    // Third message within the same window → rate limited.
    assert!(manager.check_and_record("conn-reset", &config));

    // Simulate the window having elapsed by backdating the last-message time.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backdated = now.saturating_sub(config.rate_limit_window_seconds + 1);
    {
        let map = manager.last_message_time.read().unwrap();
        if let Some(entry) = map.get("conn-reset") {
            entry.store(backdated, Ordering::Relaxed);
        }
    }

    // After the window has elapsed, the counter resets and the connection
    // is allowed to send again (not rate limited).
    assert!(
        !manager.check_and_record("conn-reset", &config),
        "Connection should be allowed again after the rate limit window elapses"
    );
}

/// Test check_and_record with concurrent connections
#[test]
fn check_and_record_concurrent_connections() {
    use std::thread;
    let manager = Arc::new(ConnectionManager::new());
    let config = Arc::new(RateLimitConfig {
        max_messages_per_second: 100,
        max_connections: 10,
        rate_limit_window_seconds: 10,
        ..Default::default()
    });
    let mut handles = vec![];
    for i in 0..5 {
        let mgr = manager.clone();
        let cfg = config.clone();
        handles.push(thread::spawn(move || {
            let conn_id = format!("thread-conn-{}", i);
            mgr.check_and_record(&conn_id, &cfg)
        }));
    }
    let results: Vec<bool> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    for result in &results {
        assert!(!result, "All concurrent connections should be allowed");
    }
}

/// Test ConnectionManager::remove_connection cleans up rate limit data
#[tokio::test]
async fn connection_manager_remove_cleans_up_rate_limit_data() {
    let manager = ConnectionManager::new();
    let config = RateLimitConfig::default();
    let (conn, _) = WebSocketConnection::new("cleanup-test".to_string());
    manager
        .add_connection("cleanup-test".to_string(), conn)
        .await;
    manager.check_and_record("cleanup-test", &config);
    manager.remove_connection("cleanup-test").await;
    assert!(manager
        .message_counts
        .read()
        .unwrap()
        .get("cleanup-test")
        .is_none());
    assert!(manager
        .last_message_time
        .read()
        .unwrap()
        .get("cleanup-test")
        .is_none());
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

/// Test AppState::new creates with default config
#[cfg(feature = "security")]
#[test]
fn app_state_new_creates_default_config() {
    let manager = Arc::new(ConnectionManager::new());
    let state = AppState::new(manager.clone());
    assert!(state.config.auth.is_none());
    assert_eq!(state.config.rate_limit.max_connections, 1000);
}

/// Test AppState::with_config preserves custom rate_limit config
#[test]
fn app_state_with_config_preserves_rate_limit() {
    let manager = Arc::new(ConnectionManager::new());
    let config = WebSocketConfig {
        rate_limit: RateLimitConfig {
            max_messages_per_second: 50,
            max_message_size: 2048,
            max_connections: 500,
            rate_limit_window_seconds: 30,
        },
        ..Default::default()
    };
    let state = AppState::with_config(config, manager.clone());
    assert_eq!(state.config.rate_limit.max_messages_per_second, 50);
    assert_eq!(state.config.rate_limit.max_message_size, 2048);
    assert_eq!(state.config.rate_limit.max_connections, 500);
    assert_eq!(state.config.rate_limit.rate_limit_window_seconds, 30);
}

/// Test AppState::with_config preserves custom config (with auth)
#[cfg(feature = "security")]
#[test]
fn app_state_with_config_preserves_settings() {
    let manager = Arc::new(ConnectionManager::new());
    let config = WebSocketConfig {
        auth: None,
        rate_limit: RateLimitConfig {
            max_messages_per_second: 50,
            max_message_size: 2048,
            max_connections: 500,
            rate_limit_window_seconds: 30,
        },
    };
    let state = AppState::with_config(config, manager.clone());
    assert_eq!(state.config.rate_limit.max_messages_per_second, 50);
    assert_eq!(state.config.rate_limit.max_message_size, 2048);
    assert_eq!(state.config.rate_limit.max_connections, 500);
    assert_eq!(state.config.rate_limit.rate_limit_window_seconds, 30);
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
        rate_limit: RateLimitConfig {
            max_messages_per_second: 200,
            max_message_size: 2_097_152,
            max_connections: 5000,
            rate_limit_window_seconds: 60,
        },
    };
    let state = AppState::with_config(config, manager);
    assert!(state.config.auth.is_some());
    assert_eq!(state.config.rate_limit.max_messages_per_second, 200);
}

/// Test RateLimitConfig Debug output
#[test]
fn rate_limit_config_debug_output() {
    let config = RateLimitConfig {
        max_messages_per_second: 42,
        max_message_size: 4096,
        max_connections: 50,
        rate_limit_window_seconds: 5,
    };
    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("42"));
    assert!(debug_str.contains("4096"));
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

/// Test RateLimitConfig validate with all maximum valid values
#[test]
fn rate_limit_config_all_max_valid() {
    let config = RateLimitConfig {
        max_messages_per_second: 1_000_000,
        max_message_size: 100_000_000,
        max_connections: 100_000,
        rate_limit_window_seconds: 86400,
    };
    assert!(config.validate().is_ok());
}

/// Test RateLimitConfig validate with all minimum valid values
#[test]
fn rate_limit_config_all_min_valid() {
    let config = RateLimitConfig {
        max_messages_per_second: 1,
        max_message_size: 1,
        max_connections: 1,
        rate_limit_window_seconds: 1,
    };
    assert!(config.validate().is_ok());
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
