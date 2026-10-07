// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Graceful shutdown.
//!
//! [`serve_with_graceful_shutdown`] implements the production shutdown
//! sequence:
//!
//! 1. **Stop accepting** new connections — triggered by the shutdown future
//!    (typically [`default_shutdown_signal`] wiring SIGTERM/SIGINT),
//! 2. **Drain** in-flight requests up to
//!    [`GracefulShutdownConfig::drain_timeout`] — the serve loop races the
//!    natural drain completion against the deadline; on deadline expiry the
//!    server is force-aborted,
//! 3. **Stop hooks** — trait-kit phased shutdown (`kit` feature, registered
//!    via [`crate::integrations::kit::set_ready_kit`]) and `#[forge(on_stop)]`
//!    lifecycle hooks run after the drain completes.
//!
//! # Example
//!
//! ```ignore
//! let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
//! sdforge::http::serve_with_graceful_shutdown(
//!     router,
//!     listener,
//!     sdforge::http::default_shutdown_signal(),
//!     GracefulShutdownConfig::default(),
//! ).await?;
//! ```

use std::pin::Pin;
use std::time::Duration;

/// Configuration for the graceful shutdown sequence.
#[derive(Debug, Clone)]
pub struct GracefulShutdownConfig {
    /// Maximum time to wait for in-flight requests after the stop trigger.
    /// When the deadline expires the server is force-aborted (phase 3).
    pub drain_timeout: Duration,
}

impl Default for GracefulShutdownConfig {
    fn default() -> Self {
        Self {
            drain_timeout: Duration::from_secs(30),
        }
    }
}

impl GracefulShutdownConfig {
    /// Config with a custom drain timeout.
    pub fn with_drain_timeout(timeout: Duration) -> Self {
        Self {
            drain_timeout: timeout,
        }
    }
}

/// Default process stop trigger: SIGTERM (unix) or Ctrl-C.
///
/// Resolves once on the first received signal. `SIGTERM` is the K8s/Docker
/// stop signal; `SIGINT` covers local Ctrl-C.
pub async fn default_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        // Signal registration happens before the select so a SIGTERM arriving
        // early is buffered rather than taking the default (kill) action.
        let term = signal(SignalKind::terminate()).ok();
        let int = tokio::signal::ctrl_c();
        match term {
            Some(mut term) => {
                tokio::select! {
                    _ = term.recv() => {}
                    _ = int => {}
                }
            }
            None => {
                let _ = int.await;
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Post-drain stop hooks (phase 3): trait-kit phased shutdown.
///
/// `AsyncKit::shutdown_async` is intentionally `!Send`, so it runs on a
/// dedicated current-thread runtime + OS thread, keeping the serve future
/// spawn-safe (Send). `#[forge(on_stop)]` lifecycle hooks hook in
/// here once the `lifecycle` feature lands.
pub(crate) fn run_stop_hooks() {
    #[cfg(feature = "kit")]
    if let Some(kit) = crate::integrations::kit::take_ready_kit() {
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(kit.shutdown_async());
            }
        })
        .join();
    }
}

/// Async stop hooks: `#[forge(on_stop)]` lifecycle hooks.
pub(crate) async fn run_lifecycle_stop_hooks() {
    #[cfg(feature = "lifecycle")]
    crate::lifecycle::run_on_stop().await;
}

/// Pre-serve start hooks: `#[forge(on_start)]` lifecycle hooks.
pub(crate) async fn run_lifecycle_start_hooks() {
    #[cfg(feature = "lifecycle")]
    crate::lifecycle::run_on_start().await;
}

/// Serve `router` on `listener` with the graceful shutdown sequence.
///
/// `shutdown` is the phase-1 stop trigger — [`default_shutdown_signal`] or
/// any custom future (tests inject a channel).
///
/// Returns once the server has stopped: after all in-flight requests drained
/// naturally, or after `config.drain_timeout` expired past the trigger
/// (force-abort). Stop hooks run before returning in both paths.
pub async fn serve_with_graceful_shutdown(
    router: axum::Router,
    listener: tokio::net::TcpListener,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    config: GracefulShutdownConfig,
) -> std::io::Result<()> {
    serve_graceful(router.into_make_service(), listener, shutdown, config, None).await
}

