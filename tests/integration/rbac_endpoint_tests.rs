// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! e2e: endpoint-level RBAC via `#[forge(auth(role = "..."))]`.
//!
//! The macro wraps the declared route with a role requirement; requests
//! authenticated without the role get 403 (not 401), requests with a
//! matching permission get 200, and unauthenticated requests still get 401
//! from the global auth middleware.

#![cfg(all(feature = "http", feature = "security"))]

use sdforge::config::{ApiKeySeed, AppConfig, AuthConfig, ServerConfig};
use sdforge::http::build_with_config;

#[sdforge::forge(
    name = "admin_stats",
    version = "v1",
    description = "Admin-only stats",
    path = "/admin/stats",
    method = "GET",
    auth(role = "admin")
)]
async fn admin_stats() -> serde_json::Value {
    serde_json::json!({ "secret": true })
}

#[sdforge::forge(
    name = "open_ping",
    version = "v1",
    description = "Unprotected endpoint",
    path = "/open/ping",
    method = "GET"
)]
async fn open_ping() -> serde_json::Value {
    serde_json::json!({ "pong": true })
}

fn apikey_config(admin_key: &str, viewer_key: &str) -> AppConfig {
    AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::ApiKey {
            header_name: "x-api-key".to_string(),
            prefix: "sk_".to_string(),
            keys: vec![
                ApiKeySeed {
                    key: admin_key.to_string(),
                    permissions: vec!["admin".to_string()],
                },
                ApiKeySeed {
                    key: viewer_key.to_string(),
                    permissions: vec!["viewer".to_string()],
                },
            ],
        },
        timeout: None,
        ..Default::default()
    }
}

async fn get_with_key(uri: &str, key: Option<&str>) -> axum::http::Response<axum::body::Body> {
    let config = apikey_config("admin-secret-key-1", "viewer-secret-key-2");
    let mut router = build_with_config(&config).unwrap();
    let mut builder = axum::http::Request::builder().uri(uri);
    if let Some(k) = key {
        builder = builder.header("x-api-key", format!("sk_{k}"));
    }
    tower::Service::call(&mut router, builder.body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn matching_role_gets_200() {
    let resp = get_with_key("/api/v1/admin/stats", Some("admin-secret-key-1")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn non_matching_role_gets_403_not_401() {
    let resp = get_with_key("/api/v1/admin/stats", Some("viewer-secret-key-2")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "FORBIDDEN");
    assert!(json["message"].as_str().unwrap().contains("admin"));
}

#[tokio::test]
async fn unauthenticated_gets_401_from_global_middleware() {
    let resp = get_with_key("/api/v1/admin/stats", None).await;
    assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
}

// Route without auth(...) keeps working with any valid key.
#[tokio::test]
async fn endpoint_without_auth_annotation_stays_open_to_any_key() {
    let resp = get_with_key("/api/v1/open/ping", Some("viewer-secret-key-2")).await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}
