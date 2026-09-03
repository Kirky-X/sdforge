// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
use crate::core::ApiMetadata;
use crate::core::Registration;
use crate::websocket::*;
use axum::Router;
#[cfg(feature = "security")]
use axum::http::StatusCode;
#[cfg(feature = "security")]
use axum::http::header::AUTHORIZATION;
use futures_util::FutureExt;
#[cfg(feature = "security")]
use std::sync::Arc;

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

/// Test build() creates router without panic
#[test]
fn build_router_creates_without_panic() {
    let router = build();
    drop(router);
}

/// Test DefaultWebSocketHandler with complex params
#[test]
fn default_handler_complex_params() {
    let handler = DefaultWebSocketHandler;
    let request = WebSocketMessage::Request {
        id: "complex".to_string(),
        method: "complex_method".to_string(),
        params: serde_json::json!({
            "nested": {"key": "value"},
            "array": [1, 2, 3],
            "bool": true,
            "null": null
        }),
    };
    let result = handler.handle(request).now_or_never().unwrap();
    match result {
        WebSocketMessage::Response { id, result } => {
            assert_eq!(id, "complex");
            assert_eq!(result["method"], "complex_method");
            assert_eq!(result["status"], "ok");
        }
        _ => panic!("Expected Response"),
    }
}

/// Test WebSocketRoute with custom handler
#[test]
fn websocket_route_custom_handler() {
    use std::sync::Arc;
    struct EchoHandler;
    impl WebSocketHandler for EchoHandler {
        fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
            Box::pin(async move { message })
        }
    }
    let route = WebSocketRoute::new(
        "/echo",
        "v2",
        || Arc::new(EchoHandler),
        || ApiMetadata {
            name: "/echo".to_string(),
            version: "v2".to_string(),
            description: "Echo handler".to_string(),
            cache_ttl: None,
            is_streaming: false,
        },
    );
    assert_eq!(route.name(), "/echo");
    assert_eq!(route.version(), "v2");
}

// ============================================================================
// handle_socket() message loop tests (via TestServer WebSocket)
// ============================================================================

/// Helper: build a test server with the websocket_upgrade handler (no auth).
fn build_ws_test_server() -> axum_test::TestServer {
    let app = Router::new().route("/ws", axum::routing::get(websocket_upgrade));
    axum_test::TestServer::builder().http_transport().build(app)
}

/// Test handle_socket processes a Request and returns a Response.
///
/// Covers the main message-loop path: parse → DefaultWebSocketHandler →
/// serialize → send. Also exercises `IntoResponse` and `websocket_upgrade`.
#[tokio::test]
async fn handle_socket_processes_request_and_returns_response() {
    let server = build_ws_test_server();
    let mut ws = server.get_websocket("/ws").await.into_websocket().await;

    let request = WebSocketMessage::Request {
        id: "req-1".to_string(),
        method: "get_data".to_string(),
        params: serde_json::json!({"key": "value"}),
    };
    ws.send_json(&request).await;

    let response: WebSocketMessage = ws.receive_json().await;
    match response {
        WebSocketMessage::Response { id, result } => {
            assert_eq!(id, "req-1");
            assert_eq!(result["status"], "ok");
            assert_eq!(result["method"], "get_data");
        }
        _ => panic!("Expected Response, got {:?}", response),
    }
}

/// Test handle_socket returns an Error message for invalid JSON.
///
/// Covers the `Err(e)` branch of `parse_websocket_message` in the
/// message loop, which sends a `WebSocketMessage::Error` back.
#[tokio::test]
async fn handle_socket_handles_invalid_json() {
    let server = build_ws_test_server();
    let mut ws = server.get_websocket("/ws").await.into_websocket().await;

    ws.send_text("not valid json").await;

    let response: WebSocketMessage = ws.receive_json().await;
    match response {
        WebSocketMessage::Error { error, .. } => {
            assert!(error.contains("Invalid JSON"));
        }
        _ => panic!("Expected Error, got {:?}", response),
    }
}

/// Test handle_socket echoes non-Request messages (Notification).
///
/// Covers the `_ => message` passthrough branch of
/// `DefaultWebSocketHandler::handle`.
#[tokio::test]
async fn handle_socket_echoes_notification_messages() {
    let server = build_ws_test_server();
    let mut ws = server.get_websocket("/ws").await.into_websocket().await;

    let notification = WebSocketMessage::Notification {
        event: "test_event".to_string(),
        data: serde_json::json!({"value": 42}),
    };
    ws.send_json(&notification).await;

    let response: WebSocketMessage = ws.receive_json().await;
    match response {
        WebSocketMessage::Notification { event, data } => {
            assert_eq!(event, "test_event");
            assert_eq!(data["value"], 42);
        }
        _ => panic!("Expected Notification, got {:?}", response),
    }
}

// ============================================================================
// ValidatedWebSocketUpgrade FromRequest extractor tests
// ============================================================================

/// Test the extractor accepts a WebSocket upgrade when no auth is configured.
///
/// Covers the path where `app_state` is `None` (no AppState in extensions),
/// skipping the auth block and creating a default ConnectionManager
/// via the `unwrap_or_else` branch.
#[tokio::test]
async fn validated_websocket_upgrade_accepts_without_auth() {
    let app = Router::new().route("/ws", axum::routing::get(websocket_upgrade));
    let server = axum_test::TestServer::builder().http_transport().build(app);

    // A successful WS connect means the extractor accepted the request.
    let mut ws = server.get_websocket("/ws").await.into_websocket().await;
    // Send a request to confirm the connection is functional.
    let request = WebSocketMessage::Request {
        id: "no-auth-1".to_string(),
        method: "ping".to_string(),
        params: serde_json::json!({}),
    };
    ws.send_json(&request).await;
    let response: WebSocketMessage = ws.receive_json().await;
    assert!(matches!(response, WebSocketMessage::Response { .. }));
}

/// Helper: build a test server with auth configured via Extension layer.
#[cfg(feature = "security")]
fn build_ws_test_server_with_auth() -> axum_test::TestServer {
    let auth = crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        .expect("valid secret");
    let config = WebSocketConfig {
        auth: Some(auth),
        ..Default::default()
    };
    let manager = Arc::new(ConnectionManager::new());
    let app_state = Arc::new(AppState::with_config(config, manager));
    let app = Router::new()
        .route("/ws", axum::routing::get(websocket_upgrade))
        .layer(axum::Extension(app_state));
    axum_test::TestServer::builder().http_transport().build(app)
}

/// Test the extractor rejects with 401 when auth is configured but no
/// Authorization header is present.
///
/// Covers `bearer_token.ok_or(StatusCode::UNAUTHORIZED)?` — the missing
/// token rejection path.
#[cfg(feature = "security")]
#[tokio::test]
async fn validated_websocket_upgrade_rejects_missing_token_with_auth() {
    let server = build_ws_test_server_with_auth();

    let response = server.get("/ws").await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}

/// Test the extractor rejects with 401 when auth is configured and an
/// invalid bearer token is provided.
///
/// Covers `auth.validate_token(&token).ok_or(StatusCode::UNAUTHORIZED)?`
/// — the invalid token rejection path.
#[cfg(feature = "security")]
#[tokio::test]
async fn validated_websocket_upgrade_rejects_invalid_token_with_auth() {
    let server = build_ws_test_server_with_auth();

    let response = server
        .get("/ws")
        .add_header(AUTHORIZATION, "Bearer invalid-token-value")
        .await;
    response.assert_status(StatusCode::UNAUTHORIZED);
}
