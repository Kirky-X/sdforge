// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! collision semantics: a user route that already claims `/healthz`
//! takes precedence — `build_with_config` skips the built-in probe instead
//! of panicking on a duplicate route.
//!
//! Isolated in its own test binary because `inventory::submit!` registers
//! process-wide and would leak the shadow route into sibling tests.

#![cfg(feature = "health")]

use axum::body::Body;
use sdforge::config::{AppConfig, AuthConfig, ServerConfig};
use sdforge::http::build_with_config;

fn shadow_route() -> sdforge::http::HttpRoute {
    let router = sdforge::axum::routing::MethodRouter::new()
        .get(|| async { (axum::http::StatusCode::IM_A_TEAPOT, "user-owned healthz") });
    sdforge::http::HttpRoute::new(
        "/healthz".to_string(),
        router,
        sdforge::core::ApiMetadata::new(
            "shadow".to_string(),
            "v1".to_string(),
            "shadow".to_string(),
            None,
            false,
        ),
        None,
    )
}

fn shadow_metadata() -> sdforge::core::ApiMetadata {
    sdforge::core::ApiMetadata::new(
        "shadow".to_string(),
        "v1".to_string(),
        "shadow".to_string(),
        None,
        false,
    )
}

sdforge::inventory::submit!(sdforge::http::RouteRegistration::new(
    "shadow",
    "v1",
    shadow_route,
    shadow_metadata,
));

#[tokio::test]
async fn user_route_takes_precedence_over_builtin_healthz() {
    let config = AppConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::None,
        timeout: None,
        ..Default::default()
    };
    let router = build_with_config(&config).unwrap(); // must not panic
    use tower::ServiceExt;
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // The user-owned handler answers, not the built-in probe.
    assert_eq!(resp.status(), axum::http::StatusCode::IM_A_TEAPOT);
}
