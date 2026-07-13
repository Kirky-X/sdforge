// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tower middleware for HTTP rate limiting.
//!
//! Provides `RateLimitLayer` (a Tower `Layer`) and `RateLimitMiddleware<S>` (a
//! Tower `Service<Request<Body>>`). The middleware delegates identifier
//! extraction + check to an injected `Arc<dyn HttpRequestRateLimiter>`. On
//! rejection the middleware short-circuits with `StatusCode::TOO_MANY_REQUESTS`
//! (429); on approval the request is forwarded to the inner service unchanged.
//!
//! See `design.md` D4 for the rationale (returns `BoxFuture`, `.await` in
//! async block, mirrors Tower ecosystem pattern).
//!
//! Requires the `ratelimit-http` feature.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::{Layer, Service};

use super::{HttpRequestRateLimiter, RateLimitError};

/// Type alias for the boxed future returned by `Service::call`.
///
/// `'static` because the future owns all data it captures (the limiter is
/// `Arc`-cloned, the inner service is cloned, and the request is moved in).
type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

/// Tower `Layer` that wraps an inner service with rate-limit enforcement.
///
/// Holds an `Arc<dyn HttpRequestRateLimiter>` so the same limiter can be
/// shared across multiple routes/services. Cloning is cheap (just bumps the
/// `Arc` refcount).
pub struct RateLimitLayer {
    limiter: Arc<dyn HttpRequestRateLimiter>,
}

impl RateLimitLayer {
    /// Construct a new `RateLimitLayer` from any `HttpRequestRateLimiter`.
    ///
    /// The limiter is shared (via `Arc`) across every service produced by
    /// [`Layer::layer`].
    pub fn new(limiter: Arc<dyn HttpRequestRateLimiter>) -> Self {
        Self { limiter }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// Tower `Service` that enforces rate limiting before forwarding the request.
///
/// Generic over the inner service `S`. `S` must implement
/// `Service<Request<Body>, Response = Response>` and be `Clone + Send + 'static`
/// (Tower's standard `Service::call` requires `&mut self`, and we clone the
/// inner service into the async block so the future is `'static`).
#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    limiter: Arc<dyn HttpRequestRateLimiter>,
}

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
    S::Error: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Clone the limiter Arc and the inner service so the future is
        // `'static` and `Send`. `&mut self` cannot be held across the
        // `.await` boundary, so we move owned clones into the async block.
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // `HttpRequestRateLimiter::check_request` returns a `BoxFuture`,
            // so we `.await` it directly. On rejection we short-circuit with a
            // variant-specific response and never touch the inner service.
            match limiter.check_request(&req).await {
                Ok(()) => inner.call(req).await,
                Err(err) => Ok(rate_limit_rejection_response(err)),
            }
        })
    }
}

/// Build an HTTP rejection response tailored to the specific `RateLimitError`
/// variant.
///
/// Variant → status code mapping (follows common reverse-proxy semantics):
///
/// | Variant          | Status | Retry-After | Rationale                                  |
/// |------------------|--------|-------------|--------------------------------------------|
/// | `Exceeded`       | 429    | yes         | Transient rate limit; client should wait.  |
/// | `Banned`         | 403    | no          | Longer-term block; not a transient limit.  |
/// | `CircuitOpen`    | 503    | yes         | Backend protection; caller should back off.|
/// | `QuotaExhausted` | 429    | no          | Quota window boundary unknown to variant.  |
/// | `Limiteron(_)`   | 500    | no          | Internal limiteron error; not retryable.   |
///
/// `Retry-After` is emitted in seconds (RFC 7231 §7.1.3 allows either seconds
/// or HTTP-date; seconds is simpler and avoids clock-skew issues).
fn rate_limit_rejection_response(err: RateLimitError) -> Response {
    let (status, body, retry_after_secs) = match &err {
        RateLimitError::Exceeded { window_seconds, .. } => (
            StatusCode::TOO_MANY_REQUESTS,
            "Rate limit exceeded".to_string(),
            Some(*window_seconds),
        ),
        RateLimitError::Banned { reason } => {
            (StatusCode::FORBIDDEN, format!("Banned: {}", reason), None)
        }
        RateLimitError::CircuitOpen => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Circuit breaker open".to_string(),
            // No window boundary available; suggest a conservative back-off
            // so callers don't hammer the breaker while it's half-open.
            Some(60),
        ),
        RateLimitError::QuotaExhausted { .. } => (
            StatusCode::TOO_MANY_REQUESTS,
            "Quota exhausted".to_string(),
            None,
        ),
        RateLimitError::Limiteron(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal rate limit error".to_string(),
            None,
        ),
    };

    let mut resp = Response::new(Body::from(body));
    *resp.status_mut() = status;
    if let Some(secs) = retry_after_secs {
        // `secs.to_string()` is a valid HTTP header value (digits only).
        if let Ok(value) = secs.to_string().parse() {
            resp.headers_mut().insert("Retry-After", value);
        }
    }
    resp
}
