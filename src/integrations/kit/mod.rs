// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! trait-kit 0.3 `AsyncKit` integration for sdforge.
//!
//! Enable via the `kit` cargo feature. Provides [`SdforgeModule`] — a module
//! depending on `limiteron::integrations::kit::LimiteronModule` that constructs
//! an `Arc<dyn ForgeRateLimiter + Send + Sync>` capability (wrapping
//! [`LimiteronForgeAdapter`](crate::integrations::LimiteronForgeAdapter))
//! during [`AsyncKit::build`](trait_kit::AsyncKit::build).
//!
//! Also hosts the process-global ready-kit registry (T704): register a built
//! `AsyncKit<AsyncReady>` with [`set_ready_kit`] and
//! `http::serve_with_graceful_shutdown` runs its phased
//! `shutdown_async()` as the final stage of the graceful-stop sequence.

pub mod module;
pub use module::SdforgeModule;

use std::sync::{Arc, Mutex, OnceLock};
use trait_kit::{AsyncKit, AsyncReady};

static READY_KIT: OnceLock<Mutex<Option<Arc<AsyncKit<AsyncReady>>>>> = OnceLock::new();

fn ready_kit_slot() -> &'static Mutex<Option<Arc<AsyncKit<AsyncReady>>>> {
    READY_KIT.get_or_init(|| Mutex::new(None))
}

/// Register the built (ready) kit used for process-wide integration points:
/// `/readyz` health data (T701, `health` feature) and graceful-shutdown
/// phase-3 teardown (T704, `graceful` feature).
pub fn set_ready_kit(kit: Arc<AsyncKit<AsyncReady>>) {
    if let Ok(mut guard) = ready_kit_slot().lock() {
        *guard = Some(kit);
    }
}

/// Take (consume) the registered ready kit. Returns `None` when unset.
pub fn take_ready_kit() -> Option<Arc<AsyncKit<AsyncReady>>> {
    ready_kit_slot()
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_kit_registry_set_and_take() {
        let kit = AsyncKit::new();
        let built = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(kit.build())
                .expect("empty kit builds"),
        );
        set_ready_kit(built.clone());
        let taken = take_ready_kit().expect("kit registered");
        assert!(Arc::ptr_eq(&built, &taken));
        assert!(take_ready_kit().is_none(), "take consumes the kit");
    }
}