/// Serve `router` on `listener` with the graceful shutdown sequence, exposing
/// the direct TCP peer address as `ConnectInfo<SocketAddr>` on every request.
///
/// Identical to [`serve_with_graceful_shutdown`] except the router is served
/// via `into_make_service_with_connect_info`, so handlers and middleware can
/// extract the unspoofable peer IP (`req.extensions().get::<ConnectInfo<..>>()`).
/// IP-based security layers (rate limiting, auth) should prefer this variant:
/// without `ConnectInfo`, client-IP extraction must fall back to spoofable
/// forwarded headers or a shared `"unknown"` bucket.
pub async fn serve_with_graceful_shutdown_connect_info(
    router: axum::Router,
    listener: tokio::net::TcpListener,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    config: GracefulShutdownConfig,
) -> std::io::Result<()> {
    serve_graceful(
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        listener,
        shutdown,
        config,
        None,
    )
    .await
}

/// Serve `router` on `listener` with the graceful shutdown sequence, running
/// a caller-supplied hook after the drain/stop phase.
///
/// Identical to [`serve_with_graceful_shutdown`], plus `after_drain`: an
/// async hook awaited in both shutdown paths (natural drain and
/// forced-abort) after the built-in stop hooks — kit phased shutdown, then
/// `#[forge(on_stop)]` lifecycle hooks — and before the serve future
/// resolves. Use it for last-mile teardown that must observe a fully
/// drained server (health probes flipped, registry deregistration acked…).
///
/// # Unbounded hook
///
/// The hook is awaited **without any timeout**: if it never resolves, the
/// process never exits. Callers that need a bounded shutdown must wrap the
/// hook themselves, e.g.
/// `Box::pin(tokio::time::timeout(Duration::from_secs(5), hook))` — the
/// library cannot know a sensible deadline for arbitrary teardown work.
pub async fn serve_with_graceful_shutdown_with_hooks(
    router: axum::Router,
    listener: tokio::net::TcpListener,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    config: GracefulShutdownConfig,
    after_drain: Option<Pin<Box<dyn std::future::Future<Output = ()> + Send>>>,
) -> std::io::Result<()> {
    serve_graceful(
        router.into_make_service(),
        listener,
        shutdown,
        config,
        after_drain,
    )
    .await
}

/// 排空后的统一收尾：kit 三阶段关闭 → `#[forge(on_stop)]` 生命周期钩子
/// → 调用方 `after_drain` 钩子。graceful 与 tls 两条 serve 路径共用，
/// 收尾顺序保持单一事实源。
pub(crate) async fn run_stop_phase(after_drain: Option<Pin<Box<dyn Future<Output = ()> + Send>>>) {
    run_stop_hooks();
    run_lifecycle_stop_hooks().await;
    if let Some(hook) = after_drain {
        hook.await;
    }
}

