// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T708 e2e: `#[forge(paginate)]` declarative pagination.
//!
//! The generated route accepts `page`/`size` query parameters and wraps the
//! handler's full `Vec<T>` into `{items, total, next}`.

#![cfg(all(feature = "http", feature = "paginate"))]

use sdforge::forge;
use tower::ServiceExt;

#[forge(
    name = "paginate_items",
    version = "v1",
    path = "/items",
    method = "GET",
    paginate
)]
async fn list_items() -> Vec<u32> {
    vec![10, 11, 12, 13, 14]
}

async fn get_json(uri: &str) -> (axum::http::StatusCode, serde_json::Value) {
    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    (status, json)
}

#[tokio::test]
async fn middle_page_slices_with_next() {
    let (status, json) = get_json("/api/v1/items?page=2&size=2").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(json["items"], serde_json::json!([12, 13]));
    assert_eq!(json["total"], 5);
    assert_eq!(json["next"], 3);
}

#[tokio::test]
async fn last_page_has_null_next() {
    let (status, json) = get_json("/api/v1/items?page=3&size=2").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(json["items"], serde_json::json!([14]));
    assert_eq!(json["total"], 5);
    assert_eq!(json["next"], serde_json::Value::Null);
}

#[tokio::test]
async fn defaults_apply_when_params_absent() {
    let (status, json) = get_json("/api/v1/items").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(json["items"], serde_json::json!([10, 11, 12, 13, 14]));
    assert_eq!(json["total"], 5);
    assert_eq!(json["next"], serde_json::Value::Null);
}

#[tokio::test]
async fn non_numeric_params_fall_back_to_defaults() {
    let (status, json) = get_json("/api/v1/items?page=abc&size=xyz").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(json["items"], serde_json::json!([10, 11, 12, 13, 14]));
}

#[tokio::test]
async fn oversized_size_is_clamped() {
    let (status, json) = get_json("/api/v1/items?page=1&size=99999").await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(json["items"].as_array().unwrap().len(), 5, "clamped to 100 (all items fit)");
}
