// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Tests for `RateLimitLayer` / `RateLimitMiddleware` (Tower middleware).
//!
//! These tests exercise the middleware behavior using a mock `RateLimiter`
//! (no `limiteron::Governor` involved). The middleware must:
//!
//! 1. Reject with `StatusCode::TOO_MANY_REQUESTS` (429) when the limiter
//!    returns `Err`.
//! 2. Forward the request to the inner service when the limiter returns `Ok`.
//!

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::{Layer, Service};

use crate::security::{HttpRequestRateLimiter, RateLimitLayer, RateLimitMiddleware};
use crate::security::{RateLimitError, RateLimiter};

// ============================================================================
// Mocks
// ============================================================================

/// Mock `RateLimiter` whose `check` / `check_request` always return a
/// pre-configured outcome.
///
/// Holds only a `should_reject: bool` because `RateLimitError` does not
/// implement `Clone`. On rejection we emit a fixed
/// `RateLimitError::Exceeded { limit: 100, window_seconds: 60 }` (the value
/// mandated by the middleware contract).
struct MockLimiter {
    should_reject: bool,
}

impl RateLimiter for MockLimiter {
    fn check<'a>(
        &'a self,
        _identifier: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        let should_reject = self.should_reject;
        Box::pin(async move {
            if should_reject {
                Err(RateLimitError::Exceeded {
                    limit: 100,
                    window_seconds: 60,
                })
            } else {
                Ok(())
            }
        })
    }
}

impl HttpRequestRateLimiter for MockLimiter {
    fn check_request<'a>(
        &'a self,
        _req: &'a Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        let should_reject = self.should_reject;
        Box::pin(async move {
            if should_reject {
                Err(RateLimitError::Exceeded {
                    limit: 100,
                    window_seconds: 60,
                })
            } else {
                Ok(())
            }
        })
    }
}

/// A minimal `Clone` echo `Service` that returns `200 OK` with body `"ok"`.
///
/// We implement our own struct (instead of `tower::service_fn`) because
/// `service_fn`-based services are not `Clone`, and our `Service::call`
/// implementation clones the inner service into the async block.
#[derive(Clone)]
struct EchoService;

impl Service<Request<Body>> for EchoService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response, Infallible>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Request<Body>) -> Self::Future {
        Box::pin(async move {
            let mut resp = Response::new(Body::from("ok"));
            *resp.status_mut() = StatusCode::OK;
            Ok(resp)
        })
    }
}

// ============================================================================
// RateLimitMiddleware behavior tests (limiter reject/allow paths)
// ============================================================================

/// When the limiter rejects (returns `Err`), the middleware must short-circuit
/// with `StatusCode::TOO_MANY_REQUESTS` (429) and NOT call the inner service.
///
/// This is the canonical acceptance test for the reject path.
#[tokio::test]
async fn middleware_returns_429_when_limiter_rejects() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiter {
        should_reject: true,
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware
        .call(req)
        .await
        .expect("middleware call must not surface service error");

    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "rejected request must yield 429, got {}",
        response.status()
    );
}

/// When the limiter approves (returns `Ok`), the middleware must forward the
/// request to the inner service unchanged.
///
/// This test is the second acceptance criterion (approve path forwards unchanged).
#[tokio::test]
async fn middleware_forwards_request_when_limiter_approves() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiter {
        should_reject: false,
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware
        .call(req)
        .await
        .expect("middleware call must not surface service error");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "approved request must pass through to inner service, got {}",
        response.status()
    );
}

/// `RateLimitLayer::new` must accept `Arc<dyn HttpRequestRateLimiter>` without panic, and
/// the produced `Layer` must produce a `RateLimitMiddleware` from any inner
/// service.
#[tokio::test]
async fn layer_construction_accepts_arcrate_limiter() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiter {
        should_reject: false,
    });
    let _layer = RateLimitLayer::new(limiter);
    // No panic, no assertion needed beyond reaching this point.
}

// ============================================================================
// regression: 429 response must differentiate error variants and emit
// `Retry-After` where applicable. Before the fix, every rejection collapsed
// to a generic 429 with body "Rate limit exceeded" and no `Retry-After`.
// ============================================================================

/// Mock limiter that produces a fresh `RateLimitError` on each check via a
/// factory closure. `RateLimitError` does not implement `Clone` (and
/// `LimiteronError` doesn't either), so we can't hold a single owned copy
/// and clone it per call. The factory closure lets each test mint a new
/// owned error of the desired variant.
struct MockLimiterWithError {
    factory: Box<dyn Fn() -> RateLimitError + Send + Sync>,
}

impl RateLimiter for MockLimiterWithError {
    fn check<'a>(
        &'a self,
        _identifier: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        Box::pin(async move { Err((self.factory)()) })
    }
}

impl HttpRequestRateLimiter for MockLimiterWithError {
    fn check_request<'a>(
        &'a self,
        _req: &'a Request<Body>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
        Box::pin(async move { Err((self.factory)()) })
    }
}

/// Exceeded → 429 + `Retry-After: <window_seconds>` header.
#[tokio::test]
async fn middleware_exceeded_returns_429_with_retry_after() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiterWithError {
        factory: Box::new(|| RateLimitError::Exceeded {
            limit: 100,
            window_seconds: 60,
        }),
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware.call(req).await.expect("call must succeed");
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after = response
        .headers()
        .get("Retry-After")
        .expect("Retry-After header must be present on Exceeded rejections");
    assert_eq!(retry_after.to_str().unwrap(), "60");
}

