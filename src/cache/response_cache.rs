// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! HTTP response cache middleware.
//!
//! Provides `ResponseCacheLayer` (a Tower `Layer`) and `ResponseCacheMiddleware<S>`
//! (a Tower `Service`). The middleware caches GET responses in a `SyncCache`
//! backend keyed by canonicalized URI. Cache hits short-circuit the inner
//! service; misses call through and write back.
//!
//! Requires both `cache` and `http` features.

use std::future::Future;
use std::pin::Pin;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::{Layer, Service};

use super::{SharedCache, canonicalize_cache_key};

/// Type alias for boxed futures emitted by `Service::call`.
type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

/// Serialize a `Response` body into `Vec<u8>` for cache storage.
///
/// Format: `[status_u16 big-endian][body bytes]`.
/// Headers are NOT cached — only status + body. This keeps the cache
/// entry small and avoids caching hop-by-hop headers.
async fn serialize_response(resp: Response) -> Option<(StatusCode, Vec<u8>)> {
    let status = resp.status();
    let body_bytes = match axum::body::to_bytes(resp.into_body(), usize::MAX).await {
        Ok(b) => b,
        Err(_) => return None,
    };
    Some((status, body_bytes.to_vec()))
}

/// Reconstruct a `Response` from cached `(StatusCode, Vec<u8>)`.
fn deserialize_response(status: StatusCode, body: Vec<u8>) -> Response {
    Response::builder()
        .status(status)
        .header("x-cache", "HIT")
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("cache deserialization error"))
                .unwrap()
        })
}

/// Build a cache key from the request URI.
///
/// Uses `canonicalize_cache_key` (lowercase + trim) so that
/// `/Users` and `/users` map to the same entry.
fn cache_key_from_request(req: &Request<Body>) -> String {
    let uri = req.uri().to_string();
    canonicalize_cache_key(&uri)
}

/// Tower `Layer` that wraps an inner service with response caching.
///
/// Only GET requests are cached. All other methods pass through to the
/// inner service without cache interaction.
pub struct ResponseCacheLayer {
    cache: SharedCache,
    ttl_secs: u64,
}

impl ResponseCacheLayer {
    /// Create a new `ResponseCacheLayer`.
    ///
    /// # Arguments
    /// * `cache` — shared sync cache backend
    /// * `ttl_secs` — TTL hint (informational; the `SyncCache` trait does not
    ///   enforce TTL natively, but the value is available for future use or
    ///   for backends that support it)
    pub fn new(cache: SharedCache, ttl_secs: u64) -> Self {
        Self { cache, ttl_secs }
    }
}

impl<S> Layer<S> for ResponseCacheLayer {
    type Service = ResponseCacheMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ResponseCacheMiddleware {
            inner,
            cache: self.cache.clone(),
            ttl_secs: self.ttl_secs,
        }
    }
}

/// Tower `Service` that caches GET responses.
///
/// Generic over the inner service `S`. On GET requests, checks the cache
/// first; on a hit, returns the cached response. On a miss, delegates to
/// the inner service and writes the response back to the cache.
#[derive(Clone)]
#[expect(
    dead_code,
    reason = "TTL 目前仅信息性存储，SyncCache 不消费；为后端集成预留"
)]
pub struct ResponseCacheMiddleware<S> {
    inner: S,
    cache: SharedCache,
    ttl_secs: u64,
}

impl<S> Service<Request<Body>> for ResponseCacheMiddleware<S>
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
        // Only cache GET requests
        if req.method() != axum::http::Method::GET {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        let key = cache_key_from_request(&req);

        // Cache lookup (synchronous)
        if let Some(cached_bytes) = self.cache.get(&key)
            && cached_bytes.len() >= 2
        {
            let status_code = u16::from_be_bytes([cached_bytes[0], cached_bytes[1]]);
            if let Ok(status) = StatusCode::from_u16(status_code) {
                let body = cached_bytes[2..].to_vec();
                return Box::pin(async move { Ok(deserialize_response(status, body)) });
            }
        }

        // Cache miss — call inner service and write back
        let mut inner = self.inner.clone();
        let cache = self.cache.clone();
        Box::pin(async move {
            let resp = inner.call(req).await?;

            // Only cache successful responses
            if resp.status().is_success() {
                if let Some((status, body)) = serialize_response(resp).await {
                    let mut store = Vec::with_capacity(2 + body.len());
                    store.extend_from_slice(&status.as_u16().to_be_bytes());
                    store.extend_from_slice(&body);
                    cache.set(&key, store);
                    return Ok(deserialize_response(status, body));
                }
                // serialize_response failed (body too large?) — return 500
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("response cache serialization failed"))
                    .unwrap());
            }

            Ok(resp)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::OxcacheSyncCache;
    use std::sync::Arc;

    #[test]
    fn test_cache_key_from_request_normalizes() {
        let req = Request::builder()
            .uri("/Users/123")
            .body(Body::empty())
            .unwrap();
        let key = cache_key_from_request(&req);
        assert_eq!(key, "/users/123");
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let status = StatusCode::OK;
        let body = b"hello world".to_vec();
        let resp = deserialize_response(status, body.clone());
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("x-cache").unwrap(), "HIT");
    }

    #[test]
    fn test_response_cache_layer_creation() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let layer = ResponseCacheLayer::new(cache, 300);
        assert_eq!(layer.ttl_secs, 300);
    }

    #[tokio::test]
    async fn test_cache_hit_returns_cached_response() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        // Pre-populate cache with a synthetic entry
        let key = canonicalize_cache_key("/test");
        let status = StatusCode::OK.as_u16().to_be_bytes();
        let mut entry = status.to_vec();
        entry.extend_from_slice(b"cached body");
        cache.set(&key, entry);

        // Verify cache lookup
        let cached = cache.get(&key).unwrap();
        let status_code = u16::from_be_bytes([cached[0], cached[1]]);
        assert_eq!(status_code, 200);
        assert_eq!(&cached[2..], b"cached body");
    }
}
