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
///
/// `cache_state` 写入 `x-cache` 响应头：命中缓存为 `"HIT"`，回源后首次写回为
/// `"MISS"`——两者不得混用，否则冷启动首个请求也会对客户端谎报命中（并使
/// 基于该头的命中率统计失真）。
fn deserialize_response(status: StatusCode, body: Vec<u8>, cache_state: &'static str) -> Response {
    Response::builder()
        .status(status)
        .header("x-cache", cache_state)
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
}

impl ResponseCacheLayer {
    /// Create a new `ResponseCacheLayer`.
    ///
    /// # Arguments
    /// * `cache` — shared sync cache backend
    ///
    /// TTL 承载于缓存条目编码之外（`SyncCache` 契约无逐条 TTL）；条目过期
    /// 由后端策略决定，本层不复制 TTL 配置。
    pub fn new(cache: SharedCache) -> Self {
        Self { cache }
    }
}

impl<S> Layer<S> for ResponseCacheLayer {
    type Service = ResponseCacheMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ResponseCacheMiddleware {
            inner,
            cache: self.cache.clone(),
        }
    }
}

/// Tower `Service` that caches GET responses.
///
/// Generic over the inner service `S`. On GET requests, checks the cache
/// first; on a hit, returns the cached response. On a miss, delegates to
/// the inner service and writes the response back to the cache.
#[derive(Clone)]
pub struct ResponseCacheMiddleware<S> {
    inner: S,
    cache: SharedCache,
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
                return Box::pin(async move { Ok(deserialize_response(status, body, "HIT")) });
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
                    // 本次是回源后首次写回，标 MISS（不得复用 HIT 标签）。
                    return Ok(deserialize_response(status, body, "MISS"));
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
mod middleware_behavior_tests {
    use super::*;
    use crate::cache::OxcacheSyncCache;
    use axum::http::Method;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    /// 内部服务 + 缓存中间件（宏形态：避免 `impl Trait` 返回类型丢失
    /// `S::Future: Send` 等中间件 Service impl 所需的边界）。记录真实回源次数。
    macro_rules! mw {
        ($cache:expr, $hits:expr, $status:expr, $body:expr) => {{
            let counter = Arc::clone(&$hits);
            let inner = tower::service_fn(move |_req: Request<Body>| {
                let c = Arc::clone(&counter);
                let status = $status;
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok::<Response, std::io::Error>(
                        Response::builder()
                            .status(status)
                            .body(Body::from($body))
                            .unwrap(),
                    )
                }
            });
            ResponseCacheLayer::new(Arc::clone(&$cache)).layer(inner)
        }};
    }

    async fn take_body(resp: Response) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn req(method: Method, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// GET 首次 MISS（走 inner）、第二次 HIT（不再走 inner），且两者响应体一致。
    #[tokio::test]
    async fn get_first_miss_then_hit() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let first = mw!(cache, hits, StatusCode::OK, "dynamic")
            .oneshot(req(Method::GET, "/data"))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(take_body(first).await, "dynamic");
        assert_eq!(hits.load(Ordering::SeqCst), 1, "首次必须回源");

        let second = mw!(cache, hits, StatusCode::OK, "dynamic2")
            .oneshot(req(Method::GET, "/data"))
            .await
            .unwrap();
        assert_eq!(
            second
                .headers()
                .get("x-cache")
                .and_then(|v| v.to_str().ok()),
            Some("HIT")
        );
        assert_eq!(
            take_body(second).await,
            "dynamic",
            "HIT 必须返回缓存体而非新响应"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1, "HIT 不得回源");
    }

