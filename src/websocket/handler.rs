// Copyright (c) 2026 Kirky.X🌠
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
use futures_util::SinkExt;
use futures_util::StreamExt;
use std::pin::Pin;
use std::sync::Arc;

use crate::websocket::{ConnectionManager, SdForgeState};
use crate::websocket::{MAX_MESSAGE_SIZE, WebSocketMessage, parse_websocket_message};

use crate::core::ApiMetadata;

/// Boxed future type for async WebSocket handling
pub type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// WebSocket handler trait
pub trait WebSocketHandler: Send + Sync {
    /// Handle a WebSocket message and return a response
    fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage>;
}

/// WebSocket route registration entry (compile-time, collected via `inventory`).
///
/// Hand-written instead of `define_registration!` because it carries the
/// declared mount path (`#[forge(ws_path = "...")]` /
/// [`WebSocketRoute::with_path`]): the forge `name` is an internal identifier,
/// so a declared `ws_path` cannot be recovered from it. The instance type is
/// `Arc<dyn WebSocketHandler>` so handlers are shared across connections.
#[derive(Debug, Clone, Copy)]
pub struct WebSocketRoute {
    /// API name
    pub name: &'static str,
    /// API version
    pub version: &'static str,
    /// Function that creates the instance at runtime
    pub create_fn: fn() -> Arc<dyn WebSocketHandler>,
    /// Function that creates the metadata at runtime
    pub metadata_fn: fn() -> ApiMetadata,
    /// Declared mount path (`ws_path`). Empty = derive from `name`.
    pub path: &'static str,
}

impl WebSocketRoute {
    /// Create a registration without an explicit path (mount path derives
    /// from `name`).
    pub const fn new(
        name: &'static str,
        version: &'static str,
        create_fn: fn() -> Arc<dyn WebSocketHandler>,
        metadata_fn: fn() -> ApiMetadata,
    ) -> Self {
        Self {
            name,
            version,
            create_fn,
            metadata_fn,
            path: "",
        }
    }

    /// Declare the mount path (`#[forge(ws_path = "...")]`); const so
    /// `inventory::submit!` keeps accepting the builder chain.
    #[must_use]
    pub const fn with_path(mut self, path: &'static str) -> Self {
        self.path = path;
        self
    }

    /// Resolve the mount path served by `build()`: a declared `ws_path` wins,
    /// otherwise the path derives from `name`. A missing leading slash is
    /// normalized so axum never receives a panic-inducing relative path.
    pub fn mount_path(&self) -> String {
        if self.path.is_empty() {
            format!("/{}", self.name)
        } else {
            format!("/{}", self.path.trim_start_matches('/'))
        }
    }
}

impl crate::core::Registration for WebSocketRoute {
    type Instance = Arc<dyn WebSocketHandler>;
    type Metadata = ApiMetadata;

    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> &str {
        self.version
    }
    fn create(&self) -> Self::Instance {
        (self.create_fn)()
    }
    fn metadata(&self) -> Self::Metadata {
        (self.metadata_fn)()
    }
}

inventory::collect!(WebSocketRoute);

/// Default implementation of WebSocketHandler
pub struct DefaultWebSocketHandler;

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
/// 2. If auth is configured in `SdForgeState`, validates the bearer token (returns 401 if invalid)
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
pub struct ValidatedWebSocketUpgrade {
    ws: WebSocketUpgrade,
    manager: Arc<ConnectionManager>,
    /// 该路由对应的自定义消息处理器；由 `build()` 链路注入。
    /// 为 `None` 时回退到 `DefaultWebSocketHandler`。
    handler: Option<Arc<dyn WebSocketHandler>>,
}

impl ValidatedWebSocketUpgrade {
    /// 为本次升级绑定自定义消息处理器（`build()` 在按路由注册时注入）。
    pub fn with_handler(mut self, handler: Arc<dyn WebSocketHandler>) -> Self {
        self.handler = Some(handler);
        self
    }
}

impl IntoResponse for ValidatedWebSocketUpgrade {
    fn into_response(self) -> Response {
        let handler = self
            .handler
            .unwrap_or_else(|| Arc::new(DefaultWebSocketHandler));
        self.ws
            .on_upgrade(move |socket| handle_socket(socket, self.manager.clone(), handler))
    }
}

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

        // Get SdForgeState from either wiring channel:
        // - router state (`MethodRouter::with_state`, the wiring `build()`
        //   uses): axum bakes the state into the handler service and hands it
        //   to `from_request` as `&S` — it is never written into request
        //   extensions;
        // - an `Extension` layer (the pattern used by handler_tests): the
        //   state arrives in request extensions.
        // Consulting both keeps the shared `ConnectionManager`/auth config
        // reachable instead of silently falling back to a fresh per-connection
        // manager (which breaks `broadcast` and ignores ws auth).
        let app_state = (state as &dyn std::any::Any)
            .downcast_ref::<Arc<SdForgeState>>()
            .cloned()
            .or_else(|| req.extensions().get::<Arc<SdForgeState>>().cloned());

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

