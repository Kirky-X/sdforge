// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `SdforgeModule` — trait-kit 0.2.2 `AsyncKit` integration for sdforge.
//!
//! Phase 5 (T036 Red / T037 Green) of the `trait-kit-async-integration`
//! change. Wires sdforge's rate-limiting capability into the `AsyncKit`
//! dependency injection framework, depending on `LimiteronModule` for the
//! underlying `Governor`.
//!
//! Gated by the `kit` feature (which implies `limiteron-integration`).
//!
//! # Rule 7 divergences from `spec.md` / `design.md` (expose, don't paper
//! over)
//!
//! `spec.md` R-sdforge-module-003 and `design.md` Decision 3 (lines 441-470)
//! wrote the following pseudo-code; sdforge's actual API diverges on **four**
//! independent points, all surfaced here rather than papered over:
//!
//! 1. **`ForgeApp` trait does NOT exist** — spec wrote
//!    `Capability = Arc<dyn ForgeApp + Send + Sync>`. Grep for `ForgeApp` in
//!    `sdforge/src` returns 0 matches. The capability is
//!    `Arc<dyn ForgeRateLimiter + Send + Sync>` (the adapter itself is the
//!    capability — it IS the rate limiter abstraction).
//!
//! 2. **`ForgeBuilder` does NOT exist** — spec wrote
//!    `ForgeBuilder::new().config(config).rate_limiter(adapter).build().await`.
//!    Grep for `ForgeBuilder` in `sdforge/src` returns 0 matches. There is no
//!    multi-step builder to construct a ForgeApp. The adapter is wrapped in
//!    `Arc` and returned directly as the capability.
//!
//! 3. **`SdforgeConfig` does NOT exist** — spec wrote
//!    `kit.config::<SdforgeConfig>()`. Grep for `SdforgeConfig` in
//!    `sdforge/src` returns 0 matches. The adapter only needs the `Governor`
//!    from `LimiteronModule`; no additional config is read.
//!
//! 4. **Error type is `SdForgeError` (capital F), not `SdforgeError`** — spec
//!    wrote `Error = SdforgeError`. The actual type is
//!    [`SdForgeError`] (defined at
//!    `core/error/sdforge_error.rs`, implements `std::error::Error` via
//!    thiserror). We use `SdForgeError` as `AsyncAutoBuilder::Error`.

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, OnceLock};

use trait_kit::prelude::*;

use limiteron::integrations::kit::LimiteronModule;

use crate::core::error::SdForgeError;
use crate::domain::rate_limiter::ForgeRateLimiter;
use crate::integrations::limiteron_adapter::LimiteronForgeAdapter;

/// trait-kit `AsyncKit` module that constructs an sdforge rate-limiter
/// capability.
///
/// Depends on `LimiteronModule` (registered first via topological sort).
/// Register with `AsyncKit::register::<SdforgeModule>()`, configure
/// `LimiteronModule` via `kit.set_config(FlowControlConfig { ... })`, then
/// `kit.build().await` and retrieve the capability with
/// `kit.require::<SdforgeModule>()`.
///
/// The returned `Arc<dyn ForgeRateLimiter + Send + Sync>` wraps a
/// [`LimiteronForgeAdapter`] — see the module-level docs for the
/// design-divergence rationale (spec.md wrote `Arc<dyn ForgeApp>`, but
/// `ForgeApp` does not exist in sdforge).
pub struct SdforgeModule;

impl ModuleMeta for SdforgeModule {
    const NAME: &'static str = "sdforge";

    fn dependencies() -> &'static [(&'static str, TypeId)] {
        // OnceLock lazy init — mirrors dbnexus's pattern (Phase 4).
        // OnceLock (stable since 1.70) gives a `&'static` reference to a
        // runtime-constructed `Vec`.
        static DEPS: OnceLock<Vec<(&'static str, TypeId)>> = OnceLock::new();
        DEPS.get_or_init(|| vec![("limiteron", TypeId::of::<LimiteronModule>())])
            .as_slice()
    }
}

impl AsyncAutoBuilder for SdforgeModule {
    type Capability = Arc<dyn ForgeRateLimiter + Send + Sync>;
    type Error = SdForgeError;

