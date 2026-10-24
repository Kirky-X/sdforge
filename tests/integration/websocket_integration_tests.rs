// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
// WebSocket Integration Tests
// Tests WebSocket upgrade with authentication flow

#[cfg(feature = "websocket")]
mod websocket_integration_tests {
    use sdforge::websocket::{ConnectionManager, SdForgeState, WebSocketConfig};
    use std::sync::Arc;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        let _cloned = config.clone();
        // `max_message_size` is now a top-level field (default 1 MiB).
        assert_eq!(config.max_message_size, 1_048_576);
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        let _ = manager;
    }

    #[test]
    fn test_app_state_with_manager() {
        let manager = Arc::new(ConnectionManager::new());
        let state = SdForgeState::new(manager);
        let _state_clone = state.clone();
        assert_eq!(state.config.max_message_size, 1_048_576);
    }

    #[test]
    fn test_websocket_integration_basic() {
        // Test basic WebSocket integration components
        let _config = WebSocketConfig::default();
        let manager = Arc::new(ConnectionManager::new());
        let state = SdForgeState::new(manager);

        // Basic verification - state was created successfully
        assert_eq!(state.config.max_message_size, 1_048_576);
    }
}

// Enhanced WebSocket Integration tests
#[cfg(feature = "websocket")]
mod websocket_enhanced_integration_tests {
    use sdforge::websocket::{ConnectionManager, WebSocketConnection};
    use std::sync::Arc;

    /// Test 1: WebSocket connection creation and basic operations
    #[tokio::test]
    async fn test_websocket_connection_creation() {
        let (conn, _rx) = WebSocketConnection::new("test-conn-1".to_string());

        assert_eq!(conn.id(), "test-conn-1");
    }

    /// Test 2: Connection manager with multiple connections
    #[tokio::test]
    async fn test_connection_manager_multiple_connections() {
        let manager = Arc::new(ConnectionManager::new());

        for i in 0..3 {
            let conn_id = format!("integration-conn-{}", i);
            let (conn, _rx) = WebSocketConnection::new(conn_id.clone());
            manager.add_connection(conn_id, conn).await;
        }

        assert_eq!(manager.connection_count().await, 3);

        let conn = manager.get_connection("integration-conn-1").await;
        assert!(conn.is_some());
        assert_eq!(conn.unwrap().id(), "integration-conn-1");
    }

    /// Test 3: Connection removal
    #[tokio::test]
    async fn test_connection_removal() {
        let manager = Arc::new(ConnectionManager::new());

        let (conn, _rx) = WebSocketConnection::new("remove-test".to_string());
        manager
            .add_connection("remove-test".to_string(), conn)
            .await;
        assert_eq!(manager.connection_count().await, 1);

        manager.remove_connection("remove-test").await;
        assert_eq!(manager.connection_count().await, 0);

        let conn = manager.get_connection("remove-test").await;
        assert!(conn.is_none());
    }
}

/// `#[forge(ws_path = "...")]` 回归：声明式挂载路径必须由宏注册进
/// `WebSocketRoute` 并被 `websocket::build` 消费（此前 ws_path 被解析后
/// 丢弃，路由只挂在 `/{name}` 上）。
#[cfg(feature = "websocket")]
mod websocket_ws_path_tests {
    use sdforge::websocket::{BoxFuture, WebSocketHandler, WebSocketMessage};
    use std::sync::Arc;

    struct Echo;
    impl WebSocketHandler for Echo {
        fn handle(&self, message: WebSocketMessage) -> BoxFuture<'static, WebSocketMessage> {
            Box::pin(async move { message })
        }
    }

    #[sdforge::forge(name = "it_ws_internal", version = "v1", ws_path = "/ws/it-declared")]
    fn it_ws_internal() -> Arc<dyn WebSocketHandler> {
        Arc::new(Echo)
    }

    #[test]
    fn registered_ws_route_exposes_declared_path() {
        let routes = sdforge::get_websocket_routes();
        let route = routes
            .iter()
            .find(|r| r.name == "it_ws_internal")
            .expect("ws route registered via #[forge(ws_path)]");
        assert_eq!(route.path, "/ws/it-declared");
        assert_eq!(route.mount_path(), "/ws/it-declared");
    }

    /// 端到端：`build` 后的路由必须在声明的 ws_path 上完成 101 升级。
    #[tokio::test]
    async fn build_serves_declared_ws_path() {
        let app = sdforge::websocket::build();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = sdforge::axum::serve(listener, app).await;
        });

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = format!(
            "GET /ws/it-declared HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        );
        sock.write_all(req.as_bytes()).await.unwrap();
        let mut buf = vec![0u8; 1024];
        let n = sock.read(&mut buf).await.unwrap();
        let head = String::from_utf8_lossy(&buf[..n]).to_string();
        assert!(
            head.starts_with("HTTP/1.1 101"),
            "declared ws_path must serve the upgrade: {head:?}"
        );
    }
}

