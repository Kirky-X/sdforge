// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! e2e: ETag / conditional requests through `build_with_config`.
//!
//! Routes are declared via `#[forge]` (inventory) so the `build_with_config`
//! middleware stack applies to them (axum layers only affect routes
//! registered before them).

#![cfg(all(feature = "http", feature = "etag"))]

use axum::body::Body;
use sdforge::forge;
use sdforge::http::build_with_config;
use tower::ServiceExt;

#[forge(
    name = "etag_report",
    version = "v1",
    path = "/report",
    method = "GET",
    description = "Conditional-GET report"
)]
async fn report() -> serde_json::Value {
    serde_json::json!({"rev": 7})
}

#[forge(
    name = "etag_create",
    version = "v1",
    path = "/report",
    method = "POST",
    status = 201,
    description = "Create report"
)]
async fn create_report() -> serde_json::Value {
    serde_json::json!({"ok": true})
}

fn app() -> axum::Router {
    let config = sdforge::config::SdForgeConfig::default();
    build_with_config(&config).unwrap()
}

async fn send(method: &str, uri: &str, if_none_match: Option<&str>) -> axum::http::Response<Body> {
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    if let Some(inm) = if_none_match {
        builder = builder.header(axum::http::header::IF_NONE_MATCH, inm);
    }
    app()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn json_get_gets_etag_then_304_roundtrip() {
    // First request: 200 + ETag + full body.
    let res = send("GET", "/api/v1/report", None).await;
    assert_eq!(res.status(), 200);
    let etag = res
        .headers()
        .get(axum::http::header::ETAG)
        .expect("ETag must be present")
        .to_str()
        .unwrap()
        .to_string();

    // Second request with If-None-Match: 304, no body.
    let res = send("GET", "/api/v1/report", Some(&etag)).await;
    assert_eq!(res.status(), axum::http::StatusCode::NOT_MODIFIED);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body.is_empty());
}

#[tokio::test]
async fn star_and_mismatch_semantics() {
    // `*` matches any current representation.
    let res = send("GET", "/api/v1/report", Some("*")).await;
    assert_eq!(res.status(), axum::http::StatusCode::NOT_MODIFIED);

    // A stale ETag misses: full 200 with body.
    let res = send("GET", "/api/v1/report", Some("\"stale-etag\"")).await;
    assert_eq!(res.status(), 200);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(serde_json::from_slice::<serde_json::Value>(&body).is_ok());
}

#[tokio::test]
async fn post_responses_do_not_get_etag() {
    let res = send("POST", "/api/v1/report", None).await;
    assert_eq!(res.status(), 201);
    assert!(res.headers().get(axum::http::header::ETAG).is_none());
}
