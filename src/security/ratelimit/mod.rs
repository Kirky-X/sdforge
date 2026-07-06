// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Rate limiting module backed by limiteron 0.2.1.
//!
//! Provides a unified rate limiting abstraction (`RateLimiter` trait) and a
//! concrete adapter (`LimiteronAdapter`) wrapping `limiteron::Governor`.
//! Includes a Tower middleware for HTTP request rate limiting.
//!
//! Requires the `ratelimit` feature.

use std::future::Future;
use std::pin::Pin;

use axum::body::Body;
use axum::http::Request;

// Concrete adapter wrapping `limiteron::Governor`.
mod adapter;
pub use adapter::{LimiteronAdapter, LimiteronAdapterBuilder};

// Tower middleware for HTTP rate limiting.
mod middleware;
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
pub trait RateLimiter: Send + Sync {
    /// Check whether the given identifier (e.g. IP, user ID) is allowed.
    ///
    /// Returns `Ok(())` if the request is allowed, or a `RateLimitError`
    /// describing the rejection reason.
    fn check<'a>(
        &'a self,
        identifier: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>>;

    /// Extract the identifier from an HTTP request and check it.
    ///
    /// Default implementors should extract the client IP (or other identifier)
    /// and delegate to `check`.
    fn check_request<'a>(
        &'a self,
        req: &'a Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>>;
}

/// Rate limiting error variants aligned with limiteron's `Decision` types.
///
/// `Display` and `std::error::Error` impls are provided by `thiserror::Error`
/// derive. `From<limiteron::FlowGuardError>` is auto-derived via `#[from]`.
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
    /// Wraps a limiteron `FlowGuardError`.
    #[error("Limiteron error: {0}")]
    Limiteron(#[from] limiteron::FlowGuardError),
}
