// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

//! `LimiteronForgeAdapter` — adapts `limiteron::Governor` to `ForgeRateLimiter`.
//!
//! Phase 5 (T034 Red / T035 Green) of the `trait-kit-async-integration`
//! change. Wraps an `Arc<limiteron::Governor>` and implements
//! [`ForgeRateLimiter`] by delegating `check` to `Governor::check` and making
//! `record` a no-op (Governor::check is atomic — check + consume together;
//! there is no separate `record` method on `Governor`).
//!
//! Gated by the `limiteron-integration` feature.
//!
//! # Rule 7 divergence from `spec.md`
//!
//! `spec.md` R-sdforge-module-002 wrote the error mapping as
//! `ForgeError::rate_limiter(message)`. The actual `ForgeError` enum (defined
//! in T033 at `domain/rate_limiter.rs`) has **no `rate_limiter` constructor**;
//! the closest semantic match is [`ForgeError::internal(impl Display)`], which
//! we use for mapping `limiteron::FlowGuardError`. This divergence is
//! documented here rather than silently papered over.

use crate::domain::rate_limiter::{ForgeError, ForgeRateLimiter};
use limiteron::{Decision, Governor, RequestContext};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Adapter wrapping `limiteron::Governor` as a [`ForgeRateLimiter`].
///
/// Holds an `Arc<Governor>` so the same governor can be shared across
/// handlers/threads. `check` delegates to `Governor::check` (atomic
/// check + consume); `record` is a no-op because `Governor::check` already
/// consumes the token atomically — there is no separate `record` method on
/// `Governor`.
pub struct LimiteronForgeAdapter {
    governor: Arc<Governor>,
}

impl LimiteronForgeAdapter {
    /// Construct a new adapter wrapping the given `Arc<Governor>`.
    #[must_use]
    pub fn new(governor: Arc<Governor>) -> Self {
        Self { governor }
    }
}

