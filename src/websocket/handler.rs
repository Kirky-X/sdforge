// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
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
    Router,
    extract::ws::{WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
// AUTHORIZATION header is only referenced inside the `security`-gated bearer-token
// extraction in `ValidatedWebSocketUpgrade::from_request`; gated separately so
// `http,websocket` (without `security`) does not warn about an unused import.
#[cfg(all(feature = "websocket", feature = "security"))]
use axum::http::header::AUTHORIZATION;
#[cfg(feature = "websocket")]
use futures_util::SinkExt;
#[cfg(feature = "websocket")]
use futures_util::StreamExt;
#[cfg(feature = "websocket")]
use std::pin::Pin;
#[cfg(feature = "websocket")]
use std::sync::Arc;

#[cfg(feature = "websocket")]
use crate::websocket::{AppState, ConnectionManager};
#[cfg(feature = "websocket")]
use crate::websocket::{MAX_MESSAGE_SIZE, WebSocketMessage, parse_websocket_message};

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
    /// 该路由对应的自定义消息处理器；由 `build()` 链路注入。
    /// 为 `None` 时回退到 `DefaultWebSocketHandler`（diting HIGH-002 修复）。
    handler: Option<Arc<dyn WebSocketHandler>>,
}

#[cfg(feature = "websocket")]
impl ValidatedWebSocketUpgrade {
    /// 为本次升级绑定自定义消息处理器（`build()` 在按路由注册时注入）。
    pub fn with_handler(mut self, handler: Arc<dyn WebSocketHandler>) -> Self {
        self.handler = Some(handler);
        self
    }
}

