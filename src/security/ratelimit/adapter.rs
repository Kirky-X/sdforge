// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! `LimiteronAdapter` — concrete `RateLimiter` wrapping `limiteron::Governor`.
//!
//! This is the production implementation of the [`RateLimiter`] trait. It
//! delegates all decisions to a [`limiteron::Governor`] instance held behind
//! an `Arc` (shared ownership, dependency-injected).
//!
//! See `design.md` D3 for the rationale behind the construction pattern
//! (async `new()` / `default()` + sync `with_dependencies`).
//!
//! Requires the `ratelimit` feature.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use limiteron::config::{GlobalConfig, Matcher, Rule};
use limiteron::storage::{MemoryBanStorage, MemoryStorage};
use limiteron::{
    ActionConfig, BanStorage, Decision, FlowControlConfig, Governor, LimiterConfig, RequestContext,
    Storage,
};

use super::{RateLimitError, RateLimiter};

/// Production [`RateLimiter`] backed by a [`limiteron::Governor`].
///
/// Holds an `Arc<Governor>` so the same governor can be shared across
/// handlers/threads. The governor is constructed via `Governor::builder()`
/// with a minimal valid config (see [`default_config`]) or injected directly
/// via [`Self::with_dependencies`].
///
/// # Why no `Default` impl?
///
/// `Governor::new()` is async and panics with `FlowControlConfig::default()`
/// (empty `rules` fails validation). We provide [`Self::default()`] (an async
/// fn) that uses [`default_config`] instead.
pub struct LimiteronAdapter {
    /// Shared governor instance. `Arc` because `Governor` is shared across
    /// clones of the adapter and may be accessed concurrently.
    governor: Arc<Governor>,
}

/// Construct a minimal valid `FlowControlConfig` for out-of-the-box usage.
///
/// `FlowControlConfig::default()` has an empty `rules` vec, which fails
/// `validate()` with "至少需要一个规则" (and `Governor::new()` panics on this).
/// This helper provides one permissive rule: match all IPs (`0.0.0.0/0`)
/// with a token bucket (100 burst, 10 req/s refill) and default `Reject`
/// action on exceed.
///
/// Production users should inject their own config via
/// [`LimiteronAdapter::with_dependencies`] or the builder.
fn default_config() -> FlowControlConfig {
    FlowControlConfig {
        version: "0.1.0".to_string(),
        global: GlobalConfig::default(),
        rules: vec![Rule {
            id: "sdforge-default".to_string(),
            name: "sdforge default rate limit".to_string(),
            priority: 100,
            matchers: vec![Matcher::Ip {
                ip_ranges: vec!["0.0.0.0/0".to_string()],
            }],
            limiters: vec![LimiterConfig::TokenBucket {
                capacity: 100,
                refill_rate: 10,
            }],
            action: ActionConfig::default(),
        }],
    }
}

/// Extract the client identifier (IP) from an HTTP request.
///
/// Delegates to [`crate::security::ip_util::extract_client_ip_core`] so that
/// the rate-limit adapter applies the **same spoofing-defense logic** as the
/// authentication middleware:
///
/// 1. The direct TCP peer IP (`ConnectInfo<SocketAddr>`) is the only
///    unspoofable source.
/// 2. `X-Forwarded-For` / `X-Real-IP` headers are trusted **only** when the
///    direct peer is a trusted reverse proxy (private ranges). Otherwise an
///    attacker could spoof these headers to bypass IP-based rate limits.
/// 3. When no `ConnectInfo` is available (e.g. test environments without a
///    real TCP connection), headers are trusted as a last-resort fallback.
///
/// Returns `"unknown"` only when no IP can be determined from any source —
/// in that case all such requests share a single `"unknown"` bucket (which
/// is the intended conservative behavior: deny-by-shared-limit rather than
/// allow-by-spoofed-header).
fn extract_identifier(req: &Request<Body>) -> String {
    crate::security::ip_util::extract_client_ip_core(req).unwrap_or_else(|| "unknown".to_string())
}

