// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! collision semantics: a user route that already claims `/healthz`
//! takes precedence — `build_with_config_ext` skips the built-in probe instead
//! of panicking on a duplicate route.
//!
//! The precedence check only scans inventory registrations
//! (`http::route_path_taken`), so a path claimed by the caller's `base`
//! router is invisible to it: the built-in probe is still mounted and axum
//! rejects the duplicate route.
//!
//! Isolated in its own test binary because `inventory::submit!` registers
//! process-wide and would leak the shadow route into sibling tests.

#![cfg(feature = "health")]

use axum::body::Body;
use sdforge::config::{AuthConfig, SdForgeConfig, ServerConfig};
use sdforge::http::build_with_config_ext;

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

fn test_config() -> SdForgeConfig {
    SdForgeConfig {
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
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

#[tokio::test]
async fn user_route_takes_precedence_over_builtin_healthz() {
    let config = test_config();
    let router = build_with_config_ext(axum::Router::new(), &config).unwrap(); // must not panic
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

/// `route_path_taken` only scans inventory registrations, so a path claimed
/// by the caller's `base` router is not detected: the built-in probe is still
/// mounted on the merged router and axum rejects the duplicate route.
#[tokio::test]
async fn base_claiming_readyz_panics_with_overlapping_method_route() {
    let config = test_config();
    let base = axum::Router::new().route(
        "/readyz",
        axum::routing::get(|| async { (axum::http::StatusCode::IM_A_TEAPOT, "user-owned readyz") }),
    );
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        build_with_config_ext(base, &config)
    }));
    let payload = match outcome {
        Ok(_) => panic!("base-claimed /readyz must not silently win over the built-in probe"),
        Err(payload) => payload,
    };
    let msg = panic_message(payload.as_ref());
    assert!(
        msg.contains("Overlapping method route"),
        "unexpected panic message: {msg}"
    );
}
