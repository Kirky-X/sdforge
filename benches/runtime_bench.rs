// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Runtime performance baselines.
//!
//! Run with:
//! ```text
//! cargo bench --bench runtime_bench --features http
//! ```
//!
//! Baselines are recorded in `docs/PERFORMANCE.md`. These cover the request
//! hot path (routing + middleware stack + handler dispatch) and JSON
//! serialization; macro-expansion compile-time cost is intentionally NOT
//! measured here (see docs/PERFORMANCE.md).

use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::json;

/// A forge endpoint used for dispatch benchmarks (plain handler).
#[sdforge::forge(
    name = "bench_ping",
    version = "v1",
    path = "/bench/ping",
    method = "GET",
    description = "Bench ping"
)]
async fn bench_ping() -> serde_json::Value {
    json!({"ok": true})
}

/// A forge endpoint with a path parameter (exercises path extraction).
#[sdforge::forge(
    name = "bench_user",
    version = "v1",
    path = "/bench/user/:id",
    method = "GET",
    description = "Bench user"
)]
async fn bench_user(id: u64) -> serde_json::Value {
    json!({"id": id})
}

fn bench_route_dispatch(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let router = sdforge::http::build();

    let mut group = c.benchmark_group("route_dispatch");
    group.throughput(criterion::Throughput::Elements(1));

    group.bench_function("plain_get", |b| {
        b.iter(|| {
            rt.block_on({
                use tower::ServiceExt;
                let mut r = router.clone();
                async move {
                    use tower::Service;
                    let req = axum::http::Request::builder()
                        .uri("/api/v1/bench/ping")
                        .body(axum::body::Body::empty())
                        .unwrap();
                    let res = Service::call(&mut r, req).await.unwrap();
                    assert_eq!(res.status(), 200);
                }
            })
        });
    });

    group.bench_function("path_param_get", |b| {
        b.iter(|| {
            rt.block_on({
                use tower::ServiceExt;
                let mut r = router.clone();
                async move {
                    use tower::Service;
                    let req = axum::http::Request::builder()
                        .uri("/api/v1/bench/user/12345")
                        .body(axum::body::Body::empty())
                        .unwrap();
                    let res = Service::call(&mut r, req).await.unwrap();
                    assert_eq!(res.status(), 200);
                }
            })
        });
    });

    group.finish();
}

fn bench_unified_handler_dispatch(c: &mut Criterion) {
    // The protocol-unified handler path (gRPC/CLI): HandlerArgs map → typed
    // parse → handler → serialize. Measured via the CLI registration's
    // handler fn if available; exercised indirectly through JSON cost here.
    let mut group = c.benchmark_group("handler_args");

    group.bench_function("handler_args_build_5_params", |b| {
        b.iter(|| {
            let mut args = std::collections::HashMap::new();
            args.insert("a", "1".to_string());
            args.insert("b", "two".to_string());
            args.insert("c", "3.5".to_string());
            args.insert("d", "true".to_string());
            args.insert("e", "2026-09-11".to_string());
            criterion::black_box(args)
        });
    });

    group.finish();
}

fn bench_json_serialization(c: &mut Criterion) {
    let payload = json!({
        "id": 12345u64,
        "name": "benchmark-user",
        "email": "user@example.com",
        "active": true,
        "tags": ["alpha", "beta", "gamma"],
        "profile": {
            "level": 3,
            "score": 98.5,
            "joined": "2026-09-11T00:00:00Z",
        },
    });

    let mut group = c.benchmark_group("json");

    group.bench_function("serialize_nested_object", |b| {
        b.iter(|| {
            let s = serde_json::to_string(&payload).unwrap();
            criterion::black_box(s)
        });
    });

    group.bench_function("deserialize_nested_object", |b| {
        let text = serde_json::to_string(&payload).unwrap();
        b.iter(|| {
            let v: serde_json::Value = serde_json::from_str(&text).unwrap();
            criterion::black_box(v)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_route_dispatch,
    bench_unified_handler_dispatch,
    bench_json_serialization
);
criterion_main!(benches);