impl LimiteronAdapter {
    /// Out-of-the-box construction (mode 1).
    ///
    /// Internally uses `Governor::builder()` with [`default_config`] and
    /// in-memory storage. The default rule is permissive (100 req burst,
    /// 10 req/s refill, match all IPs) — suitable for tests and getting
    /// started. Production should inject a custom config via
    /// [`Self::with_dependencies`] or [`Self::builder`].
    pub async fn new() -> Self {
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
        let ban_storage: Arc<dyn BanStorage> = Arc::new(MemoryBanStorage::new());
        let governor = Governor::builder()
            .with_config(default_config())
            .with_storage(storage)
            .with_ban_storage(ban_storage)
            .build()
            .await
            .expect("default_config() must produce a valid FlowControlConfig");
        Self {
            governor: Arc::new(governor),
        }
    }

    /// Async equivalent of `Default::default()` (mode 1 alternative).
    ///
    /// Provided because `Governor` construction is async, so we cannot
    /// implement the sync `Default` trait.
    pub async fn default() -> Self {
        Self::new().await
    }

    /// Builder pattern (mode 2).
    ///
    /// Returns a [`LimiteronAdapterBuilder`] for fluent construction with
    /// custom config.
    pub fn builder() -> LimiteronAdapterBuilder {
        LimiteronAdapterBuilder::default()
    }

    /// Full dependency injection (mode 3).
    ///
    /// Accepts an already-constructed `Arc<Governor>`, bypassing
    /// `Governor::with_dependencies` (which requires `DashMap` — forbidden
    /// in sdforge direct code). Use this when you need a custom governor
    /// configuration that `new()` or `builder()` cannot express.
    pub fn with_dependencies(governor: Arc<Governor>) -> Self {
        Self { governor }
    }
}

impl RateLimiter for LimiteronAdapter {
    fn check<'a>(
        &'a self,
        identifier: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        Box::pin(async move {
            let mut ctx = RequestContext::new();
            // limiteron's `IpExtractor::extract()` and `MatchCondition::Ip`
            // both read `context.client_ip` (not `context.ip`). We treat the
            // incoming `identifier` as a client IP, which matches the default
            // config's `Matcher::Ip { ip_ranges: ["0.0.0.0/0"] }`.
            ctx.client_ip = Some(identifier.to_string());
            let decision = self.governor.check(&ctx).await?;
            match decision {
                Decision::Allowed(_) => Ok(()),
                Decision::Rejected(meta) => Err(RateLimitError::Exceeded {
                    limit: meta.limit,
                    // `retry_after` (seconds to wait before retry) is the closest
                    // available field to "window_seconds". Semantically imperfect
                    // but the best mapping from limiteron's RejectionMetadata.
                    window_seconds: meta.retry_after,
                }),
                Decision::Banned(info) => Err(RateLimitError::Banned {
                    reason: info.reason().to_string(),
                }),
            }
        })
    }

    fn check_request<'a>(
        &'a self,
        req: &'a Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        // Extract the identifier synchronously (before entering the async
        // block) so the future doesn't capture `&Request<Body>`. `Body` is
        // not `Sync`, so `&Request<Body>` is not `Send` and cannot cross the
        // `.await` boundary inside a `Send` future.
        let identifier = extract_identifier(req);
        Box::pin(async move { self.check(&identifier).await })
    }
}

/// Builder for [`LimiteronAdapter`] (mode 2 construction).
///
/// Allows fluent configuration of the `FlowControlConfig` before building.
/// If no config is set, [`default_config`] is used.
#[derive(Default)]
pub struct LimiteronAdapterBuilder {
    config: Option<FlowControlConfig>,
}

impl LimiteronAdapterBuilder {
    /// Set a custom `FlowControlConfig`.
    pub fn with_config(mut self, config: FlowControlConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Build the `LimiteronAdapter` with in-memory storage and the configured
    /// (or default) config.
    ///
    /// Returns `Err(RateLimitError)` when the configured `FlowControlConfig`
    /// fails limiteron's validation (e.g. empty `rules`). This is a
    /// user-input failure, so we surface it as an explicit `Result` rather
    /// than panicking (Rule 12: failures must be explicit).
    pub async fn build(self) -> Result<LimiteronAdapter, RateLimitError> {
        let config = self.config.unwrap_or_else(default_config);
        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
        let ban_storage: Arc<dyn BanStorage> = Arc::new(MemoryBanStorage::new());
        let governor = Governor::builder()
            .with_config(config)
            .with_storage(storage)
            .with_ban_storage(ban_storage)
            .build()
            .await?;
        Ok(LimiteronAdapter::with_dependencies(Arc::new(governor)))
    }
}
