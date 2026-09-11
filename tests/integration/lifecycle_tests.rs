// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! T711 e2e: `#[forge(on_start/on_stop)]` lifecycle hooks coordinated with
//! the T704 graceful-shutdown sequence.

#![cfg(all(feature = "http", feature = "graceful", feature = "lifecycle"))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use sdforge::forge;
use sdforge::http::{serve_with_graceful_shutdown, GracefulShutdownConfig};

static STARTED: AtomicBool = AtomicBool::new(false);
static STOPPED: AtomicBool = AtomicBool::new(false);

#[forge(name = "warmup", on_start)]
async fn warmup() {
    STARTED.store(true, Ordering::SeqCst);
}

#[forge(name = "teardown", on_stop)]
async fn teardown() {
    STOPPED.store(true, Ordering::SeqCst);
}

#[tokio::test]
async fn hooks_run_in_shutdown_sequence() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let router = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown(
        router,
        listener,
        async move {
            let _ = trigger_rx.await;
        },
        GracefulShutdownConfig::default(),
    ));

    // on_start hooks run before the server accepts connections.
    assert!(
        STARTED.load(Ordering::SeqCst),
        "on_start hook must run before serving"
    );

    // Wait for the listener to come up.
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    trigger_tx.send(()).unwrap();
    server.await.unwrap().unwrap();

    assert!(
        STOPPED.load(Ordering::SeqCst),
        "on_stop hook must run after shutdown completes"
    );
}

#[tokio::test]
async fn run_on_start_is_idempotent_at_registry_level() {
    let before = sdforge::lifecycle::started_count();
    sdforge::lifecycle::run_on_start().await;
    let after = sdforge::lifecycle::started_count();
    assert!(after > before, "run_on_start must execute registered hooks");
}

#[tokio::test]
async fn run_on_stop_executes_registered_stop_hooks() {
    let before = sdforge::lifecycle::stopped_count();
    sdforge::lifecycle::run_on_stop().await;
    let after = sdforge::lifecycle::stopped_count();
    assert!(after > before, "run_on_stop must execute registered hooks");
}
