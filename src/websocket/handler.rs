// Copyright (c) 2026 Kirky.X
//! WebSocket handler trait, default implementation, and Axum integration.
//!
//! Provides:
//! - [`WebSocketHandler`] trait for custom message handlers
//! - [`DefaultWebSocketHandler`] echoing non-Request messages and replying with
//!   a status JSON for Request messages
//! - [`ValidatedWebSocketUpgrade`] extractor that performs optional JWT auth
//!   before upgrading the connection
//! - `handle_socket` message loop invoked after a successful upgrade
//! - [`build`] function that assembles a [`Router`] from all registered
//!   [`WebSocketRoute`] entries

#[cfg(feature = "websocket")]
use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::{header::AUTHORIZATION, StatusCode},
    response::{IntoResponse, Response},
    Router,
};
#[cfg(feature = "websocket")]
use futures_util::SinkExt;
#[cfg(feature = "websocket")]
use futures_util::StreamExt;
#[cfg(feature = "websocket")]
use std::pin::Pin;
#[cfg(feature = "websocket")]
use std::sync::Arc;

#[cfg(feature = "websocket")]
use crate::websocket::connection::{AppState, ConnectionManager};
#[cfg(feature = "websocket")]
use crate::websocket::message::{parse_websocket_message, WebSocketMessage, MAX_MESSAGE_SIZE};

#[cfg(feature = "websocket")]
use crate::core::ApiMetadata;
#[cfg(feature = "websocket")]
use crate::define_registration;

#[cfg(feature = "websocket")]
/// Boxed future type for async WebSocket handling
pub type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

#[cfg(feature = "websocket")]
/// WebSocket handler trait
pub trait WebSocketHandler: Send + Sync {
    /// Handle a WebSocket message and return a response
    fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage>;
}

#[cfg(feature = "websocket")]
define_registration!(WebSocketRoute, Arc<dyn WebSocketHandler>, ApiMetadata);

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
                                    r#"{"type":"error","id":"","error":"Internal error processing request"}"#
                                        .to_string()
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
use crate::websocket::connection::WebSocketConnection;

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

#[cfg(feature = "websocket")]
#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;
    use std::panic::AssertUnwindSafe;

    /// Test that handle_socket closes the connection when a message exceeds
    /// MAX_MESSAGE_SIZE (lines 173-177).
    ///
    /// Sends a text message larger than MAX_MESSAGE_SIZE and verifies the
    /// server closes the connection without sending a normal WebSocketMessage
    /// response.
    #[tokio::test]
    async fn handle_socket_closes_connection_on_oversized_message() {
        let app = Router::new().route("/ws", axum::routing::get(websocket_upgrade));
        let server = axum_test::TestServer::builder().http_transport().build(app);

        let mut ws = server.get_websocket("/ws").await.into_websocket().await;

        // Send a message larger than MAX_MESSAGE_SIZE
        let oversized = "x".repeat(MAX_MESSAGE_SIZE + 1);
        ws.send_text(&oversized).await;

        // Give the server time to process and close the connection
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // The server should have closed the connection without responding.
        // We use catch_unwind because receive_text panics on a closed
        // connection in axum_test.
        let receive_result = tokio::time::timeout(
            tokio::time::Duration::from_millis(500),
            AssertUnwindSafe(ws.receive_text()).catch_unwind(),
        )
        .await;

        match receive_result {
            Err(_) => { /* Timeout — connection closed */ }
            Ok(Err(_)) => { /* Panic caught — connection closed */ }
            Ok(Ok(text)) => {
                // If we received text, it should NOT be a valid WebSocketMessage
                let parse_result: Result<WebSocketMessage, _> = serde_json::from_str(&text);
                assert!(
                    parse_result.is_err(),
                    "Should not receive a valid WebSocketMessage, got: {}",
                    text
                );
            }
        }
    }

    /// Test that handle_socket silently ignores binary messages (the implicit
    /// else of `if let Ok(text) = msg.to_text()` on line 171).
    ///
    /// Sends a binary message followed by a text Request, and verifies only
    /// the text Request gets a Response — the binary message is dropped.
    #[tokio::test]
    async fn handle_socket_ignores_binary_messages() {
        let app = Router::new().route("/ws", axum::routing::get(websocket_upgrade));
        let server = axum_test::TestServer::builder().http_transport().build(app);

        let mut ws = server.get_websocket("/ws").await.into_websocket().await;

        // Send a binary message with invalid UTF-8 — should be silently
        // ignored by handle_socket (msg.to_text() returns Err, so the
        // `if let Ok(text)` branch on line 171 is not taken).
        ws.send_message(axum_test::WsMessage::Binary(vec![0xFF, 0xFE, 0xFD].into()))
            .await;

        // Send a text Request — should get a Response
        let request = WebSocketMessage::Request {
            id: "after-binary".to_string(),
            method: "ping".to_string(),
            params: serde_json::json!({}),
        };
        ws.send_json(&request).await;

        // We should receive a Response (not the binary message)
        let response: WebSocketMessage = ws.receive_json().await;
        match response {
            WebSocketMessage::Response { id, result } => {
                assert_eq!(id, "after-binary");
                assert_eq!(result["status"], "ok");
            }
            _ => panic!("Expected Response, got {:?}", response),
        }
    }
}
