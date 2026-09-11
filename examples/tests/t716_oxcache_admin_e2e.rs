// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T716 e2e: oxcache management endpoints over `#[forge]` + BackendRegistry.

#![cfg(feature = "oxcache_admin_example")]

use tower::ServiceExt;

#[sdforge::forge(
    name = "t716_kinds",
    version = "v1",
    path = "/kinds",
    method = "GET",
    no_prefix = true,
    description = "list kinds"
)]
async fn kinds() -> Result<serde_json::Value, sdforge::core::ApiError> {
    Ok(serde_json::json!({
        "kinds": oxcache::backend::BackendRegistry::global().registered(),
    }))
}

#[tokio::test]
async fn registry_lists_and_builds_builtin_backends() {
    use oxcache::backend::BackendRegistry;
    let registry = BackendRegistry::global();
    let kinds = registry.registered();
    assert!(
        kinds.iter().any(|k| k == "moka" || k == "memory"),
        "builtin memory backend must be registered: {kinds:?}"
    );

    // Build through the registry and run a round-trip.
    let spec = oxcache::backend::BackendSpec::new("memory");
    let backend = registry.build(&spec).await.expect("memory backend builds");
    use std::sync::Arc;
    backend
        .set(Arc::from("e2e:key"), Arc::new(b"value".to_vec()), None)
        .await
        .expect("set");
    let v = backend.get("e2e:key").await.expect("get");
    assert_eq!(v.as_deref(), Some(b"value".as_slice()));
}

#[tokio::test]
async fn forge_route_serves_kinds_over_http() {
    let router = sdforge::http::build();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/kinds")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["kinds"].as_array().is_some(), "kinds array: {json}");
}