    /// 首次回源写入缓存时不得伪装成 HIT（否则客户端与命中率指标均失真）。
    #[tokio::test]
    async fn miss_is_labelled_miss_not_hit() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));
        let resp = mw!(cache, hits, StatusCode::OK, "fresh")
            .oneshot(req(Method::GET, "/label"))
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("MISS"),
            "回源写回的响应必须标 MISS"
        );
        assert_eq!(take_body(resp).await, "fresh");
    }

    /// 缓存 key 取 URI（含 query）+ 大小写归一：同资源不同 query 不得串台。
    #[tokio::test]
    async fn query_string_participates_in_cache_key() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));
        mw!(cache, hits, StatusCode::OK, "a1")
            .oneshot(req(Method::GET, "/search?q=1"))
            .await
            .unwrap();
        let resp = mw!(cache, hits, StatusCode::OK, "a2")
            .oneshot(req(Method::GET, "/search?q=2"))
            .await
            .unwrap();
        assert_eq!(take_body(resp).await, "a2", "不同 query 必须各自回源");
        assert_eq!(hits.load(Ordering::SeqCst), 2);

        // 大小写归一：/Users 与 /users 命中同一 entry
        let hits2 = Arc::new(AtomicUsize::new(0));
        mw!(cache, hits2, StatusCode::OK, "u1")
            .oneshot(req(Method::GET, "/Users"))
            .await
            .unwrap();
        let hit = mw!(cache, hits2, StatusCode::OK, "u2")
            .oneshot(req(Method::GET, "/users"))
            .await
            .unwrap();
        assert_eq!(
            hit.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("HIT"),
            "key 已小写归一，/users 应命中 /Users 的缓存"
        );
    }

    #[tokio::test]
    async fn non_get_methods_pass_through_and_never_cache() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));
        for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
            let resp = mw!(cache, hits, StatusCode::OK, "mut")
                .oneshot(req(method.clone(), "/write"))
                .await
                .unwrap();
            assert!(
                resp.headers().get("x-cache").is_none(),
                "{method} 不得经过缓存层"
            );
        }
        assert_eq!(hits.load(Ordering::SeqCst), 4, "非 GET 每次都回源");
        // 缓存内不得出现该 key
        let key = canonicalize_cache_key("/write");
        assert!(cache.get(&key).is_none(), "非 GET 响应不得写入缓存");
    }

    #[tokio::test]
    async fn non_success_response_is_not_cached() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));
        for expect in 1..=2 {
            let resp = mw!(cache, hits, StatusCode::NOT_FOUND, "missing")
                .oneshot(req(Method::GET, "/gone"))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            assert_eq!(
                hits.load(Ordering::SeqCst),
                expect,
                "4xx/5xx 不得缓存，下次仍回源"
            );
        }
    }

    /// 脏缓存条目（长度不足或状态码非法）不得 panic，也不得掩盖真实回源。
    #[tokio::test]
    async fn corrupt_cache_entries_fall_through_to_inner() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        cache.set(&canonicalize_cache_key("/short"), vec![200]);
        cache.set(
            &canonicalize_cache_key("/badstatus"),
            vec![3, 232, b'b'], // 1000 非合法 HTTP 状态码
        );

        for uri in ["/short", "/badstatus"] {
            let hits = Arc::new(AtomicUsize::new(0));
            let resp = mw!(cache, hits, StatusCode::OK, "real")
                .oneshot(req(Method::GET, uri))
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "{uri} 脏条目应回源而非出错");
            assert_eq!(take_body(resp).await, "real");
            assert_eq!(hits.load(Ordering::SeqCst), 1, "{uri} 脏条目必须回源");
        }
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
        let resp = deserialize_response(status, body.clone(), "HIT");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("x-cache").unwrap(), "HIT");
    }

    #[test]
    fn test_response_cache_layer_creation() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let layer = ResponseCacheLayer::new(cache);
        let _service = layer.layer(tower::service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
        }));
    }

    /// 注：本例只验证缓存条目自身的存取编码（状态码大端 + 体），不经过
    /// `ResponseCacheMiddleware`；中间件的命中/回源行为由同文件
    /// `middleware_behavior_tests` 真实驱动验证。
    #[tokio::test]
    async fn cached_entry_roundtrips_through_store() {
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
