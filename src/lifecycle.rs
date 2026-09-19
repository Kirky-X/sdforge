// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! Lifecycle hooks.
//!
//! `#[forge(on_start)]` / `#[forge(on_stop)]` mark zero-parameter async fns
//! as process lifecycle hooks. `serve_with_graceful_shutdown` runs
//! `on_start` hooks before binding and `on_stop` hooks after the drain
//! completes — the final phase of the shutdown sequence.
//!
//! ```ignore
//! #[forge(name = "warmup", on_start)]
//! async fn warmup() { /* load caches */ }
//!
//! #[forge(name = "flush", on_stop)]
//! async fn flush() { /* flush audit/log sinks */ }
//! ```

use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};

/// Boxed Send future runner.
pub type HookFuture = Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// Hook phase identifier (`"start"` or `"stop"`).
pub type Phase = &'static str;

/// A lifecycle hook registered via `inventory::submit!` by the
/// `#[forge(on_start)]` / `#[forge(on_stop)]` macro arguments.
#[derive(Debug)]
pub struct LifecycleHookRegistration {
    /// `"start"` or `"stop"`.
    pub phase: Phase,
    /// Zero-cost runner producing the hook future.
    pub create: fn() -> HookFuture,
}

impl LifecycleHookRegistration {
    /// Const constructor for macro-generated `inventory::submit!` sites.
    pub const fn new(phase: Phase, create: fn() -> HookFuture) -> Self {
        Self { phase, create }
    }
}

inventory::collect!(LifecycleHookRegistration);

static STARTED: AtomicU64 = AtomicU64::new(0);
static STOPPED: AtomicU64 = AtomicU64::new(0);

/// Collect and run all `on_start` hooks in registration order.
///
/// Hook panics are isolated (a panicking hook does not abort the process and
/// does not prevent the remaining hooks from running).
pub async fn run_on_start() {
    for hook in inventory::iter::<LifecycleHookRegistration>() {
        if hook.phase == "start" {
            STARTED.fetch_add(1, Ordering::Relaxed);
            let fut = (hook.create)();
            let _ = std::panic::AssertUnwindSafe(fut).catch_unwind().await;
        }
    }
}

/// Collect and run all `on_stop` hooks in registration order.
pub async fn run_on_stop() {
    for hook in inventory::iter::<LifecycleHookRegistration>() {
        if hook.phase == "stop" {
            STOPPED.fetch_add(1, Ordering::Relaxed);
            let fut = (hook.create)();
            let _ = std::panic::AssertUnwindSafe(fut).catch_unwind().await;
        }
    }
}

/// Number of start hooks executed (diagnostics/tests).
pub fn started_count() -> u64 {
    STARTED.load(Ordering::Relaxed)
}

/// Number of stop hooks executed (diagnostics/tests).
pub fn stopped_count() -> u64 {
    STOPPED.load(Ordering::Relaxed)
}

// Re-export FutureExt::catch_unwind member used above.
use futures_util::FutureExt as _;

#[cfg(all(test, feature = "lifecycle"))]
mod tests {
    use super::*;

    #[test]
    fn registration_is_const_constructible() {
        fn runner() -> HookFuture {
            Box::pin(async {})
        }
        let reg = LifecycleHookRegistration::new("start", runner);
        assert_eq!(reg.phase, "start");
    }
}