/// WebSocket upgrade handler with optional JWT authentication.
///
/// Security: When `WebSocketConfig::auth` is `Some`, this handler validates
/// the `Authorization: Bearer <token>` header before upgrading the connection.
/// Invalid or missing tokens result in HTTP 401 Unauthorized.
pub async fn websocket_upgrade(ws: ValidatedWebSocketUpgrade) -> impl IntoResponse {
    ws // IntoResponse performs the upgrade
}

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
                            // 使用该连接路由绑定的自定义 handler；
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

use crate::websocket::WebSocketConnection;

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
    let state = Arc::new(SdForgeState::new(manager));

    for route in inventory::iter::<WebSocketRoute> {
        // Declared `ws_path` wins; registrations without one mount at
        // `/{name}` (see `WebSocketRoute::mount_path`).
        let path = route.mount_path();
        // 将路由的自定义 handler 注入升级提取器，使连接真正分发到用户 handler
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
        let state = Arc::new(SdForgeState::new(manager.clone()));
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

    /// `with_state` 接线回归（HIGH）：`build()` 经 `MethodRouter::with_state`
    /// 注入 `Arc<SdForgeState>`，axum 把它按值烘进 handler service、从不写入
    /// request extensions。此前提取器只查 extensions，`build()` 注册的路由
    /// 全部回退到每连接新建的 ConnectionManager：共享 manager 上
    /// `connection_count()==0`，broadcast 永不到达客户端。修复后两条注入
    /// 通道（router state / Extension）都必须命中同一 manager。
    #[tokio::test]
    async fn with_state_pattern_shares_connection_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = Arc::new(SdForgeState::new(manager.clone()));
        let app = Router::new().route(
            "/ws",
            axum::routing::get(websocket_upgrade).with_state(state),
        );
        let server = axum_test::TestServer::builder().http_transport().build(app);

        let mut ws = server.get_websocket("/ws").await.into_websocket().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        assert_eq!(
            manager.connection_count().await,
            1,
            "with_state must reach the shared ConnectionManager"
        );

        let notification = Arc::new(WebSocketMessage::Notification {
            event: "state-pattern".to_string(),
            data: serde_json::json!({"via": "with_state"}),
        });
        manager.broadcast(&notification).await;
        let received = tokio::time::timeout(tokio::time::Duration::from_secs(2), ws.receive_json())
            .await
            .expect("broadcast must reach the client wired via with_state");
        assert!(matches!(
            received,
            WebSocketMessage::Notification { ref event, .. } if event == "state-pattern"
        ));
    }

    /// `with_state` 通道下的 ws 认证回归：此前 state 不可达时认证配置被静默
    /// 忽略（无 token 也 101）；修复后配置的 BearerAuth 必须在升级前生效。
    #[cfg(feature = "security")]
    #[tokio::test]
    async fn with_state_pattern_enforces_configured_auth() {
        let auth =
            crate::security::BearerAuth::try_new("ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ")
                .expect("valid secret");
        let config = crate::websocket::WebSocketConfig {
            auth: Some(auth),
            ..Default::default()
        };
        let manager = Arc::new(ConnectionManager::new());
        let state = Arc::new(SdForgeState::with_config(config, manager));
        let app = Router::new().route(
            "/ws",
            axum::routing::get(websocket_upgrade).with_state(state),
        );
        let server = axum_test::TestServer::builder().http_transport().build(app);

        let response = server.get("/ws").await;
        response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
    }

    /// `ws_path` 声明回归：显式挂载路径必须优先于 `name` 派生路径，且缺少
    /// 前导斜杠时归一化（axum 对相对路径 panic）。
    #[test]
    fn mount_path_prefers_declared_ws_path() {
        fn handler() -> Arc<dyn WebSocketHandler> {
            Arc::new(DefaultWebSocketHandler)
        }
        fn metadata() -> ApiMetadata {
            ApiMetadata {
                name: "drv_ws_internal".to_string(),
                version: "v1".to_string(),
                description: String::new(),
                cache_ttl: None,
                is_streaming: true,
                i18n_key: None,
                lifecycle: None,
            }
        }

        let derived = WebSocketRoute::new("drv_ws_internal", "v1", handler, metadata);
        assert_eq!(derived.mount_path(), "/drv_ws_internal");
        assert_eq!(derived.path, "");

        let declared =
            WebSocketRoute::new("drv_ws_internal", "v1", handler, metadata).with_path("/ws/driver");
        assert_eq!(declared.mount_path(), "/ws/driver");

        let normalized =
            WebSocketRoute::new("drv_ws_internal", "v1", handler, metadata).with_path("ws/driver");
        assert_eq!(normalized.mount_path(), "/ws/driver");
    }

    /// 连接泄漏修复回归（HIGH）：此前超大消息路径 close 后直接 return，
    /// 跳过 remove_connection 且无 Drop 兜底，连接条目在 manager 中永久
    /// 泄漏（可被远程滥用为连接表耗尽 DoS）。修复后所有退出路径都清理。
    #[tokio::test]
    async fn oversized_message_removes_connection_from_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = Arc::new(SdForgeState::new(manager.clone()));
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
