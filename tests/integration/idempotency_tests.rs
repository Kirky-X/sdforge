// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! e2e: `Idempotency-Key` replay protection (feature = `idempotency`).
//!
//! 同 key 两次 POST：第二次重放缓存响应（`Idempotency-Replayed: true`）且
//! handler 只执行一次；并发在途 409；无 key 请求零影响（T021）。

#![cfg(all(feature = "http", feature = "idempotency"))]

use axum::body::Body;
use sdforge::config::{IdempotencyConfig, SdForgeConfig, ServerConfig};
use sdforge::forge;
use sdforge::http::build_with_config;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tower::ServiceExt;

/// 计数 handler —— 验证重放不重复执行业务逻辑。
static EXEC_COUNT: AtomicU32 = AtomicU32::new(0);

#[forge(
    name = "idem_payment",
    version = "v1",
    path = "/payments",
    method = "POST"
)]
async fn pay(amount: u64) -> Result<serde_json::Value, sdforge::core::ApiError> {
    EXEC_COUNT.fetch_add(1, Ordering::SeqCst);
    Ok(serde_json::json!({ "paid": amount, "id": "tx-1" }))
}

fn idem_config() -> SdForgeConfig {
    SdForgeConfig {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout_secs: 30,
            cors: None,
            idempotency: IdempotencyConfig {
                enabled: true,
                ttl_secs: 3600,
                inflight_ttl_secs: 30,
                max_response_bytes: 1024 * 1024,
                store: None,
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

async fn post(
    router: axum::Router,
    uri: &str,
    key: Option<&str>,
    body: &str,
) -> axum::http::Response<Body> {
    let mut builder = axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(k) = key {
        builder = builder.header("idempotency-key", k);
    }
    router
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
#[serial_test::serial]
async fn duplicate_key_replays_cached_response() {
    EXEC_COUNT.store(0, Ordering::SeqCst);
    let router = build_with_config(&idem_config()).unwrap();

    let first = post(router.clone(), "/api/v1/payments", Some("dup-1"), "100").await;
    assert_eq!(first.status(), 200);
    let body = axum::body::to_bytes(first.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["paid"], 100);

    let second = post(router.clone(), "/api/v1/payments", Some("dup-1"), "100").await;
    assert_eq!(second.status(), 200);
    assert_eq!(
        second.headers().get("idempotency-replayed").unwrap(),
        "true",
        "重放响应必须带 Idempotency-Replayed: true"
    );
    let body2 = axum::body::to_bytes(second.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(body, body2, "重放体必须与首次一致");

    assert_eq!(EXEC_COUNT.load(Ordering::SeqCst), 1, "handler 只执行一次");
}

#[tokio::test]
#[serial_test::serial]
async fn request_without_key_passes_through_every_time() {
    EXEC_COUNT.store(0, Ordering::SeqCst);
    let router = build_with_config(&idem_config()).unwrap();
    for _ in 0..2 {
        let resp = post(router.clone(), "/api/v1/payments", None, "7").await;
        assert_eq!(resp.status(), 200);
        assert!(resp.headers().get("idempotency-replayed").is_none());
    }
    assert_eq!(EXEC_COUNT.load(Ordering::SeqCst), 2, "无 key 不参与幂等");
}

#[tokio::test]
#[serial_test::serial]
async fn disabled_config_leaves_behavior_unchanged() {
    EXEC_COUNT.store(0, Ordering::SeqCst);
    let mut cfg = idem_config();
    cfg.server.idempotency.enabled = false;
    let router = build_with_config(&cfg).unwrap();
    for _ in 0..2 {
        let resp = post(router.clone(), "/api/v1/payments", Some("dup-2"), "5").await;
        assert_eq!(resp.status(), 200);
    }
    assert_eq!(EXEC_COUNT.load(Ordering::SeqCst), 2, "disabled 时零防护");
}

#[forge(name = "idem_blob", version = "v1", path = "/blob", method = "POST")]
async fn blob() -> Result<serde_json::Value, sdforge::core::ApiError> {
    // ~300 字节响应体，配合 max_response_bytes=64 构造超限场景。
    Ok(serde_json::json!({ "payload": "x".repeat(300) }))
}

#[tokio::test]
#[serial_test::serial]
async fn oversized_response_passes_through_intact() {
    // T002: 超过 max_response_bytes 的成功响应必须原样透传（不缓存、不 413），
    // handler 副作用已提交 —— 413 会诱导客户端重试造成重复副作用。
    let mut cfg = idem_config();
    cfg.server.idempotency.enabled = true;
    cfg.server.idempotency.max_response_bytes = 64; // 故意压到极小
    let router = build_with_config(&cfg).unwrap();

    let resp = post(router.clone(), "/api/v1/blob", Some("over-1"), "{}").await;
    assert_eq!(resp.status(), 200, "超限成功响应必须原样返回");
    assert_eq!(
        resp.headers()
            .get("idempotency-skipped")
            .map(|v| v.to_str().unwrap()),
        Some("oversized")
    );
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        parsed["payload"].as_str().unwrap().len(),
        300,
        "响应体必须完整"
    );

    // claim 已 abort：同 key 重试会再次执行（而不是拿到缓存/409）。
    let retry = post(router, "/api/v1/blob", Some("over-1"), "{}").await;
    assert_eq!(retry.status(), 200);
    assert!(retry.headers().get("idempotency-replayed").is_none());
}

#[tokio::test]
#[serial_test::serial]
async fn http_in_flight_returns_409_with_header() {
    // T032: HTTP 层在途并发 —— 注入 store 并预占 InFlight claim，
    // 同 key POST 得到 409 + Idempotency-Replayed: in-flight。
    let store = Arc::new(sdforge::cache::IdempotencyStore::new());
    assert_eq!(
        store.begin("/api/v1/payments", "inflight-http", 30),
        sdforge::cache::IdempotencyOutcome::Execute
    );
    let mut cfg = idem_config();
    cfg.server.idempotency.store = Some(store);
    let router = build_with_config(&cfg).unwrap();

    let resp = post(router, "/api/v1/payments", Some("inflight-http"), "1").await;
    assert_eq!(resp.status(), 409);
    assert_eq!(
        resp.headers().get("idempotency-replayed").unwrap(),
        "in-flight"
    );
}

#[tokio::test]
async fn concurrent_in_flight_second_request_gets_conflict() {
    // 直接驱动 IdempotencyStore 语义（路由级并发双执行由 store.begin 的
    // InFlight 态保证；HTTP 层映射为 409 CONFLICT）。
    let store = Arc::new(sdforge::cache::IdempotencyStore::new());
    assert_eq!(
        store.begin("/payments", "conc", 30),
        sdforge::cache::IdempotencyOutcome::Execute
    );
    assert_eq!(
        store.begin("/payments", "conc", 30),
        sdforge::cache::IdempotencyOutcome::InFlight
    );
}