/// Banned → 403 Forbidden (not 429) + no `Retry-After`.
#[tokio::test]
async fn middleware_banned_returns_403_without_retry_after() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiterWithError {
        factory: Box::new(|| RateLimitError::Banned {
            reason: "abuse".to_string(),
        }),
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware.call(req).await.expect("call must succeed");
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Banned must map to 403 (longer-term block, not transient rate limit)"
    );
    assert!(
        response.headers().get("Retry-After").is_none(),
        "Banned must NOT emit Retry-After (no automatic retry semantics)"
    );
}

/// CircuitOpen → 503 Service Unavailable + `Retry-After` header.
#[tokio::test]
async fn middleware_circuit_open_returns_503_with_retry_after() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiterWithError {
        factory: Box::new(|| RateLimitError::CircuitOpen),
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware.call(req).await.expect("call must succeed");
    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "CircuitOpen must map to 503 (transient backend unavailability)"
    );
    assert!(
        response.headers().get("Retry-After").is_some(),
        "CircuitOpen must emit Retry-After (caller should back off)"
    );
}

/// QuotaExhausted → 429 (still a rate limit) + no `Retry-After` (window
/// boundary is not exposed by the error variant).
#[tokio::test]
async fn middleware_quota_exhausted_returns_429_without_retry_after() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiterWithError {
        factory: Box::new(|| RateLimitError::QuotaExhausted {
            used: 100,
            total: 100,
        }),
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware.call(req).await.expect("call must succeed");
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(
        response.headers().get("Retry-After").is_none(),
        "QuotaExhausted does not carry a window_seconds → no Retry-After"
    );
}

/// Limiteron (internal error) → 500 Internal Server Error.
#[tokio::test]
async fn middleware_limiteron_error_returns_500() {
    use limiteron::LimiteronError;
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiterWithError {
        factory: Box::new(|| {
            RateLimitError::Limiteron(LimiteronError::ConfigError("internal".to_string()))
        }),
    });
    let layer = RateLimitLayer::new(limiter);
    let mut middleware: RateLimitMiddleware<EchoService> = layer.layer(EchoService);

    let req = Request::builder()
        .body(Body::empty())
        .expect("request build");

    let response = middleware.call(req).await.expect("call must succeed");
    assert_eq!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "Limiteron internal errors must surface as 500, not 429"
    );
    assert!(
        response.headers().get("Retry-After").is_none(),
        "Internal errors must not suggest a retry cadence"
    );
}

// ============================================================================
// Layer Clone regression (axum Router::layer requires `L: Layer + Clone`)
// ============================================================================

/// `RateLimitLayer` must be `Clone` so it can mount directly via
/// `axum::Router::layer` (which bounds the layer by `Clone`).
#[test]
fn rate_limit_layer_is_clone() {
    let limiter: Arc<dyn HttpRequestRateLimiter> = Arc::new(MockLimiter {
        should_reject: false,
    });
    let layer = RateLimitLayer::new(limiter);
    let clone = layer.clone();

    // Both layer instances must produce working middleware from the same
    // shared limiter (Arc refcount semantics, not a structural copy).
    let a: RateLimitMiddleware<EchoService> = layer.layer(EchoService);
    let b: RateLimitMiddleware<EchoService> = clone.layer(EchoService);
    let _ = (a, b);

    // Compile-time Clone bound: exactly what `Router::layer` requires.
    fn assert_clone<T: Clone>(_: &T) {}
    assert_clone(&layer);
}

/// Cloned layers enforce a SHARED limit: exhausting the limit through a
/// service built from the original layer also rejects through a service
/// built from the clone (proves the Arc is shared, not duplicated).
#[tokio::test]
async fn cloned_layer_shares_limiter_state() {
    #[derive(Clone, Default)]
    struct OnceLimiter {
        remaining: Arc<std::sync::atomic::AtomicU64>,
    }

    impl RateLimiter for OnceLimiter {
        fn check<'a>(
            &'a self,
            _identifier: &'a str,
        ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
            Box::pin(async move {
                if self
                    .remaining
                    .fetch_sub(1, std::sync::atomic::Ordering::SeqCst)
                    > 1
                {
                    Ok(())
                } else {
                    Err(RateLimitError::Exceeded {
                        limit: 1,
                        window_seconds: 60,
                    })
                }
            })
        }
    }

    impl HttpRequestRateLimiter for OnceLimiter {
        fn check_request<'a>(
            &'a self,
            _req: &'a Request<Body>,
        ) -> Pin<Box<dyn Future<Output = Result<(), RateLimitError>> + Send + 'a>> {
            Box::pin(async move { self.check("test").await })
        }
    }

    let limiter = OnceLimiter {
        remaining: Arc::new(std::sync::atomic::AtomicU64::new(2)),
    };

    let original = RateLimitLayer::new(Arc::new(limiter.clone()));
    let cloned = original.clone();

    let mut via_original: RateLimitMiddleware<EchoService> = original.layer(EchoService);
    let mut via_cloned: RateLimitMiddleware<EchoService> = cloned.layer(EchoService);

    let req = || {
        Request::builder()
            .body(Body::empty())
            .expect("request build")
    };

    // First request (via original): consumes the shared budget → allowed.
    let resp = via_original.call(req()).await.expect("call must succeed");
    assert_eq!(resp.status(), StatusCode::OK);
    // Second request (via CLONE): shared budget now empty → 429.
    let resp = via_cloned.call(req()).await.expect("call must succeed");
    assert_eq!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "clone must share the limiter state (Arc), not duplicate it"
    );
}
