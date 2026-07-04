// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for middleware execution via real requests: request ID middleware,
//! JWT and ApiKey auth middleware (security feature), and hot-reload
//! integration (hot-reload feature).

use axum::body::Body;
use axum::Router;

use crate::config::{AppConfig, AuthConfig, ServerConfig};
use crate::http::{build_with_config, X_REQUEST_ID};

// ============================================================================
// Request ID Middleware Execution Tests (covers lines 226-244)
// These tests send real requests through the router to exercise the
// middleware closure body, which is never executed by build-only tests.
// ============================================================================

#[tokio::test]
async fn test_request_id_middleware_generates_uuid_when_absent() {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::None,
        timeout: None,
    };
    let router = build_with_config(&config).unwrap();
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let request_id = response
        .headers()
        .get(X_REQUEST_ID)
        .expect("response should have x-request-id header");
    let id_str = request_id.to_str().unwrap();
    assert_eq!(id_str.len(), 36, "generated request ID should be a UUID");
}

#[tokio::test]
async fn test_request_id_middleware_preserves_custom_id() {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::None,
        timeout: None,
    };
    let router = build_with_config(&config).unwrap();
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header(X_REQUEST_ID, "my-custom-id-789")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let request_id = response.headers().get(X_REQUEST_ID).unwrap();
    assert_eq!(request_id.to_str().unwrap(), "my-custom-id-789");
}

#[tokio::test]
async fn test_request_id_middleware_non_utf8_header_generates_uuid() {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::None,
        timeout: None,
    };
    let router = build_with_config(&config).unwrap();
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header(
                X_REQUEST_ID,
                axum::http::HeaderValue::from_bytes(b"\xff\xfe").unwrap(),
            )
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let request_id = response.headers().get(X_REQUEST_ID).unwrap();
    let id_str = request_id.to_str().unwrap();
    assert_eq!(id_str.len(), 36, "should generate UUID for non-UTF8 header");
}

// ============================================================================
// JWT Auth Middleware Execution Tests (security feature, covers lines 334-359)
// These tests exercise the extract_auth closure body for JWT auth.
// ============================================================================

#[cfg(feature = "security")]
fn create_test_jwt(secret: &[u8], payload: &serde_json::Value) -> String {
    use base64::Engine;
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(&header).unwrap());
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_string(payload).unwrap());

    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature);

    format!("{}.{}", signing_input, signature_b64)
}

#[cfg(feature = "security")]
fn build_jwt_test_router(secret: &str) -> Router {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::Jwt {
            secret: secret.to_string(),
        },
        timeout: None,
    };
    build_with_config(&config).unwrap()
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_jwt_auth_missing_header_returns_401() {
    let router = build_jwt_test_router("MySecureSecret123!@#ABCDEFGHIJKLM");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_jwt_auth_non_bearer_token_returns_401() {
    let router = build_jwt_test_router("MySecureSecret123!@#ABCDEFGHIJKLM");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_jwt_auth_empty_bearer_token_returns_401() {
    let router = build_jwt_test_router("MySecureSecret123!@#ABCDEFGHIJKLM");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("authorization", "Bearer ")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_jwt_auth_invalid_token_returns_401() {
    let router = build_jwt_test_router("MySecureSecret123!@#ABCDEFGHIJKLM");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("authorization", "Bearer invalid.token.here")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_jwt_auth_valid_token_returns_200() {
    let secret = "MySecureSecret123!@#ABCDEFGHIJKLM";
    let router = build_jwt_test_router(secret);

    let payload = serde_json::json!({
        "sub": "test-user",
        "permissions": ["read"],
        "iat": chrono::Utc::now().timestamp(),
        "exp": chrono::Utc::now().timestamp() + 3600
    });
    let token = create_test_jwt(secret.as_bytes(), &payload);

    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

// ============================================================================
// ApiKey Auth Middleware Execution Tests (security feature, covers lines 288-328)
// These tests exercise the extract_auth closure body for ApiKey auth.
//
// NOTE: The ApiKey success path (lines 316-321) is UNREACHABLE through
// build_with_config() because AppApiKeyAuth::new() creates an empty
// authenticator with no registered keys. All valid-format keys will fail
// validation. This is a design limitation of build_with_config().
// ============================================================================

#[cfg(feature = "security")]
fn build_apikey_test_router(header_name: &str, prefix: &str) -> Router {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
        },
        authentication: AuthConfig::ApiKey {
            header_name: header_name.to_string(),
            prefix: prefix.to_string(),
        },
        timeout: None,
    };
    build_with_config(&config).unwrap()
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_apikey_auth_missing_header_returns_401() {
    let router = build_apikey_test_router("X-API-Key", "key-");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_apikey_auth_wrong_prefix_returns_401() {
    let router = build_apikey_test_router("X-API-Key", "key-");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("X-API-Key", "Bearer-something")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_apikey_auth_empty_prefix_returns_401() {
    // Config with empty prefix — security check rejects empty prefix (lines 310-312)
    let router = build_apikey_test_router("X-API-Key", "");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("X-API-Key", "some-key-value")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_apikey_auth_unregistered_key_returns_401() {
    // Valid format (correct prefix) but key is not registered in the
    // empty AppApiKeyAuth — validate_key returns None (lines 322-324)
    let router = build_apikey_test_router("X-API-Key", "key-");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("X-API-Key", "key-unregistered-key-123")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_apikey_auth_with_x_forwarded_for_header() {
    // Test client IP extraction path (lines 300-306) with x-forwarded-for
    let router = build_apikey_test_router("X-API-Key", "key-");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("X-API-Key", "key-test")
            .header("x-forwarded-for", "203.0.113.50")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    // Should still be 401 because key is not registered
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[cfg(feature = "security")]
#[tokio::test]
async fn test_apikey_auth_with_x_real_ip_header() {
    // Test client IP extraction fallback to x-real-ip (lines 302-304)
    let router = build_apikey_test_router("X-API-Key", "key-");
    let response = tower::ServiceExt::oneshot(
        router,
        axum::http::Request::builder()
            .uri("/cov-test-route")
            .header("X-API-Key", "key-test")
            .header("x-real-ip", "203.0.113.99")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}
