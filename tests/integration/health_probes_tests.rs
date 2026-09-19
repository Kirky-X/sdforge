// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! e2e: built-in health probes.
//!
//! `build_with_config` auto-mounts `/healthz` and `/readyz` **after** the
//! auth middleware, so probes bypass authentication while normal routes
//! still require credentials. Readiness folds in registered readiness
//! checks and (with the `kit` feature) the trait-kit health aggregate.

#![cfg(feature = "health")]

use axum::body::Body;
use sdforge::config::{AuthConfig, SdForgeConfig, ServerConfig};
use sdforge::http::build_with_config;

// A user route registered through inventory, used to prove that global auth
// still applies to ordinary endpoints while probes bypass it.
fn user_route() -> sdforge::http::HttpRoute {
    let router = sdforge::axum::routing::MethodRouter::new().get(|| async {
        (
            axum::http::StatusCode::OK,
            sdforge::axum::extract::Json(serde_json::json!({"ok": true})),
        )
    });
    sdforge::http::HttpRoute::new(
        "/api/v1/protected".to_string(),
        router,
        sdforge::core::ApiMetadata::new(
            "protected".to_string(),
            "v1".to_string(),
            "protected endpoint".to_string(),
            None,
            false,
        ),
        None,
    )
}

fn user_metadata() -> sdforge::core::ApiMetadata {
    sdforge::core::ApiMetadata::new(
        "protected".to_string(),
        "v1".to_string(),
        "protected endpoint".to_string(),
        None,
        false,
    )
}

sdforge::inventory::submit!(sdforge::http::RouteRegistration::new(
    "protected",
    "v1",
    user_route,
    user_metadata,
));

fn jwt_config() -> SdForgeConfig {
    SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            ..Default::default()
        },
        authentication: AuthConfig::Jwt {
            secret: "Test-Secret-That-Is-Long-Enough-For-Bearer-Auth-0123456789".to_string(),
        },
        timeout: None,
        ..Default::default()
    }
}

async fn get(router: axum::Router, uri: &str) -> axum::http::Response<Body> {
    use tower::ServiceExt;
    router
        .oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn body_json(resp: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}

#[tokio::test]
#[serial_test::serial]
async fn healthz_bypasses_auth() {
    sdforge::health::clear_readiness_checks();
    sdforge::health::clear_health_source();
    let router = build_with_config(&jwt_config()).unwrap();

    // Probe: 200 without any Authorization header.
    let resp = get(router.clone(), "/healthz").await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "healthy");

    // Contrast: the ordinary route is blocked by global auth (401).
    let resp = get(router, "/api/v1/protected").await;
    assert_eq!(resp.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial_test::serial]
async fn readyz_bypasses_auth_and_defaults_ready() {
    sdforge::health::clear_readiness_checks();
    sdforge::health::clear_health_source();
    let router = build_with_config(&jwt_config()).unwrap();
    let resp = get(router, "/readyz").await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "ready");
}

#[tokio::test]
#[serial_test::serial]
async fn readyz_reports_failing_check_as_503() {
    sdforge::health::clear_readiness_checks();
    sdforge::health::clear_health_source();
    sdforge::health::register_readiness_check_fn("cache", || {
        sdforge::health::CheckOutcome::unhealthy("cache", "connection refused")
    });
    let router = build_with_config(&jwt_config()).unwrap();
    let resp = get(router, "/readyz").await;
    assert_eq!(resp.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "unavailable");
    assert_eq!(json["checks"][0]["name"], "cache");
    sdforge::health::clear_readiness_checks();
}

#[tokio::test]
#[serial_test::serial]
async fn readyz_passes_when_all_checks_healthy() {
    sdforge::health::clear_readiness_checks();
    sdforge::health::clear_health_source();
    sdforge::health::register_readiness_check_fn("cache", || {
        sdforge::health::CheckOutcome::healthy("cache")
    });
    let router = build_with_config(&jwt_config()).unwrap();
    let resp = get(router, "/readyz").await;
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    sdforge::health::clear_readiness_checks();
}

// =============================================================================
// kit data source (kit + health features)
// =============================================================================
#[cfg(feature = "kit")]
mod kit_source_e2e {
    use super::{body_json, get, jwt_config};
    use sdforge::http::build_with_config;
    use std::sync::Arc;
    use trait_kit::AsyncKit;

    #[tokio::test]
    #[serial_test::serial]
    async fn readyz_uses_kit_health_report() {
        sdforge::health::clear_readiness_checks();
        sdforge::health::clear_health_source();

        let kit = AsyncKit::new();
        let ready = Arc::new(kit.build().await.expect("kit builds"));
        sdforge::health::kit_source::register_kit_health_source(ready);

        let router = build_with_config(&jwt_config()).unwrap();
        let resp = get(router, "/readyz").await;
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["checks"][0]["name"], "kit");
        assert_eq!(json["checks"][0]["details"]["status"], "healthy");
        sdforge::health::clear_health_source();
    }
}
