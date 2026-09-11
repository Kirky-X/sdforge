// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T712 e2e: non-HTTP protocol authentication — gRPC interceptor and WS
//! handshake, reusing the HTTP credential stores (bearer JWT / API key).

#![cfg(all(feature = "security", feature = "grpc", feature = "websocket"))]

use std::sync::Arc;

use sdforge::grpc::sdforge_v1::sd_forge_service_server::SdForgeService;
use sdforge::grpc::SdForgeGrpcService;
use sdforge::security::grpc_auth::{ApiKeyVerifier, BearerVerifier};
use sdforge::security::AppApiKeyAuth;

// =============================================================================
// gRPC interceptor
// =============================================================================

#[sdforge::forge(
    name = "grpc_auth_ping",
    version = "v1",
    grpc_method = "auth_ping",
    description = "Authenticated gRPC ping"
)]
async fn grpc_auth_ping() -> Result<serde_json::Value, sdforge::core::ApiError> {
    Ok(serde_json::json!({"pong": true}))
}

fn grpc_service_with(verifier: Arc<dyn sdforge::security::grpc_auth::GrpcAuthVerifier>) -> SdForgeGrpcService {
    SdForgeGrpcService::default().with_auth_interceptor(verifier)
}

fn call_request(auth: Option<(&'static str, String)>) -> sdforge::tonic::Request<sdforge::grpc::sdforge_v1::CallRequest> {
    let mut req = sdforge::tonic::Request::new(sdforge::grpc::sdforge_v1::CallRequest {
        method: "auth_ping".to_string(),
        parameters: Default::default(),
        data: String::new(),
    });
    if let Some((key, value)) = auth {
        req.metadata_mut()
            .insert(key, value.parse().expect("valid header value"));
    }
    req
}

#[tokio::test]
async fn grpc_without_credentials_is_unauthenticated() {
    let service = grpc_service_with(Arc::new(BearerVerifier::from_secret(
        "Grpc-Test-Secret-Key-0123456789-AbCdEf",
    ).unwrap()));
    let err = service.call(call_request(None)).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn grpc_with_invalid_bearer_is_unauthenticated() {
    let service = grpc_service_with(Arc::new(BearerVerifier::from_secret(
        "Grpc-Test-Secret-Key-0123456789-AbCdEf",
    ).unwrap()));
    let err = service
        .call(call_request(Some(("authorization", "Bearer bogus.token.here".to_string()))))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn grpc_with_valid_bearer_proceeds_to_dispatch() {
    let service = grpc_service_with(Arc::new(BearerVerifier::from_secret(
        "Grpc-Test-Secret-Key-0123456789-AbCdEf",
    ).unwrap()));

    let token = mint_jwt("Grpc-Test-Secret-Key-0123456789-AbCdEf");
    let res = service
        .call(call_request(Some(("authorization", format!("Bearer {token}")))))
        .await
        .expect("valid bearer must pass the interceptor");
    // Authenticated dispatch reached the forge handler.
    assert_eq!(res.get_ref().data, r#"{"pong":true}"#);
}

#[tokio::test]
async fn grpc_with_valid_api_key_proceeds_to_dispatch() {
    let store = Arc::new(AppApiKeyAuth::new());
    store.add_key("grpc-secret-key-9", vec!["admin".to_string()]);
    let service =
        grpc_service_with(Arc::new(ApiKeyVerifier::new(store, "sk_")));

    let err = service
        .call(call_request(Some(("x-api-key", "sk_wrong-key".to_string()))))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated);

    let res = service
        .call(call_request(Some(("x-api-key", "sk_grpc-secret-key-9".to_string()))))
        .await
        .expect("valid api key must pass the interceptor");
    assert_eq!(res.get_ref().data, r#"{"pong":true}"#);
}

// Minimal HS256 JWT minter (same shape the HTTP middleware validates).
fn mint_jwt(secret: &str) -> String {
    use base64::Engine;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&serde_json::json!({"sub": "grpc-test", "exp": 9999999999u64})).unwrap());
    let signing_input = format!("{header_b64}.{payload_b64}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signing_input.as_bytes());
    let sig_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(mac.finalize().into_bytes());
    format!("{signing_input}.{sig_b64}")
}

// =============================================================================
// WebSocket handshake auth
// =============================================================================

mod ws_handshake {
    use super::*;
    use axum::body::Body;
    use sdforge::websocket::{websocket_upgrade, AppState, ConnectionManager, WebSocketConfig};
    use tower::ServiceExt;

    fn ws_app_with(config: WebSocketConfig) -> axum::Router {
        let state = Arc::new(AppState {
            config: Arc::new(config),
            manager: Arc::new(ConnectionManager::new()),
        });
        axum::Router::new()
            .route("/ws", axum::routing::get(websocket_upgrade))
            .layer(axum::extract::Extension(state))
    }

    async fn upgrade_with(headers: &[(&str, &str)]) -> axum::http::StatusCode {
        let mut builder = axum::http::Request::builder()
            .method("GET")
            .uri("/ws")
            .header("host", "localhost")
            .header("connection", "Upgrade")
            .header("upgrade", "websocket")
            .header("sec-websocket-version", "13")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==");
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        let resp = ws_app_with(ws_config())
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        resp.status()
    }

    fn ws_config() -> WebSocketConfig {
        let mut config = WebSocketConfig::default();
        config.auth = Some(
            sdforge::security::BearerAuth::try_new("Ws-Test-Secret-Key-0123456789-AbCdEf")
                .unwrap(),
        );
        let store = Arc::new(AppApiKeyAuth::new());
        store.add_key("ws-secret-key-1", vec!["viewer".to_string()]);
        config.api_key_auth = Some(store);
        config
    }

    #[tokio::test]
    async fn ws_without_credentials_is_rejected_401() {
        assert_eq!(upgrade_with(&[]).await, axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn ws_with_invalid_credentials_is_rejected_401() {
        let status = upgrade_with(&[
            ("authorization", "Bearer bogus.token.here"),
            ("x-api-key", "wrong-key"),
        ])
        .await;
        assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn ws_with_valid_api_key_passes_handshake_auth() {
        let status = upgrade_with(&[("x-api-key", "ws-secret-key-1")]).await;
        // Auth gate passed: the request leaves the 401 auth-rejection path.
        // In a `tower::oneshot` there is no hyper upgrade channel, so axum's
        // `WebSocketUpgrade` extractor fails with 400 AFTER our validation —
        // 400 proves the handshake auth accepted the key (401 would mean the
        // gate rejected it).
        assert_ne!(status, axum::http::StatusCode::UNAUTHORIZED, "valid api key must authenticate");
    }

    #[tokio::test]
    async fn ws_with_valid_bearer_passes_handshake_auth() {
        let token = mint_jwt("Ws-Test-Secret-Key-0123456789-AbCdEf");
        let status = upgrade_with(&[("authorization", &format!("Bearer {token}"))]).await;
        assert_ne!(
            status,
            axum::http::StatusCode::UNAUTHORIZED,
            "valid bearer must authenticate (status {status})"
        );
    }
}
