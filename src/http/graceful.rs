// Copyright (c) 2026 Kirky.X
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
fn run_stop_hooks() {
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
async fn run_lifecycle_stop_hooks() {
    #[cfg(feature = "lifecycle")]
    crate::lifecycle::run_on_stop().await;
}

/// Pre-serve start hooks: `#[forge(on_start)]` lifecycle hooks.
async fn run_lifecycle_start_hooks() {
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
    // Fan the phase-1 trigger out so both axum's graceful-shutdown future and
    // the drain deadline observe the same signal instant.
    let (trigger_tx, mut trigger_rx) = tokio::sync::watch::channel(false);
    let axum_shutdown = async move {
        shutdown.await;
        let _ = trigger_tx.send(true);
    };

    // run on_start hooks before accepting connections.
    run_lifecycle_start_hooks().await;

    let server = axum::serve(listener, router).with_graceful_shutdown(axum_shutdown);

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
            run_stop_hooks();
            run_lifecycle_stop_hooks().await;
            result
        }
        _ = deadline => {
            // Dropping `server` here aborts the accept loop and any
            // in-flight connection — the forced path of phase 2.
            run_stop_hooks();
            run_lifecycle_stop_hooks().await;
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