impl ForgeRateLimiter for LimiteronForgeAdapter {
    fn check<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ForgeError>> + Send + 'a>> {
        Box::pin(async move {
            let mut ctx = RequestContext::new();
            // limiteron's `MatchCondition::Ip` reads `context.client_ip`.
            // We treat the incoming `key` as a client IP, matching the
            // config's `Matcher::Ip { ip_ranges: ["0.0.0.0/0"] }`.
            ctx.client_ip = Some(key.to_string());
            let decision = self
                .governor
                .check(&ctx)
                .await
                .map_err(ForgeError::internal)?;
            match decision {
                Decision::Allowed(_) => Ok(true),
                Decision::Rejected(_) | Decision::Banned(_) => Ok(false),
            }
        })
    }

    fn record<'a>(
        &'a self,
        _key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ForgeError>> + Send + 'a>> {
        // No-op: Governor::check is atomic (check + consume together).
        // There is no separate `record` method on `Governor`.
        Box::pin(async move { Ok(()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*; // brings in LimiteronForgeAdapter, ForgeError, ForgeRateLimiter
    use crate::domain::rate_limiter::ForgeRateLimiter;
    use limiteron::config::{GlobalConfig, Matcher, Rule};
    use limiteron::storage::{MemoryBanStorage, MemoryStorage};
    use limiteron::{
        ActionConfig, BanStorage, FlowControlConfig, Governor, LimiterConfig, Storage,
    };
    use std::sync::Arc;

    /// Construct a minimal valid `FlowControlConfig` matching all IPs with
    /// a permissive token bucket (100 burst, 10 req/s refill). Mirrors the
    /// `default_config()` pattern from `src/security/ratelimit/adapter.rs`.
    fn make_minimal_config() -> FlowControlConfig {
        FlowControlConfig {
            version: "0.1.0".to_string(),
            global: GlobalConfig::default(),
            rules: vec![Rule {
                id: "test-default".to_string(),
                name: "test default rule".to_string(),
                priority: 100,
                matchers: vec![Matcher::Ip {
                    ip_ranges: vec!["0.0.0.0/0".to_string()],
                }],
                limiters: vec![LimiterConfig::TokenBucket {
                    capacity: 1,
                    refill_rate: 1,
                }],
                action: ActionConfig::default(),
            }],
        }
    }

    /// Construct a real `Governor` backed by in-memory storage for testing.
    async fn make_governor() -> Governor {
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
        let ban_storage: Arc<dyn BanStorage> = Arc::new(MemoryBanStorage::new());
        Governor::builder()
            .with_config(make_minimal_config())
            .with_storage(storage)
            .with_ban_storage(ban_storage)
            .build()
            .await
            .expect("minimal config must produce a valid Governor")
    }

    /// R-sdforge-module-002 #1: `LimiteronForgeAdapter::new` accepts an
    /// `Arc<Governor>` and returns a `LimiteronForgeAdapter`.
    #[tokio::test]
    async fn limiteron_forge_adapter_new_accepts_arc_governor() {
        let governor = make_governor().await;
        let _adapter = LimiteronForgeAdapter::new(Arc::new(governor));
    }

    /// R-sdforge-module-002 #2: `check` delegates to `Governor::check` and
    /// returns `Ok(true)` when the request is allowed (TokenBucket has tokens).
    #[tokio::test]
    async fn check_returns_ok_true_when_allowed() {
        let governor = make_governor().await;
        let adapter = LimiteronForgeAdapter::new(Arc::new(governor));
        let allowed = adapter.check("1.2.3.4").await.expect("check should succeed");
        assert!(allowed, "first request must be allowed by the 100-token bucket");
    }

    /// R-sdforge-module-002 #3: `record` is a no-op returning `Ok(())`
    /// (Governor::check is atomic — check + consume together; there is no
    /// separate `record` method on `Governor`).
    #[tokio::test]
    async fn record_is_noop_returning_ok() {
        let governor = make_governor().await;
        let adapter = LimiteronForgeAdapter::new(Arc::new(governor));
        adapter
            .record("1.2.3.4")
            .await
            .expect("record should be a no-op Ok");
    }

    /// R-sdforge-module-002 #4: `LimiteronForgeAdapter` implements
    /// `ForgeRateLimiter` — object-safe via `Arc<dyn ForgeRateLimiter +
    /// Send + Sync>`. This is the dyn-dispatch path `SdforgeModule::build`
    /// will use (T036/T037).
    #[tokio::test]
    async fn adapter_is_object_safe_as_forge_rate_limiter() {
        let governor = make_governor().await;
        let adapter = LimiteronForgeAdapter::new(Arc::new(governor));
        let limiter: Arc<dyn ForgeRateLimiter + Send + Sync> = Arc::new(adapter);
        let allowed = limiter.check("5.6.7.8").await.expect("check via dyn dispatch");
        assert!(allowed, "dyn-dispatched check must return Ok(true)");
    }

    /// R-sdforge-module-002 #5: `LimiteronForgeAdapter` is `Send + Sync`
    /// (required by `ForgeRateLimiter: Send + Sync`).
    #[test]
    fn adapter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LimiteronForgeAdapter>();
    }

    /// R-sdforge-module-002 #6: `check` maps `Decision::Rejected` to
    /// `Ok(false)` (throttled, not an error). With capacity=1/refill=1,
    /// the 1st request consumes the only token; the 2nd immediate request
    /// is rejected because <1 second has elapsed (negligible refill).
    ///
    /// Note: Governor's L1 cache is hardcoded `true` (governor.rs line 241),
    /// independent of `CacheBackend` config. Without `clear_l1_cache()`,
    /// the 2nd check returns the cached `Allowed` decision, masking the
    /// exhausted token bucket.
    #[tokio::test]
    async fn check_returns_ok_false_when_rejected() {
        let governor = Arc::new(make_governor().await);
        let adapter = LimiteronForgeAdapter::new(governor.clone());
        // 1st request — allowed (1 token available, capacity=1).
        let allowed = adapter.check("9.9.9.9").await.expect("first check");
        assert!(allowed, "first request must be allowed");
        // Clear L1 cache so the 2nd check hits the actual token bucket
        // (not a cached Allowed decision).
        governor.clear_l1_cache().await;
        // 2nd request immediately after — rejected (0 tokens, refill=1/sec).
        let allowed = adapter
            .check("9.9.9.9")
            .await
            .expect("check should not error even when rejected");
        assert!(!allowed, "second request must be rejected (Ok(false))");
    }
}