#[cfg(feature = "websocket")]
impl IntoResponse for ValidatedWebSocketUpgrade {
    fn into_response(self) -> Response {
        let handler = self
            .handler
            .unwrap_or_else(|| Arc::new(DefaultWebSocketHandler));
        self.ws
            .on_upgrade(move |socket| handle_socket(socket, self.manager.clone(), handler))
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

        // Get bearer token from Authorization header.
        // Only needed when the `security` feature is enabled (for auth validation);
        // gated to avoid an unused-variable warning when security is off.
        #[cfg(feature = "security")]
        let bearer_token: Option<String> = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(String::from);

        // Get AppState from request extensions (injected via with_state by axum)
        // The state parameter is &Arc<AppState> since that's what we registered
        let app_state = req.extensions().get::<Arc<AppState>>().cloned();

        // Validate auth if configured. The `auth` field only exists when the
        // `security` feature is enabled (see WebSocketConfig), so the entire
        // validation block is gated to match.
        //
        // two credential paths are accepted — bearer JWT (existing) or
        // `x-api-key` against the optional API-key store. Both reuse the HTTP
        // stack's credential stores; either success authenticates the
        // handshake, otherwise the upgrade is rejected with 401.
        #[cfg(feature = "security")]
        if let Some(ref state_ref) = app_state {
            let bearer_cfg = state_ref.config.auth.as_ref();
            let api_cfg = state_ref.config.api_key_auth.as_ref();

            if bearer_cfg.is_some() || api_cfg.is_some() {
                let bearer_ok = bearer_cfg
                    .zip(bearer_token.as_deref())
                    .and_then(|(auth, token)| auth.validate_token(token))
                    .is_some();

                let api_key = req.headers().get("x-api-key").and_then(|v| v.to_str().ok());
                let api_ok = api_cfg
                    .zip(api_key)
                    .and_then(|(store, key)| store.validate_key(key, "unknown"))
                    .is_some();

                if !bearer_ok && !api_ok {
                    return Err(StatusCode::UNAUTHORIZED);
                }
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

        Ok(Self {
            ws,
            manager,
            handler: None,
        })
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
async fn handle_socket(
    socket: WebSocket,
    manager: Arc<ConnectionManager>,
    handler: Arc<dyn WebSocketHandler>,
) {
    // adopt/create a request context for this connection's lifetime so
    // logs emitted from the message loop carry correlation ids.
    #[cfg(feature = "context")]
    {
        let ctx = crate::context::current_or_new();
        crate::context::scope(ctx, handle_socket_inner(socket, manager, handler)).await
    }
    #[cfg(not(feature = "context"))]
    {
        handle_socket_inner(socket, manager, handler).await;
    }
}

#[cfg(feature = "websocket")]
async fn handle_socket_inner(
    socket: WebSocket,
    manager: Arc<ConnectionManager>,
    handler: Arc<dyn WebSocketHandler>,
) {
    let conn_id = uuid::Uuid::new_v4().to_string();
    let (conn, mut receiver) = WebSocketConnection::new(conn_id.clone());
    manager.add_connection(conn_id.clone(), conn.clone()).await;

    // RAII 清理（HIGH 修复）：此前超大消息路径 close 后直接 return，跳过函数
    // 末尾唯一的 remove_connection，且无任何 Drop 兜底——连接条目在
    // ConnectionManager 中永久泄漏，可被远程滥用为连接表耗尽 DoS。
    // Guard 保证所有退出路径（超大消息早退、客户端断开、任务被取消）都清理。
    struct ConnectionGuard {
        conn_id: String,
        manager: Arc<ConnectionManager>,
    }
    impl Drop for ConnectionGuard {
        fn drop(&mut self) {
            let manager = self.manager.clone();
            let conn_id = self.conn_id.clone();
            // Drop 是同步上下文：把异步移除交给运行时。
            // remove_connection 幂等（重复移除不会下溢计数器）。
            tokio::spawn(async move {
                manager.remove_connection(&conn_id).await;
            });
        }
    }
    let _guard = ConnectionGuard {
        conn_id: conn_id.clone(),
        manager: manager.clone(),
    };

    // 广播修复（HIGH）：此前 `WebSocketConnection::new` 返回的 receiver 被
    // 直接丢弃且无处 spawn，manager 注册的所有连接都是死通道——broadcast
    // 对真实连接必然失败并将其误删。现在所有出站消息统一经通道投递，由
    // forwarder 任务序列化并写回 socket；请求/响应也走同一路径。
    let (mut sink, mut stream) = socket.split();
    let forwarder = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            let json = serde_json::to_string(&msg).unwrap_or_else(|_| {
                serde_json::to_string(&WebSocketMessage::Error {
                    id: String::new(),
                    error: "Internal serialization error".to_string(),
                })
                .unwrap_or_else(|_| {
                    r#"{"type":"error","id":"","error":"Internal error"}"#.to_string()
                })
            });
            if sink
                .send(axum::extract::ws::Message::Text(json.into()))
                .await
                .is_err()
            {
                // socket 已关闭（客户端断开）：退出 forwarder，
                // 全部 sender 随之 drop 后通道自然排空。
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = stream.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_text() {
                    // Check message size early
                    if text.len() > MAX_MESSAGE_SIZE {
                        // Close connection immediately to prevent DoS。
                        // 连接清理由 ConnectionGuard 的 Drop 兜底；abort forwarder
                        // 释放 sink 半边，socket 随之整体丢弃断开。
                        forwarder.abort();
                        return;
                    }

                    match parse_websocket_message(text) {
                        Ok(ws_msg) => {
                            // 使用该连接路由绑定的自定义 handler（diting HIGH-002 修复）；
                            // 未注入时回退到 DefaultWebSocketHandler。
                            // 响应经通道由 forwarder 写回，保证与广播一致的单出站路径。
                            let response = handler.handle(ws_msg).await;
                            let _ = conn.send(response).await;
                        }
                        Err(e) => {
                            let error_msg = WebSocketMessage::Error {
                                id: String::new(),
                                error: e,
                            };
                            let _ = conn.send(error_msg).await;
                        }
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    // forwarder 在所有 sender（本地 conn + manager 中的注册项）drop 后随
    // 通道关闭自然退出；此处主动 abort 只是尽快释放 sink 半边。
    forwarder.abort();
    // Cleanup（ConnectionGuard 兜底，此处显式执行以同步移除）
    manager.remove_connection(&conn_id).await;
}

#[cfg(feature = "websocket")]
use crate::websocket::WebSocketConnection;

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
        // 将路由的自定义 handler 注入升级提取器，使连接真正分发到用户 handler
        // （diting HIGH-002 修复）
        router = router.route(
            &path,
            axum::routing::get(move |ws: ValidatedWebSocketUpgrade| async move {
                ws.with_handler((route.create_fn)())
            })
            .with_state(state.clone()),
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

    /// 广播修复回归（HIGH）：此前 handle_socket 丢弃 receiver 且无处 spawn，
    /// manager 注册的连接全部是死通道——broadcast 必然投递失败并误删连接。
    /// 修复后广播必须真正到达客户端，且连接保持注册。
    #[tokio::test]
    async fn broadcast_reaches_connected_client() {
        let manager = Arc::new(ConnectionManager::new());
        let state = Arc::new(AppState::new(manager.clone()));
        let app = Router::new()
            .route("/ws", axum::routing::get(websocket_upgrade))
            .layer(axum::Extension(state));
        let server = axum_test::TestServer::builder().http_transport().build(app);

        let mut ws = server.get_websocket("/ws").await.into_websocket().await;
        // 等待服务端完成连接注册
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        assert_eq!(manager.connection_count().await, 1);

        let notification = Arc::new(WebSocketMessage::Notification {
            event: "ping".to_string(),
            data: serde_json::json!({"hello": "world"}),
        });
        manager.broadcast(&notification).await;

        let received = tokio::time::timeout(tokio::time::Duration::from_secs(2), ws.receive_json())
            .await
            .expect("broadcast must reach the client within timeout");
        match received {
            WebSocketMessage::Notification { event, data } => {
                assert_eq!(event, "ping");
                assert_eq!(data["hello"], "world");
            }
            other => panic!("Expected Notification, got {:?}", other),
        }
        assert_eq!(
            manager.connection_count().await,
            1,
            "connection must stay registered after a successful broadcast"
        );
    }

    /// 连接泄漏修复回归（HIGH）：此前超大消息路径 close 后直接 return，
    /// 跳过 remove_connection 且无 Drop 兜底，连接条目在 manager 中永久
    /// 泄漏（可被远程滥用为连接表耗尽 DoS）。修复后所有退出路径都清理。
    #[tokio::test]
    async fn oversized_message_removes_connection_from_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = Arc::new(AppState::new(manager.clone()));
        let app = Router::new()
            .route("/ws", axum::routing::get(websocket_upgrade))
            .layer(axum::Extension(state));
        let server = axum_test::TestServer::builder().http_transport().build(app);

        let mut ws = server.get_websocket("/ws").await.into_websocket().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        assert_eq!(manager.connection_count().await, 1);

        let oversized = "x".repeat(MAX_MESSAGE_SIZE + 1);
        ws.send_text(&oversized).await;

        let cleaned = tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
            while manager.connection_count().await > 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        })
        .await;
        assert!(
            cleaned.is_ok(),
            "connection entry must be removed after oversized-message close (leak)"
        );
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
