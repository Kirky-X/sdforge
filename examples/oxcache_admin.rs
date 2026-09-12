// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Example: oxcache management endpoints via `BackendRegistry`.
//!
//! `#[forge]` declares the management plane once; sdforge surfaces it over
//! HTTP (and any other protocol). The oxcache global registry builds backends
//! by `kind` — exactly the management surface the deleted `cli` feature used
//! to own, now hosted by sdforge.
//!
//! Run: `cargo run -p sdforge-examples --example oxcache_admin`

use sdforge::forge;

/// List backend kinds registered in the oxcache global registry.
#[forge(
    name = "oxcache_kinds",
    version = "v1",
    path = "/admin/oxcache/kinds",
    method = "GET",
    no_prefix = true,
    description = "List registered oxcache backend kinds"
)]
async fn oxcache_kinds() -> Result<serde_json::Value, sdforge::core::ApiError> {
    Ok(serde_json::json!({
        "kinds": oxcache::backend::BackendRegistry::global().registered(),
    }))
}

/// Build a backend by kind and smoke-test a round-trip set/get.
#[forge(
    name = "oxcache_build",
    version = "v1",
    path = "/admin/oxcache/build/:kind",
    method = "GET",
    no_prefix = true,
    description = "Build an oxcache backend by kind and run a round-trip"
)]
async fn oxcache_build(kind: String) -> Result<serde_json::Value, sdforge::core::ApiError> {
    use oxcache::backend::BackendRegistry;
    let spec = oxcache::backend::BackendSpec::new(kind.clone());
    let backend = BackendRegistry::global()
        .build(&spec)
        .await
        .map_err(|e| sdforge::core::ApiError::internal_error(e.to_string(), "oxcache.build"))?;
    use std::sync::Arc;
    backend
        .set(Arc::from("admin:probe"), Arc::new(b"pong".to_vec()), None)
        .await
        .map_err(|e| sdforge::core::ApiError::internal_error(e.to_string(), "oxcache.set"))?;
    let value = backend
        .get("admin:probe")
        .await
        .map_err(|e| sdforge::core::ApiError::internal_error(e.to_string(), "oxcache.get"))?;
    Ok(serde_json::json!({
        "kind": kind,
        "built": true,
        "round_trip": value.as_deref() == Some(b"pong".as_slice()),
    }))
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let router = sdforge::http::build();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:8092").await.unwrap();
        eprintln!("oxcache admin example on http://127.0.0.1:8092");
        sdforge::axum::serve(listener, router).await.unwrap();
    });
}
