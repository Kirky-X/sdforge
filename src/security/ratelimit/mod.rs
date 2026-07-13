// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Rate limiting module backed by limiteron 0.2.1.
//!
//! Provides a unified rate limiting abstraction (`RateLimiter` trait) and a
//! concrete adapter (`LimiteronAdapter`) wrapping `limiteron::Governor`.
//!
//! # Feature gating
//!
//! - `ratelimit`: Core rate limiting — `RateLimiter` trait (check by
//!   identifier), `LimiteronAdapter`, `RateLimitError`. No HTTP dependency.
//! - `ratelimit-http` (implies `ratelimit`): HTTP integration — adds
//!   `HttpRequestRateLimiter` trait (check_request), Tower middleware
//!   (`RateLimitLayer`/`RateLimitMiddleware`), and IP-extraction utilities.

use std::future::Future;
use std::pin::Pin;

// Concrete adapter wrapping `limiteron::Governor`.
mod adapter;
pub use adapter::{LimiteronAdapter, LimiteronAdapterBuilder};

// Tower middleware for HTTP rate limiting (only with `ratelimit-http`).
#[cfg(feature = "ratelimit-http")]
mod middleware;
#[cfg(feature = "ratelimit-http")]
pub use middleware::{RateLimitLayer, RateLimitMiddleware};

#[cfg(test)]
mod tests;

/// Rate limiter abstraction (async methods returning `BoxFuture`).
///
/// Uses `Pin<Box<dyn Future>>` instead of `async-trait` to avoid pulling in
/// an extra dependency. This aligns with the Tower ecosystem pattern.
///
/// All methods take `&self` (not `&mut self`) and the trait inherits
/// `Send + Sync` to support `Arc<dyn RateLimiter>` dependency injection.
///
/// # Feature availability
///
/// This trait is available with the `ratelimit` feature (no HTTP dependency).
/// For HTTP request rate limiting, enable `ratelimit-http` and use
/// [`HttpRequestRateLimiter`].
pub trait RateLimiter: Send + Sync {
    /// Check whether the given identifier (e.g. IP, user ID) is allowed.
    ///
    /// Returns `Ok(())` if the request is allowed, or a `RateLimitError`
    /// describing the rejection reason.
    fn check<'a>(
        &'a self,
        identifier: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>>;
}

/// HTTP request rate limiter abstraction — extends [`RateLimiter`] with
/// HTTP-specific request checking.
///
/// Only available with the `ratelimit-http` feature. Implementors extract the
/// client identifier (e.g. IP) from an HTTP request and delegate to
/// [`RateLimiter::check`].
#[cfg(feature = "ratelimit-http")]
pub trait HttpRequestRateLimiter: RateLimiter {
    /// Extract the identifier from an HTTP request and check it.
    ///
    /// Default implementors should extract the client IP (or other identifier)
    /// and delegate to `check`.
    fn check_request<'a>(
        &'a self,
        req: &'a axum::http::Request<axum::body::Body>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>>;
}

/// Rate limiting error variants aligned with limiteron's `Decision` types.
///
/// `Display` and `std::error::Error` impls are provided by `thiserror::Error`
/// derive. `From<limiteron::LimiteronError>` is auto-derived via `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    /// Request rejected because the rate limit was exceeded.
    #[error("Rate limit exceeded: {limit} per {window_seconds}s")]
    Exceeded {
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
    /// Circuit breaker is open — requests rejected fast until it recovers.
    #[error("Circuit breaker open")]
    CircuitOpen,
    /// Quota exhausted (e.g. daily/monthly allowance used up).
    #[error("Quota exhausted: {used}/{total}")]
    QuotaExhausted {
        /// Quota amount already consumed.
        used: u64,
        /// Total quota allowance.
        total: u64,
    },
    /// Wraps a limiteron `LimiteronError`.
    #[error("Limiteron error: {0}")]
    Limiteron(#[from] limiteron::LimiteronError),
}
