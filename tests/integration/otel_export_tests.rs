// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T714 e2e: OTLP/HTTP export to an in-process mock collector.
//!
//! Spans recorded by the request-span middleware (plus a manual protocol
//! span) are flushed as OTLP/HTTP JSON to a local axum collector; a metrics
//! snapshot is exported to `/v1/metrics`.

#![cfg(all(feature = "http", feature = "otel", feature = "metrics"))]

use std::sync::{Arc, Mutex};

use sdforge::forge;

// The span-producing route (registered via inventory so build_with_config
// middleware applies).
#[forge(
    name = "otel_trace_me",
    version = "v1",
    path = "/trace-me",
    method = "GET",
    description = "Span-producing endpoint"
)]
async fn trace_me() -> serde_json::Value {
    serde_json::json!({"traced": true})
}

// =============================================================================
// Mock collector: captures OTLP POST bodies.
// =============================================================================

#[derive(Default)]
struct Collector {
    traces: Mutex<Vec<serde_json::Value>>,
    metrics: Mutex<Vec<serde_json::Value>>,
}

fn spawn_collector() -> (std::net::SocketAddr, Arc<Collector>) {
    let collector = Arc::new(Collector::default());
    let traces_state = collector.clone();
    let metrics_state = collector.clone();

    let app = axum::Router::new()
        .route(
            "/v1/traces",
            axum::routing::post(move |body: String| async move {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    traces_state.traces.lock().unwrap().push(v);
                }
                axum::http::StatusCode::OK
            }),
        )
        .route(
            "/v1/metrics",
            axum::routing::post(move |body: String| async move {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    metrics_state.metrics.lock().unwrap().push(v);
                }
                axum::http::StatusCode::OK
            }),
        );

    // Run the collector on a dedicated OS thread (own current-thread runtime)
    // so the exporter's blocking TcpStream and the test runtime stay decoupled.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let _ = tx.send(addr);
            axum::serve(listener, app).await.unwrap();
        });
    });
    let addr = rx.recv().unwrap();
    // Wait until the collector accepts connections.
    for _ in 0..100 {
        if std::net::TcpStream::connect(addr).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    (addr, collector)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn spans_and_metrics_export_to_mock_collector() {
    use tower::ServiceExt;

    let (addr, collector) = spawn_collector();
    let config = sdforge::otel::OtelConfig {
        endpoint: format!("http://{addr}"),
        service_name: "sdforge-otel-e2e".to_string(),
    };

    // Generate a request span through the real middleware stack (the
    // #[forge] route above is already registered via inventory).
    let router = sdforge::http::build_with_config(&sdforge::config::AppConfig::default()).unwrap();
    let resp = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/trace-me")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Manual protocol span (documents the non-HTTP integration point).
    let span = sdforge::otel::start_span("grpc.dispatch");
    let span = sdforge::otel::with_attr(span, "rpc.system", serde_json::json!("grpc"));
    sdforge::otel::finish_span(span);

    // Flush synchronously (the periodic task path uses the same function).
    let status = sdforge::otel::flush_spans(&config).expect("span export succeeds");
    assert_eq!(status, 200);

    let traces = collector.traces.lock().unwrap();
    assert!(!traces.is_empty(), "collector must receive span exports");
    let payload = &traces[0];
    let resource = &payload["resourceSpans"][0];
    assert_eq!(
        resource["resource"]["attributes"][0]["value"]["stringValue"],
        "sdforge-otel-e2e"
    );
    let spans = resource["scopeSpans"][0]["spans"].as_array().unwrap();
    assert!(
        spans.iter().any(|s| s["name"] == "http.request"),
        "request span must be exported; got names: {:?}",
        spans.iter().map(|s| s["name"].clone()).collect::<Vec<_>>()
    );
    assert!(
        spans.iter().any(|s| s["name"] == "grpc.dispatch"),
        "manual protocol span must be exported"
    );

    // Metrics snapshot export.
    sdforge::metrics::global_registry().reset();
    let status = sdforge::otel::flush_metrics(&config).expect("metrics export succeeds");
    assert_eq!(status, 200);
    let metrics = collector.metrics.lock().unwrap();
    assert!(!metrics.is_empty(), "collector must receive metric exports");
    assert_eq!(
        metrics[0]["resourceMetrics"][0]["scopeMetrics"][0]["metrics"][0]["name"],
        "sdforge_http_requests_total"
    );
}