    fn build<'a>(
        kit: &'a AsyncKit,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Capability, Self::Error>> + Send + 'a>> {
        Box::pin(async move {
            // 1. Require LimiteronModule capability (Arc<Governor>).
            let governor = kit
                .require::<LimiteronModule>()
                .map_err(|e| SdForgeError::internal(format!("require LimiteronModule: {e}")))?;

            // 2. Wrap in LimiteronForgeAdapter — the adapter IS the capability
            //    (Rule 7: ForgeApp/ForgeBuilder don't exist; the rate-limiter
            //    abstraction IS the capability, not a ForgeApp that contains it).
            let adapter = LimiteronForgeAdapter::new(governor);

            // 3. Return as Arc<dyn ForgeRateLimiter + Send + Sync>.
            Ok(Arc::new(adapter) as Arc<dyn ForgeRateLimiter + Send + Sync>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use limiteron::config::{
        Action, ActionConfig, FlowControlConfig, LimiterConfig, Matcher, Rule,
    };

    /// Build a minimal valid `FlowControlConfig` (1 rule matching all IPs with
    /// a permissive token bucket). `FlowControlConfig::default()` has an
    /// empty rules vec which fails validation. Mirrors the config pattern from
    /// `limiteron_adapter::tests::make_minimal_config`.
    fn make_minimal_valid_config() -> FlowControlConfig {
        let mut config = FlowControlConfig::default();
        config.rules.push(Rule {
            id: "default".to_string(),
            name: "default rule".to_string(),
            priority: 0,
            matchers: vec![Matcher::Ip {
                ip_ranges: vec!["0.0.0.0/0".to_string()],
            }],
            limiters: vec![LimiterConfig::TokenBucket {
                capacity: 100,
                refill_rate: 10,
            }],
            action: ActionConfig {
                on_exceed: Action::Reject,
                ban: None,
            },
        });
        config
    }

    /// R-sdforge-module-003 #1: `SdforgeModule::NAME == "sdforge"`.
    #[test]
    fn sdforge_module_meta_name() {
        assert_eq!(SdforgeModule::NAME, "sdforge");
    }

    /// R-sdforge-module-003 #2: `SdforgeModule::dependencies()` declares a
    /// dependency on `LimiteronModule`.
    #[test]
    fn sdforge_module_meta_dependencies() {
        let deps = SdforgeModule::dependencies();
        assert_eq!(deps.len(), 1, "SdforgeModule should depend on 1 module");
        assert_eq!(deps[0].0, "limiteron", "dep name should be 'limiteron'");
        assert_eq!(
            deps[0].1,
            TypeId::of::<LimiteronModule>(),
            "dep TypeId should match LimiteronModule"
        );
    }

    /// R-sdforge-module-003 #3: `SdforgeModule` satisfies `AsyncAutoBuilder`
    /// trait bounds — `Capability: Clone + Send + Sync + 'static` and
    /// `Error: std::error::Error + Send + 'static`.
    #[test]
    fn sdforge_module_satisfies_async_auto_builder_bounds() {
        fn assert_cap<T: Clone + Send + Sync + 'static>() {}
        assert_cap::<Arc<dyn ForgeRateLimiter + Send + Sync>>();
        fn assert_err<T: std::error::Error + Send + 'static>() {}
        assert_err::<SdForgeError>();
    }

    /// R-sdforge-module-003 #4: Full integration — register LimiteronModule +
    /// SdforgeModule, set config, build, require SdforgeModule → get a
    /// working `Arc<dyn ForgeRateLimiter + Send + Sync>` that delegates
    /// `check` to the underlying `Governor`.
    #[tokio::test]
    async fn sdforge_module_build_returns_rate_limiter() {
        let mut kit = AsyncKit::new();
        kit.set_config(make_minimal_valid_config());
        kit.register::<LimiteronModule>()
            .expect("register LimiteronModule");
        kit.register::<SdforgeModule>()
            .expect("register SdforgeModule");
        let kit = kit.build().await.expect("AsyncKit::build");
        let limiter: Arc<dyn ForgeRateLimiter + Send + Sync> = kit
            .require::<SdforgeModule>()
            .expect("require SdforgeModule");
        // Verify the rate limiter is usable — first check must be allowed
        // (TokenBucket has 100 tokens).
        let allowed = limiter
            .check("1.2.3.4")
            .await
            .expect("check should succeed");
        assert!(allowed, "first request must be allowed");
    }

    /// R-sdforge-module-003 #5: build fails with a clear error if
    /// LimiteronModule is not registered (dependency missing).
    #[tokio::test]
    async fn sdforge_module_build_fails_without_limiteron() {
        let mut kit = AsyncKit::new();
        kit.set_config(make_minimal_valid_config());
        // Register only SdforgeModule — LimiteronModule is missing.
        kit.register::<SdforgeModule>()
            .expect("register SdforgeModule");
        let err = kit.build().await.expect_err("build should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("limiteron"),
            "error should mention limiteron dependency, got: {msg}"
        );
    }
}
