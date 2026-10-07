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
    use std::cell::Cell;

    // 钩子注册集是进程全局（inventory），且其余测试（如 graceful 的 drain 与
    // 收尾路径）也会调 run_on_start/run_on_stop，因此不能拿 started_count()/
    // stopped_count() 做增量断言，也不能用共享布尔量证明“本测试跑到了钩子”
    //（别的线程也会把它置 true）。
    // `#[tokio::test]` 默认 current_thread，run_on_start/run_on_stop 在当前线程
    // 上逐钩同步执行，故线程局部计数只会被本测试抬高——断言既确定又能真正
    // 证明 phase 分流与 panic 隔离。
    thread_local! {
        static START_HITS: Cell<usize> = const { Cell::new(0) };
        static STOP_HITS: Cell<usize> = const { Cell::new(0) };
    }

    fn start_ok() -> HookFuture {
        Box::pin(async {
            START_HITS.with(|c| c.set(c.get() + 1));
        })
    }

    /// 钩子 future 内部 panic：文档承诺不得拖垮进程也不得阻止其余钩子运行。
    /// 计数在 panic 之前完成，因此“本线程计数达到注册数”即证明循环未被短路。
    fn start_panics() -> HookFuture {
        Box::pin(async {
            START_HITS.with(|c| c.set(c.get() + 1));
            panic!("lifecycle hook future panic (expected by test)");
        })
    }

    fn stop_ok() -> HookFuture {
        Box::pin(async {
            STOP_HITS.with(|c| c.set(c.get() + 1));
        })
    }

    inventory::submit! { LifecycleHookRegistration::new("start", start_ok) }
    inventory::submit! { LifecycleHookRegistration::new("start", start_panics) }
    inventory::submit! { LifecycleHookRegistration::new("stop", stop_ok) }

    fn registered(phase: Phase) -> usize {
        inventory::iter::<LifecycleHookRegistration>()
            .filter(|h| h.phase == phase)
            .count()
    }

    #[tokio::test]
    async fn hooks_run_per_phase_and_panics_are_isolated() {
        let want_start = registered("start");
        assert!(
            want_start >= 2,
            "本测试至少应注册 2 个 start 钩子（含一个会 panic 的）"
        );

        run_on_start().await;

        assert_eq!(
            START_HITS.with(|c| c.get()),
            want_start,
            "phase=start 钩子应全部被执行，panic 钩子不得短路后续"
        );
        assert_eq!(
            STOP_HITS.with(|c| c.get()),
            0,
            "start 轮次不得执行 phase=stop 钩子"
        );

        run_on_stop().await;

        assert_eq!(
            STOP_HITS.with(|c| c.get()),
            registered("stop"),
            "phase=stop 钩子应被执行"
        );
        assert_eq!(
            START_HITS.with(|c| c.get()),
            want_start,
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
