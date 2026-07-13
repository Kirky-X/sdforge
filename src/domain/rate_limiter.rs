// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Rate limiter abstraction for the trait-kit 0.3 `AsyncKit` integration.
//!
//! Defines the [`ForgeRateLimiter`] trait — the minimal rate-limiter
//! interface sdforge consumes when wiring up `SdforgeModule` (Phase 5 of
//! `trait-kit-async-integration`). The trait uses explicit
//! `Pin<Box<dyn Future + Send>>` return types (rather than `async fn` in
//! trait) to remain object-safe (`dyn ForgeRateLimiter` compiles) without
//! pulling in the `async-trait` crate. This mirrors the pattern used by
//! `trait_kit::AsyncAutoBuilder` (see `trait-kit/src/core/meta.rs`) and
//! dbnexus's `DbCacheProvider` (Phase 4).
//!
//! # Relationship to the existing `RateLimiter` trait
//!
//! sdforge already defines a `RateLimiter` trait at
//! `src/security/ratelimit/mod.rs` (gated by the `ratelimit` feature). That
//! trait is identifier-based (it has a `check(&str)` method and returns
//! `Result<(), RateLimitError>`). The HTTP-specific `check_request` method
//! lives in the separate `HttpRequestRateLimiter` trait (gated by
//! `ratelimit-http`). `ForgeRateLimiter` is a distinct, simpler abstraction
//! for the kit integration domain: it is key-based
//! (`check(&str) -> Result<bool, ForgeError>`) and has a separate `record`
//! method. The two traits coexist for different purposes — do not merge them.

use std::future::Future;
use std::pin::Pin;

/// Error type returned by [`ForgeRateLimiter`] implementations.
///
/// A small, dependency-free error enum (thiserror-based) so the trait can be
/// defined in the always-compiled `domain` module without pulling in
/// `limiteron` or other optional crates. Adapter implementations map
/// provider-specific errors (e.g. `limiteron::LimiteronError`) into the
/// `Internal` variant — mirroring how `LimiteronModule` maps `TraitKitError` via
/// `LimiteronError::ConfigError(format!(...))` (Phase 3 precedent).
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    /// Rate limit exceeded — the identifier is allowed `limit` requests per
    /// `window_seconds` window.
    #[error("Rate limit exceeded: {limit} per {window_seconds}s")]
    RateLimited {
        /// Maximum requests allowed in the window.
        limit: u64,
        /// Window duration in seconds.
        window_seconds: u64,
    },
    /// Identifier is banned (longer-term block).
    #[error("Identifier banned: {reason}")]
    Banned {
        /// Human-readable ban reason.
        reason: String,
    },
    /// Wraps any other error surfaced by the underlying rate-limiter provider
    /// (e.g. `limiteron::LimiteronError`). The source error is stringified so
    /// `ForgeError` stays dependency-free.
    #[error("Rate limiter internal error: {message}")]
    Internal {
        /// Stringified source error message.
        message: String,
    },
}

impl ForgeError {
    /// Construct an `Internal` variant from anything that implements
    /// `Display`. Convenience for adapter implementations mapping
    /// provider-specific errors.
    #[must_use]
    pub fn internal(error: impl std::fmt::Display) -> Self {
        Self::Internal {
            message: error.to_string(),
        }
    }
}

/// Rate limiter abstraction consumed by the trait-kit 0.3 `AsyncKit`
/// integration (Phase 5 of `trait-kit-async-integration`).
///
/// Defines a minimal, key-based rate-limiter interface: `check` queries
/// whether a request for the given key is allowed (returns `true` = allowed,
/// `false` = throttled), and `record` commits a request observation.
///
/// The trait uses explicit `Pin<Box<dyn Future + Send>>` return types (rather
/// than `async fn` in trait) to remain object-safe — `dyn ForgeRateLimiter`
/// compiles and can be stored as `Arc<dyn ForgeRateLimiter + Send + Sync>`.
/// This mirrors the pattern used by `trait_kit::AsyncAutoBuilder` and
/// dbnexus's `DbCacheProvider`. No `async-trait` crate is pulled in.
///
/// # Implementations
///
/// - [`LimiteronForgeAdapter`](crate::integrations::LimiteronForgeAdapter)
///   (gated by the `limiteron-integration` feature) wraps a
///   `limiteron::Governor`.
pub trait ForgeRateLimiter: Send + Sync {
    /// Check whether a request for the given key is allowed.
    ///
    /// Returns `Ok(true)` if the request is allowed, `Ok(false)` if it is
    /// throttled (rate-limited or banned), or `Err(ForgeError)` if the
    /// underlying provider failed.
    ///
    /// # Arguments
    ///
    /// * `key` — the rate-limit key (e.g. IP, user ID, API key).
    #[allow(
        clippy::type_complexity,
        reason = "Pin<Box<dyn Future + Send>> is the canonical dyn-compatible async trait dispatch type; mirrors AsyncAutoBuilder::build"
    )]
    fn check<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<bool, ForgeError>> + Send + 'a>>;

    /// Record a request observation for the given key.
    ///
    /// Some rate-limiter backends combine check + consume into a single
    /// atomic operation (e.g. limiteron's `Governor::check`); for those
    /// backends, `record` may be a no-op. The separation in the trait
    /// accommodates backends that distinguish "peek" from "commit".
    ///
    /// # Arguments
    ///
    /// * `key` — the rate-limit key.
    #[allow(
        clippy::type_complexity,
        reason = "Pin<Box<dyn Future + Send>> is the canonical dyn-compatible async trait dispatch type; mirrors AsyncAutoBuilder::build"
    )]
    fn record<'a>(
        &'a self,
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), ForgeError>> + Send + 'a>>;
}

