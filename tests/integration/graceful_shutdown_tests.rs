// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! integration: graceful shutdown sequence.
//!
//! Verifies the three phases of `serve_with_graceful_shutdown`:
//! 1. stop trigger → server stops accepting new connections,
//! 2. in-flight requests complete before the serve future resolves
//!    (drain), with a forced path when `drain_timeout` expires,
//! 3. registered kit teardown (`shutdown_async`) runs before the serve
//!    future returns.

#![cfg(feature = "graceful")]

use std::time::Duration;

use sdforge::http::{GracefulShutdownConfig, serve_with_graceful_shutdown};

/// Bind an ephemeral listener for tests.
async fn test_listener() -> tokio::net::TcpListener {
    tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap()
}

/// Wait until the server accepts TCP connections (poll-connect).
async fn wait_until_up(addr: std::net::SocketAddr) {
    for _ in 0..200 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("server never came up at {addr}");
}

async fn get_status(url: String) -> Result<u16, reqwest::Error> {
    reqwest::get(url).await?.error_for_status().map(|_| 200)
}

#[tokio::test]
async fn inflight_request_completes_before_shutdown() {
    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();

    // Slow route: sleeps 300ms then answers.
    let router = axum::Router::new().route(
        "/slow",
        axum::routing::get(|| async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            "done"
        }),
    );

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown(
        router,
        listener,
        async move {
            let _ = trigger_rx.await;
        },
        GracefulShutdownConfig::with_drain_timeout(Duration::from_secs(5)),
    ));

    wait_until_up(addr).await;

    // Fire the in-flight slow request, then trigger shutdown while it runs.
    let in_flight = tokio::spawn(get_status(format!("http://{addr}/slow")));
    tokio::time::sleep(Duration::from_millis(100)).await;
    trigger_tx.send(()).unwrap();

    // Phase 2: the in-flight request still completes successfully.
    let status = in_flight
        .await
        .unwrap()
        .expect("in-flight request must be drained, not dropped");
    assert_eq!(status, 200);

    // The serve future resolves after draining.
    let result = tokio::time::timeout(Duration::from_secs(3), server).await;
    assert!(
        result.is_ok(),
        "serve future must resolve after in-flight drain"
    );
    result.unwrap().unwrap().expect("graceful path returns Ok");
}

#[tokio::test]
async fn drain_timeout_forces_shutdown() {
    let listener = test_listener().await;
    let addr = listener.local_addr().unwrap();

    // Route that outlives the drain timeout (1.5s hold vs 200ms budget).
    let router = axum::Router::new().route(
        "/stuck",
        axum::routing::get(|| async {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            "late"
        }),
    );

    let (trigger_tx, trigger_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve_with_graceful_shutdown(
        router,
        listener,
        async move {
            let _ = trigger_rx.await;
        },
        GracefulShutdownConfig::with_drain_timeout(Duration::from_millis(200)),
    ));

    wait_until_up(addr).await;
    let in_flight = tokio::spawn(get_status(format!("http://{addr}/stuck")));
    tokio::time::sleep(Duration::from_millis(50)).await;
    let t0 = std::time::Instant::now();
    trigger_tx.send(()).unwrap();

    // Forced path: serve resolves quickly (~drain timeout), not waiting the
    // full 1.5s handler hold.
    let result = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .expect("serve must resolve after forced drain timeout")
        .unwrap();
    assert!(result.is_ok());
    let elapsed = t0.elapsed();
    assert!(
        elapsed < Duration::from_millis(1200),
        "forced shutdown must not wait for the stuck handler; took {elapsed:?}"
    );

    // The stuck request's connection is severed — whatever the client sees,
    // the server did not linger for the handler.
    let _ = in_flight.await;
}

#[cfg(feature = "kit")]
mod kit_teardown {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use trait_kit::core::ModuleMeta;
    use trait_kit::{AsyncAutoBuilder, AsyncKit};

    /// Module whose capability is a shutdown flag; `on_shutdown` flips it.
    struct FlagModule;

    impl ModuleMeta for FlagModule {
        const NAME: &'static str = "graceful-flag";
    }

    impl AsyncAutoBuilder for FlagModule {
        type Capability = Arc<AtomicBool>;
        type Error = trait_kit::TraitKitError;

        fn build<'a>(
            _kit: &'a AsyncKit,
        ) -> Pin<Box<dyn Future<Output = Result<Self::Capability, Self::Error>> + Send + 'a>>
        {
            Box::pin(async move { Ok(Arc::new(AtomicBool::new(false))) })
        }
    }

    impl trait_kit::AsyncLifecycle for FlagModule {
        fn on_shutdown<'a>(
            cap: &'a Arc<AtomicBool>,
        ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            Box::pin(async move {
                cap.store(true, Ordering::SeqCst);
            })
        }
    }

    #[tokio::test]
    async fn registered_kit_shuts_down_in_phase_three() {
        let listener = test_listener().await;
        let addr = listener.local_addr().unwrap();

        let mut kit = AsyncKit::new();
        kit.register::<FlagModule>().expect("register flag module");
        kit.register_lifecycle::<FlagModule>();
        let ready = Arc::new(kit.build().await.expect("kit builds"));
        let cap: Arc<AtomicBool> = ready.require::<FlagModule>().expect("capability");
        sdforge::integrations::kit::set_ready_kit(ready);

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
        wait_until_up(addr).await;
        trigger_tx.send(()).unwrap();

        server.await.unwrap().unwrap();

        assert!(
            cap.load(Ordering::SeqCst),
            "kit shutdown hook must run during graceful stop phase 3"
        );
    }
}
