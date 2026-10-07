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
mod hook_isolation_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static A_RAN: AtomicBool = AtomicBool::new(false);
    static STOP_RAN: AtomicBool = AtomicBool::new(false);

    fn start_ok() -> HookFuture {
        Box::pin(async {
            A_RAN.store(true, Ordering::SeqCst);
        })
    }

    /// 钩子 future 内部 panic：文档承诺不得拖垮进程也不得阻止其余钩子运行。
    fn start_panics() -> HookFuture {
        Box::pin(async { panic!("lifecycle hook future panic (expected by test)") })
    }

    fn stop_ok() -> HookFuture {
        Box::pin(async {
            STOP_RAN.store(true, Ordering::SeqCst);
        })
    }

    inventory::submit! { LifecycleHookRegistration::new("start", start_ok) }
    inventory::submit! { LifecycleHookRegistration::new("start", start_panics) }
    inventory::submit! { LifecycleHookRegistration::new("stop", stop_ok) }

    // 单测内按顺序跑两轮：计数器与标志均为进程全局，拆成两个并发测试会互相脏读。
    #[tokio::test]
    async fn hooks_run_per_phase_and_panics_are_isolated() {
        let started0 = started_count();
        let stopped0 = stopped_count();

        run_on_start().await;

        assert!(A_RAN.load(Ordering::SeqCst), "phase=start 钩子应被执行");
        assert_eq!(
            started_count() - started0,
            2,
            "两个 start 钩子均应计数（panic 钩子不短路后续）"
        );
        assert_eq!(
            stopped_count() - stopped0,
            0,
            "start 轮次不得执行 phase=stop 钩子"
        );

        run_on_stop().await;

        assert!(STOP_RAN.load(Ordering::SeqCst), "phase=stop 钩子应被执行");
        assert_eq!(stopped_count() - stopped0, 1, "仅一个 stop 钩子");
        assert_eq!(
            started_count() - started0,
            2,
            "stop 轮次不得重跑 start 钩子"
        );
    }
}

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