#[cfg(test)]
mod tests {
    // T032 Red: tests referencing the not-yet-defined ForgeRateLimiter trait
    // and ForgeError error type. This module fails to compile until T033
    // (Green) adds the trait + error definitions above.

    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    /// A mock `ForgeRateLimiter` implementation used to verify the trait is
    /// implementable and object-safe. Returns hardcoded values.
    struct MockForgeRateLimiter {
        allow: bool,
    }

    impl ForgeRateLimiter for MockForgeRateLimiter {
        fn check<'a>(
            &'a self,
            _key: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<bool, ForgeError>> + Send + 'a>> {
            Box::pin(async move { Ok(self.allow) })
        }

        fn record<'a>(
            &'a self,
            _key: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), ForgeError>> + Send + 'a>> {
            Box::pin(async move { Ok(()) })
        }
    }

    /// R-sdforge-module-001 #1: `ForgeRateLimiter` trait exists and its
    /// `check` method returns `Pin<Box<dyn Future<Output = Result<bool,
    /// ForgeError>> + Send + 'a>>` (returns true = allowed).
    #[tokio::test]
    async fn forge_rate_limiter_check_returns_pin_box_future() {
        let limiter = MockForgeRateLimiter { allow: true };
        let result: Result<bool, ForgeError> = limiter.check("key").await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    /// R-sdforge-module-001 #2: `record` method returns
    /// `Pin<Box<dyn Future<Output = Result<(), ForgeError>> + Send + 'a>>`.
    #[tokio::test]
    async fn forge_rate_limiter_record_returns_pin_box_future() {
        let limiter = MockForgeRateLimiter { allow: true };
        let result: Result<(), ForgeError> = limiter.record("key").await;
        assert!(result.is_ok());
    }

    /// R-sdforge-module-001 #3: `ForgeRateLimiter` is object-safe —
    /// `dyn ForgeRateLimiter` compiles. This is required for the kit
    /// integration to store the rate limiter as `Arc<dyn ForgeRateLimiter +
    /// Send + Sync>`.
    #[tokio::test]
    async fn forge_rate_limiter_is_object_safe() {
        let limiter: Arc<dyn ForgeRateLimiter + Send + Sync> =
            Arc::new(MockForgeRateLimiter { allow: false });
        // Verify the trait object can be called via Arc.
        let allowed: bool = limiter.check("k").await.unwrap();
        assert!(!allowed);
    }

    /// R-sdforge-module-001 #4: `ForgeRateLimiter` trait inherits
    /// `Send + Sync` (required for `Arc<dyn ForgeRateLimiter + Send + Sync>`).
    #[test]
    fn forge_rate_limiter_requires_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockForgeRateLimiter>();
    }

    /// R-sdforge-module-001 #5: `ForgeError` implements `std::error::Error`
    /// (required by `AsyncAutoBuilder::Error` bound when used as a trait
    /// method return type) and is `Send + 'static`.
    #[test]
    fn forge_error_implements_std_error() {
        fn assert_error<E: std::error::Error + Send + 'static>() {}
        assert_error::<ForgeError>();
    }

    /// R-sdforge-module-001 #6: `ForgeError` can be constructed and displayed.
    #[test]
    fn forge_error_display() {
        let err = ForgeError::RateLimited {
            limit: 100,
            window_seconds: 60,
        };
        let msg = err.to_string();
        assert!(msg.contains("100"));
        assert!(msg.contains("60"));
    }

    /// `ForgeError::internal` constructor maps a `Display`-able error into
    /// the `Internal` variant and stringifies the message.
    #[test]
    fn forge_error_internal_constructor() {
        let err = ForgeError::internal("backend down");
        let msg = err.to_string();
        assert!(
            msg.contains("backend down"),
            "Internal variant should embed source message: got {msg}"
        );
        assert!(
            msg.contains("internal error"),
            "Internal variant should use its Display template: got {msg}"
        );
    }

    /// `ForgeError::Banned` variant should round-trip its reason.
    #[test]
    fn forge_error_banned_display() {
        let err = ForgeError::Banned {
            reason: "abuse".into(),
        };
        assert!(err.to_string().contains("abuse"));
    }
}