/// `build_with_config` 装配回归：此前该入口没有任何测试守护——配置是否真的
/// 交给 `SdForgeState::with_config`、state 是否真的挂到路由上，任一处回归
/// 都不会被捕获（握手鉴权随之静默失效）。用例落在集成测试二进制内：该二进制
/// 已通过 `#[forge(ws_path)]` 注册了真实 ws 路由，`build_with_config` 才有
/// 可服务的路径。
#[cfg(all(feature = "websocket", feature = "security"))]
mod websocket_build_with_config_tests {
    use sdforge::security::BearerAuth;
    use sdforge::websocket::WebSocketConfig;

    const SECRET: &str = "ValidSecret123!ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const WS_PATH: &str = "/ws/it-declared";

    /// 用带 JWT 校验的配置装配路由（`build_with_config` 的核心入口）。
    fn app_with_auth() -> axum::Router {
        sdforge::websocket::build_with_config(WebSocketConfig {
            auth: Some(BearerAuth::try_new(SECRET).expect("valid secret")),
            ..Default::default()
        })
    }

    /// 在随机端口起服务并发起一次 ws 握手，返回响应首部文本。
    async fn handshake_head(app: axum::Router, headers: &[(&str, &str)]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = sdforge::axum::serve(listener, app).await;
        });

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut sock = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut req = format!(
            "GET {WS_PATH} HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n"
        );
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");
        sock.write_all(req.as_bytes()).await.unwrap();

        // 读到状态行结束为止，避免单次 read 只拿到半截响应。
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let n = sock.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.windows(2).any(|w| w == b"\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&buf).to_string()
    }

    /// 最小 HS256 JWT 签发器（校验口径与 HTTP 中间件一致：alg 必须 HS256、
    /// `exp` 必填）。
    fn mint_jwt(secret: &str) -> String {
        use base64::Engine;
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;

        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_string(&header).unwrap());
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            serde_json::to_string(
                &serde_json::json!({"sub": "ws-build-test", "exp": 9999999999u64}),
            )
            .unwrap(),
        );
        let signing_input = format!("{header_b64}.{payload_b64}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let sig_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{signing_input}.{sig_b64}")
    }

    /// 配置的 JWT 校验必须作用于 `build_with_config` 挂载的路由：无凭据 → 401。
    #[tokio::test]
    async fn build_with_config_rejects_handshake_without_credentials() {
        let head = handshake_head(app_with_auth(), &[]).await;
        assert!(
            head.starts_with("HTTP/1.1 401"),
            "handshake without credentials must be rejected: {head:?}"
        );
    }

    /// 错误令牌同样被拒（证明配置真的抵达校验路径，而非被忽略）。
    #[tokio::test]
    async fn build_with_config_rejects_handshake_with_invalid_bearer() {
        let head = handshake_head(
            app_with_auth(),
            &[("authorization", "Bearer bogus.token.here")],
        )
        .await;
        assert!(
            head.starts_with("HTTP/1.1 401"),
            "handshake with an invalid bearer must be rejected: {head:?}"
        );
    }

    /// 有效令牌必须放行到 101（证明配置与路由都装配到位）。
    #[tokio::test]
    async fn build_with_config_accepts_handshake_with_valid_bearer() {
        let token = mint_jwt(SECRET);
        let head = handshake_head(
            app_with_auth(),
            &[("authorization", &format!("Bearer {token}"))],
        )
        .await;
        assert!(
            head.starts_with("HTTP/1.1 101"),
            "handshake with a valid bearer must upgrade: {head:?}"
        );
    }
}