/// Shared graceful-shutdown choreography for both serve variants.
///
/// Fan the phase-1 trigger out so both axum's graceful-shutdown future and
/// the drain deadline observe the same signal instant; run start hooks before
/// accepting connections and stop hooks before returning in either path.
async fn serve_graceful<M, S>(
    make_service: M,
    listener: tokio::net::TcpListener,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    config: GracefulShutdownConfig,
    after_drain: Option<Pin<Box<dyn std::future::Future<Output = ()> + Send>>>,
) -> std::io::Result<()>
where
    // Bounds mirror `axum::serve` (which uses `tower_service::Service`,
    // re-exported as `tower::Service`) so both `IntoMakeService` and
    // `IntoMakeServiceWithConnectInfo` fit.
    M: for<'a> tower::Service<
            axum::serve::IncomingStream<'a, tokio::net::TcpListener>,
            Error = std::convert::Infallible,
            Response = S,
        > + Send
        + 'static,
    for<'a> <M as tower::Service<axum::serve::IncomingStream<'a, tokio::net::TcpListener>>>::Future:
        Send,
    S: tower::Service<
            axum::extract::Request,
            Error = std::convert::Infallible,
            Response = axum::response::Response,
        > + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    // Fan the phase-1 trigger out so both axum's graceful-shutdown future and
    // the drain deadline observe the same signal instant.
    let (trigger_tx, mut trigger_rx) = tokio::sync::watch::channel(false);
    let axum_shutdown = async move {
        shutdown.await;
        let _ = trigger_tx.send(true);
    };

    // run on_start hooks before accepting connections.
    run_lifecycle_start_hooks().await;

    let server = axum::serve(listener, make_service).with_graceful_shutdown(axum_shutdown);

    // Deadline window: opens when the trigger fires, closes after
    // drain_timeout. Winning this race force-aborts the server (phase 2 cap).
    let drain_timeout = config.drain_timeout;
    let deadline = async move {
        loop {
            if *trigger_rx.borrow() {
                break;
            }
            if trigger_rx.changed().await.is_err() {
                // Sender dropped without firing — treat as immediate stop
                // (matches axum's semantics for a dropped shutdown future).
                break;
            }
        }
        tokio::time::sleep(drain_timeout).await;
    };

    tokio::select! {
        result = server => {
            run_stop_phase(after_drain).await;
            result
        }
        _ = deadline => {
            // Dropping `server` here aborts the accept loop and any
            // in-flight connection — the forced path of phase 2.
            run_stop_phase(after_drain).await;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_thirty_second_drain() {
        assert_eq!(
            GracefulShutdownConfig::default().drain_timeout,
            Duration::from_secs(30)
        );
        let custom = GracefulShutdownConfig::with_drain_timeout(Duration::from_millis(250));
        assert_eq!(custom.drain_timeout, Duration::from_millis(250));
    }
}

/// 停机时序的单元级钉住：自然排空 / 在途请求排空 / 超时强制中止 三条路径
/// 及 `after_drain` 钩子在两条路径都被 await。与集成测试（lifecycle_tests、
/// graceful_shutdown_tests）口径一致，但不起真实客户端集群，仅本机回环。
#[cfg(all(test, feature = "http", feature = "tokio"))]
mod shutdown_sequence_tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::routing::get;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    type AfterDrainHook = Arc<
        dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync,
    >;

    /// 返回 (服务地址, server 任务, 停机扳机发送端)。handler 按 sleep_ms 延时并计数。
    async fn spawn_server(
        sleep_ms: u64,
        drain: Duration,
        hits: Arc<AtomicUsize>,
        after_drain: Option<AfterDrainHook>,
    ) -> (
        std::net::SocketAddr,
        tokio::task::JoinHandle<std::io::Result<()>>,
        tokio::sync::oneshot::Sender<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let counter = Arc::clone(&hits);
        let handler = move || {
            let c = Arc::clone(&counter);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                if sleep_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                }
                (StatusCode::OK, "served")
            }
        };
        let router = axum::Router::new().route("/", get(handler));
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let shutdown = async move {
            let _ = rx.await;
        };
        let server = match after_drain {
            Some(hook) => tokio::spawn(serve_with_graceful_shutdown_with_hooks(
                router,
                listener,
                shutdown,
                GracefulShutdownConfig::with_drain_timeout(drain),
                Some(hook()),
            )),
            None => tokio::spawn(serve_with_graceful_shutdown(
                router,
                listener,
                shutdown,
                GracefulShutdownConfig::with_drain_timeout(drain),
            )),
        };
        (addr, server, tx)
    }

    /// 发送一个 HTTP/1.1 请求并读完整个响应（连接关闭为界）。
    async fn http_get(addr: std::net::SocketAddr) -> Result<String, std::io::Error> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(addr).await?;
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        Ok(String::from_utf8_lossy(&buf).into_owned())
    }

    #[tokio::test]
    async fn natural_drain_returns_ok_and_awaits_after_drain_hook() {
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let hook: AfterDrainHook = Arc::new(move || {
            let f = Arc::clone(&flag);
            Box::pin(async move {
                f.store(true, Ordering::SeqCst);
            })
        });
        let hits = Arc::new(AtomicUsize::new(0));
        let (addr, server, tx) =
            spawn_server(0, Duration::from_secs(5), Arc::clone(&hits), Some(hook)).await;

        let body = http_get(addr).await.unwrap();
        assert!(
            body.contains("200 OK") && body.contains("served"),
            "响应: {body}"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1, "请求应真实到达 handler");

        tx.send(()).unwrap();
        server.await.unwrap().unwrap();
        assert!(
            ran.load(Ordering::SeqCst),
            "自然排空路径必须 await 调用方的 after_drain 钩子"
        );
    }

    /// 排空窗口内的在途请求负完成：停机不得掉断已开始的响应。
    #[tokio::test]
    async fn in_flight_request_completes_during_drain() {
        let hits = Arc::new(AtomicUsize::new(0));
        let (addr, server, tx) =
            spawn_server(200, Duration::from_secs(5), Arc::clone(&hits), None).await;
        let client = tokio::spawn(async move { http_get(addr).await });
        tokio::time::sleep(Duration::from_millis(40)).await;
        tx.send(()).unwrap();
        let body = client.await.unwrap().unwrap();
        assert!(
            body.contains("200 OK") && body.contains("served"),
            "在途请求应被排空而非掉断: {body}"
        );
        server.await.unwrap().unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    /// 超过 drain_timeout：强制中止（drop server），serve 仍返 Ok 且不得等完 handler。
    #[tokio::test]
    async fn drain_timeout_forces_abort_and_returns_ok() {
        let hits = Arc::new(AtomicUsize::new(0));
        let (addr, server, tx) =
            spawn_server(3_000, Duration::from_millis(100), Arc::clone(&hits), None).await;
        let client = tokio::spawn(async move { http_get(addr).await });
        tokio::time::sleep(Duration::from_millis(40)).await;
        tx.send(()).unwrap();

        // server 必须在远小于 handler 延时的时间内返回（否则为未强制中止）
        let done = tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .expect("drain 超时后应强制中止，而非等完 3s handler");
        done.unwrap().unwrap();

        let res = tokio::time::timeout(Duration::from_millis(500), client)
            .await
            .map(|joined| joined.unwrap());
        match res {
            Err(_) => {} // 连接被掉：客户端永不读完
            Ok(Ok(body)) => assert!(
                !body.contains("served"),
                "强制中止不得将完整 handler 响应交付客户端: {body}"
            ),
            Ok(Err(_)) => {} // 读失败亦属中止可观测结果
        }
    }

    /// 强制中止路径同样必须 await after_drain 钩子（两条收尾路径一致）。
    #[tokio::test]
    async fn after_drain_hook_runs_on_forced_abort_too() {
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let hits = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let counter = Arc::clone(&hits);
        let slow = move || {
            let c = Arc::clone(&counter);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(3_000)).await;
                (StatusCode::OK, "served")
            }
        };
        let router = axum::Router::new().route("/", get(slow));
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(serve_with_graceful_shutdown_with_hooks(
            router,
            listener,
            async move {
                let _ = rx.await;
            },
            GracefulShutdownConfig::with_drain_timeout(Duration::from_millis(100)),
            Some(Box::pin(async move {
                flag.store(true, Ordering::SeqCst);
            })),
        ));
        let client = tokio::spawn(async move { http_get(addr).await });
        tokio::time::sleep(Duration::from_millis(40)).await;
        tx.send(()).unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(2), client).await;
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .expect("强制中止路径应按时完成收尾")
            .unwrap()
            .unwrap();
        assert!(
            ran.load(Ordering::SeqCst),
            "强制中止路径也必须 await after_drain 钩子"
        );
    }

    /// `ConnectInfo` 变体必须交出真实 TCP 对端地址（限流/鉴权依此取 IP）。
    #[tokio::test]
    async fn connect_info_variant_exposes_real_peer_addr() {
        use axum::extract::ConnectInfo;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = axum::Router::new().route(
            "/",
            get(
                |ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>| async move {
                    format!("{}", peer.port())
                },
            ),
        );
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(serve_with_graceful_shutdown_connect_info(
            router,
            listener,
            async move {
                let _ = rx.await;
            },
            GracefulShutdownConfig::with_drain_timeout(Duration::from_secs(5)),
        ));

        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let client_port = stream.local_addr().unwrap().port();
        let mut stream = stream;
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let body = String::from_utf8_lossy(&buf).into_owned();
        assert!(
            body.contains(&client_port.to_string()),
            "处理器应看到客户端真实临时端口（期望 {client_port}，响应: {body}）"
        );
        tx.send(()).unwrap();
        server.await.unwrap().unwrap();
    }
}
