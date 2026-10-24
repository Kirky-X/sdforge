// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! HTTP response cache middleware.
//!
//! Provides `ResponseCacheLayer` (a Tower `Layer`) and `ResponseCacheMiddleware<S>`
//! (a Tower `Service`). The middleware caches GET responses in a `SyncCache`
//! backend keyed by the canonicalized path plus the verbatim query string.
//! Cache hits short-circuit the inner service; misses call through and write back.
//!
//! # Security
//!
//! - Requests carrying credentials bypass the cache entirely: the cache key
//!   has no identity dimension, so a shared entry would hand one caller's
//!   response body to another caller. The recognized credential header names
//!   are configurable ([`ResponseCacheLayer::with_credential_header_names`]);
//!   the default set plus the header name declared by the active
//!   [`crate::config::AuthConfig`] covers API keys with custom header names.
//! - Only the path is case-normalized; query values stay byte-exact because
//!   they are usually case-sensitive.
//! - Responses larger than the cacheable limit are streamed through
//!   untouched, not cached.
//! - Responses carrying `Set-Cookie` are never cached: the cookie belongs to
//!   the caller that received it and must not be replayed to anyone else.
//!
//! Requires both `cache` and `http` features.

use std::future::Future;
use std::pin::Pin;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode};
use axum::response::Response;
use tower::{Layer, Service};

use super::{SharedCache, canonicalize_cache_key};

/// Type alias for boxed futures emitted by `Service::call`.
type BoxFuture<T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + Send>>;

/// Default ceiling for a cacheable response body (1 MiB).
///
/// Larger responses are passed through untouched instead of being buffered
/// into the cache — the backend limits entry count, not bytes.
pub const DEFAULT_MAX_CACHEABLE_BODY_BYTES: usize = 1024 * 1024;

/// Cache entry format marker (v2: status + response headers + body).
///
/// Entries written before this marker existed carry only `[status][body]`;
/// they fail the marker check and are treated as unusable (the request falls
/// through to the origin, which rewrites the entry in the current format).
const ENTRY_MAGIC: [u8; 4] = *b"SDF2";

/// Outcome of buffering a response body for caching.
enum BufferedBody {
    /// Body buffered and within the cacheable limit.
    Cacheable(StatusCode, HeaderMap, Bytes),
    /// Body exceeds the cacheable limit — served as-is (streamed), not cached.
    TooLarge(Response),
    /// Body stream failed mid-read — the body cannot be restored.
    Unreadable,
}

/// True when a response header may be stored in the cache entry and replayed.
///
/// Hop-by-hop headers are per-connection and must never be replayed;
/// `content-length` is recomputed by the server from the actual body;
/// `set-cookie` is caller-private (such responses are not cached at all);
/// `x-cache` is written by this layer.
fn is_replayable_header(name: &HeaderName) -> bool {
    !matches!(
        name.as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
            | "set-cookie"
            | "x-cache"
    )
}

/// The subset of response headers that may be stored in the cache entry and
/// replayed to the client.
///
/// Both the cache write-back (MISS) and the cache read (HIT) use this set, so
/// the two paths deliver the same header set.
fn replayable_headers(headers: &HeaderMap) -> HeaderMap {
    headers
        .iter()
        .filter(|(name, _)| is_replayable_header(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// Buffer a `Response` body for cache storage.
///
/// The body is read incrementally and only as far as the cacheable limit.
/// When the limit is exceeded the response is **not** truncated and **not**
/// turned into an error: the bytes already read are handed back as the head
/// of a stream that continues with the untouched remainder, so the client
/// receives the complete response. A declared `Content-Length` above the
/// limit short-circuits the buffering altogether.
async fn buffer_response(resp: Response, max_cacheable_bytes: usize) -> BufferedBody {
    let status = resp.status();
    let (parts, body) = resp.into_parts();

    // Declared length already exceeds the limit: never buffer, stream through.
    if let Some(len) = parts
        .headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        && len > max_cacheable_bytes as u64
    {
        return BufferedBody::TooLarge(Response::from_parts(parts, body));
    }

    let mut stream = body.into_data_stream();
    let mut buffered: Vec<u8> = Vec::new();
    loop {
        let Some(chunk) = futures_util::StreamExt::next(&mut stream).await else {
            break;
        };
        let bytes = match chunk {
            Ok(bytes) => bytes,
            Err(_) => return BufferedBody::Unreadable,
        };
        buffered.extend_from_slice(&bytes);
        if buffered.len() > max_cacheable_bytes {
            // Over the limit: hand the buffered prefix back as the head of a
            // stream that continues with the remaining frames.
            let prefix = Bytes::from(buffered);
            let rest = stream;
            let combined = futures_util::stream::unfold((Some(prefix), rest), |state| async move {
                let (mut prefix, mut rest) = state;
                if let Some(head) = prefix.take() {
                    return Some((Ok(head), (None, rest)));
                }
                let item = futures_util::StreamExt::next(&mut rest).await;
                item.map(|item| (item, (None, rest)))
            });
            return BufferedBody::TooLarge(Response::from_parts(
                parts,
                Body::from_stream(combined),
            ));
        }
    }

    BufferedBody::Cacheable(
        status,
        replayable_headers(&parts.headers),
        Bytes::from(buffered),
    )
}

/// Serialize a cache entry: `[status][magic][headers][body]`.
///
/// Header names and values are length-prefixed so that the entry is
/// self-describing and lossless for non-UTF-8 header values.
fn encode_entry(status: StatusCode, headers: &HeaderMap, body: &Bytes) -> Vec<u8> {
    let kept: Vec<(&HeaderName, &HeaderValue)> = headers
        .iter()
        .filter(|(name, _)| is_replayable_header(name))
        .collect();
    let mut out = Vec::with_capacity(2 + ENTRY_MAGIC.len() + 2 + body.len());
    out.extend_from_slice(&status.as_u16().to_be_bytes());
    out.extend_from_slice(&ENTRY_MAGIC);
    out.extend_from_slice(&(kept.len() as u16).to_be_bytes());
    for (name, value) in kept {
        let name_bytes = name.as_str().as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&(value.len() as u32).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    out.extend_from_slice(body);
    out
}

/// Parse a cache entry written by [`encode_entry`].
///
/// Returns `None` for entries of an unknown format or with a malformed
/// header block — the caller then falls through to the origin instead of
/// serving a partially decoded entry.
fn decode_entry(raw: &[u8]) -> Option<(StatusCode, HeaderMap, Bytes)> {
    let head = 2 + ENTRY_MAGIC.len();
    if raw.len() < head + 2 {
        return None;
    }
    let status = StatusCode::from_u16(u16::from_be_bytes([raw[0], raw[1]])).ok()?;
    if raw[2..head] != ENTRY_MAGIC {
        return None;
    }
    let mut pos = head;
    let header_count = u16::from_be_bytes([raw[pos], raw[pos + 1]]) as usize;
    pos += 2;
    let mut headers = HeaderMap::new();
    for _ in 0..header_count {
        let name_len = read_u16(raw, &mut pos)? as usize;
        let name = std::str::from_utf8(raw.get(pos..pos + name_len)?).ok()?;
        pos += name_len;
        let value_len = read_u32(raw, &mut pos)? as usize;
        let value = raw.get(pos..pos + value_len)?;
        pos += value_len;
        headers.append(
            HeaderName::from_bytes(name.as_bytes()).ok()?,
            HeaderValue::from_bytes(value).ok()?,
        );
    }
    Some((status, headers, Bytes::from(raw[pos..].to_vec())))
}

/// Read a big-endian `u16` at `pos`, advancing it.
fn read_u16(raw: &[u8], pos: &mut usize) -> Option<u16> {
    let bytes = raw.get(*pos..*pos + 2)?;
    *pos += 2;
    Some(u16::from_be_bytes([bytes[0], bytes[1]]))
}

/// Read a big-endian `u32` at `pos`, advancing it.
fn read_u32(raw: &[u8], pos: &mut usize) -> Option<u32> {
    let bytes = raw.get(*pos..*pos + 4)?;
    *pos += 4;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Reconstruct a `Response` from a cached `(StatusCode, headers, body)`.
///
/// The original response headers are restored (content-type and friends), so
/// a hit is indistinguishable from the origin response apart from the cache
/// marker.
///
/// `cache_state` 写入 `x-cache` 响应头：命中缓存为 `"HIT"`，回源后首次写回为
/// `"MISS"`——两者不得混用，否则冷启动首个请求也会对客户端谎报命中（并使
/// 基于该头的命中率统计失真）。
fn deserialize_response(
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
    cache_state: &'static str,
) -> Response {
    let mut response = Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("cache deserialization error"))
                .unwrap()
        });
    for (name, value) in headers.iter() {
        response.headers_mut().append(name.clone(), value.clone());
    }
    response.headers_mut().insert(
        HeaderName::from_static("x-cache"),
        HeaderValue::from_static(cache_state),
    );
    response
}

/// Build a cache key from the request URI.
///
/// The path is normalized (lowercase + trim) so that `/Users` and `/users`
/// map to the same entry; the query string is kept byte-exact because query
/// values are usually case-sensitive (`?ID=1` and `?id=1` are different
/// requests and must not share one entry).
fn cache_key_from_request(req: &Request<Body>) -> String {
    let uri = req.uri();
    let path = canonicalize_cache_key(uri.path());
    match uri.query() {
        Some(query) => format!("{path}?{query}"),
        None => path,
    }
}

/// True when the request carries credentials the cache key cannot express.
///
/// The cache key contains no identity dimension, so a response produced for
/// one caller must never be replayed to another one.
fn is_credentialed(req: &Request<Body>, credential_headers: &[HeaderName]) -> bool {
    credential_headers
        .iter()
        .any(|name| req.headers().contains_key(name))
}

/// Tower `Layer` that wraps an inner service with response caching.
///
/// Only GET requests are cached. All other methods pass through to the
/// inner service without cache interaction.
pub struct ResponseCacheLayer {
    cache: SharedCache,
    max_cacheable_body_bytes: usize,
    credential_headers: Vec<HeaderName>,
}

impl ResponseCacheLayer {
    /// Create a new `ResponseCacheLayer`.
    ///
    /// # Arguments
    /// * `cache` — shared sync cache backend
    ///
    /// TTL 承载于缓存条目编码之外（`SyncCache` 契约无逐条 TTL）；条目过期
    /// 由后端策略决定，本层不复制 TTL 配置。
    ///
    /// 响应体上限取 [`DEFAULT_MAX_CACHEABLE_BODY_BYTES`]（1 MiB）：超过该
    /// 大小的响应原样透传，不进缓存。
    ///
    /// 凭据头名取 [`crate::config::DEFAULT_CREDENTIAL_HEADER_NAMES`]；用
    /// 自定义头名做 API key 鉴权时必须经
    /// [`Self::with_credential_header_names`] 补齐，否则带凭据的响应会被
    /// 按 URL 共享给全体调用者。
    pub fn new(cache: SharedCache) -> Self {
        Self::with_credential_header_names(cache, crate::config::DEFAULT_CREDENTIAL_HEADER_NAMES)
    }

    /// Create a `ResponseCacheLayer` with an explicit cacheable body limit.
    ///
    /// # Arguments
    /// * `cache` — shared sync cache backend
    /// * `max_cacheable_body_bytes` — responses larger than this are served
    ///   as-is instead of being buffered into the cache
    #[must_use]
    pub fn with_max_cacheable_body_bytes(
        cache: SharedCache,
        max_cacheable_body_bytes: usize,
    ) -> Self {
        Self {
            cache,
            max_cacheable_body_bytes,
            credential_headers: default_credential_headers(),
        }
    }

    /// Create a `ResponseCacheLayer` with explicit credential header names.
    ///
    /// # Arguments
    /// * `cache` — shared sync cache backend
    /// * `names` — header names that carry credentials; a request carrying
    ///   any of them bypasses the cache entirely
    ///
    /// # Example
    /// ```ignore
    /// use sdforge::cache::ResponseCacheLayer;
    /// use sdforge::config::AuthConfig;
    ///
    /// let auth = AuthConfig::ApiKey {
    ///     header_name: "X-Tenant-Key".into(),
    ///     prefix: "tk-".into(),
    ///     keys: vec![],
    /// };
    /// let layer = ResponseCacheLayer::with_credential_header_names(
    ///     cache,
    ///     auth.credential_header_names(),
    /// );
    /// ```
    #[must_use]
    pub fn with_credential_header_names<I, S>(cache: SharedCache, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            cache,
            max_cacheable_body_bytes: DEFAULT_MAX_CACHEABLE_BODY_BYTES,
            credential_headers: credential_header_names(names),
        }
    }

    /// Override the credential header names (builder form).
    ///
    /// Replaces the current list — use [`AuthConfig::credential_header_names`]
    /// to keep the generic credential headers and add the configured API key
    /// header.
    ///
    /// [`AuthConfig::credential_header_names`]: crate::config::AuthConfig::credential_header_names
    #[must_use]
    pub fn with_credential_headers<I, S>(self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            credential_headers: credential_header_names(names),
            ..self
        }
    }
}

/// The default credential header names, as `HeaderName`s.
fn default_credential_headers() -> Vec<HeaderName> {
    credential_header_names(crate::config::DEFAULT_CREDENTIAL_HEADER_NAMES)
}

/// Normalize credential header names, dropping the ones that are not valid
/// header names (a typo must not silently disable the protection).
fn credential_header_names<I, S>(names: I) -> Vec<HeaderName>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut out: Vec<HeaderName> = Vec::new();
    for name in names {
        let name = name.into();
        if let Ok(header) = HeaderName::from_bytes(name.as_bytes())
            && !out.contains(&header)
        {
            out.push(header);
        }
    }
    out
}

impl<S> Layer<S> for ResponseCacheLayer {
    type Service = ResponseCacheMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ResponseCacheMiddleware {
            inner,
            cache: self.cache.clone(),
            max_cacheable_body_bytes: self.max_cacheable_body_bytes,
            credential_headers: self.credential_headers.clone(),
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
    max_cacheable_body_bytes: usize,
    credential_headers: Vec<HeaderName>,
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

        // 携带凭据的请求不进共享缓存：缓存键不含身份维度，共用条目会把
        // 一个调用方的响应体回放给另一个调用方。
        if is_credentialed(&req, &self.credential_headers) {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        let key = cache_key_from_request(&req);

        // Cache lookup (synchronous)
        if let Some(cached_bytes) = self.cache.get(&key)
            && let Some((status, headers, body)) = decode_entry(&cached_bytes)
        {
            return Box::pin(async move { Ok(deserialize_response(status, headers, body, "HIT")) });
        }

        // Cache miss — call inner service and write back
        let mut inner = self.inner.clone();
        let cache = self.cache.clone();
        let max_cacheable_body_bytes = self.max_cacheable_body_bytes;
        Box::pin(async move {
            let resp = inner.call(req).await?;

            // Only cache successful responses. A response carrying Set-Cookie
            // is caller-private: replaying it would hand one caller's cookie
            // to another, so it is passed through without being cached.
            if resp.status().is_success()
                && !resp.headers().contains_key(axum::http::header::SET_COOKIE)
            {
                match buffer_response(resp, max_cacheable_body_bytes).await {
                    BufferedBody::Cacheable(status, headers, bytes) => {
                        cache.set(&key, encode_entry(status, &headers, &bytes));
                        // 本次是回源后首次写回，标 MISS（不得复用 HIT 标签）。
                        return Ok(deserialize_response(status, headers, bytes, "MISS"));
                    }
                    // 超过可缓存上限：原样透传，不缓存（响应体完整送达）。
                    BufferedBody::TooLarge(resp) => return Ok(resp),
                    // 响应体流中途出错：体已被消费，无法还原。
                    BufferedBody::Unreadable => {
                        return Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("response cache read failed"))
                            .unwrap());
                    }
                }
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
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "the first miss must go to the origin"
        );

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
            "a HIT must return the cached body, not a fresh response"
        );
        assert_eq!(
            hits.load(Ordering::SeqCst),
            1,
            "a HIT must not go to the origin"
        );
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
            "an origin-fill response must be labelled MISS"
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
        assert_eq!(
            take_body(resp).await,
            "a2",
            "different queries must each go to the origin"
        );
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
            "the key is lowercased: /users must hit the /Users cache entry"
        );
    }

    /// query 取值大小写敏感：`?ID=1` 与 `?id=1` 必须是两条独立条目。
    #[tokio::test]
    async fn query_case_difference_does_not_share_entry() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        mw!(cache, hits, StatusCode::OK, "upper")
            .oneshot(req(Method::GET, "/search?ID=1"))
            .await
            .unwrap();
        let resp = mw!(cache, hits, StatusCode::OK, "lower")
            .oneshot(req(Method::GET, "/search?id=1"))
            .await
            .unwrap();

        assert_eq!(
            resp.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("MISS"),
            "a differently-cased query must not hit the cached entry"
        );
        assert_eq!(take_body(resp).await, "lower");
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    /// 携带凭据的请求既不读缓存也不写缓存（缓存键无身份维度）。
    #[tokio::test]
    async fn credentialed_requests_bypass_the_cache() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let credentialed = || {
            let mut r = req(Method::GET, "/private");
            r.headers_mut()
                .insert("authorization", "Bearer token-1".parse().unwrap());
            r
        };
        let key = cache_key_from_request(&credentialed());

        mw!(cache, hits, StatusCode::OK, "secret")
            .oneshot(credentialed())
            .await
            .unwrap();
        let resp = mw!(cache, hits, StatusCode::OK, "secret")
            .oneshot(credentialed())
            .await
            .unwrap();

        assert!(
            resp.headers().get("x-cache").is_none(),
            "a credentialed request must never be served from the cache"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        assert!(
            cache.get(&key).is_none(),
            "a credentialed response must not be written to the cache"
        );
    }

    /// 超过可缓存上限的响应原样透传，不进缓存。
    #[tokio::test]
    async fn oversized_response_is_passed_through_without_caching() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&hits);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("x".repeat(64)))
                        .unwrap(),
                )
            }
        });
        let service =
            ResponseCacheLayer::with_max_cacheable_body_bytes(Arc::clone(&cache), 16).layer(inner);

        let resp = service.oneshot(req(Method::GET, "/big")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(take_body(resp).await, "x".repeat(64));
        assert!(
            cache.get("/big").is_none(),
            "an oversized response must not be cached"
        );
    }

    /// 回归：超过旧硬上限（32 MiB）的大响应不得变成 500——必须完整送达。
    ///
    /// 旧实现在此处缓冲到 32 MiB 上限后丢弃响应体并返回 500，把"能返回的
    /// 大响应"变成"服务端错误"。现在超限响应走流式透传，状态码与响应体
    /// 都保持原样。
    #[tokio::test]
    async fn response_larger_than_the_old_hard_cap_is_still_delivered() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));
        // 33 MiB > 旧硬上限 32 MiB
        let payload = vec![b'x'; 33 * 1024 * 1024];

        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&hits);
            let body = payload.clone();
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(body))
                        .unwrap(),
                )
            }
        });
        let service = ResponseCacheLayer::new(Arc::clone(&cache)).layer(inner);

        let resp = service.oneshot(req(Method::GET, "/huge")).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "a large response must not be turned into a 500"
        );
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            bytes.len(),
            33 * 1024 * 1024,
            "the large response body must arrive complete"
        );
        assert!(
            cache.get("/huge").is_none(),
            "an oversized response must not be cached"
        );
    }

    /// 已声明 Content-Length 超限时完全不缓冲，原样流式透传。
    #[tokio::test]
    async fn declared_oversized_response_is_streamed_through() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&hits);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(axum::http::header::CONTENT_LENGTH, "1048576")
                        .body(Body::from("small but claims to be huge"))
                        .unwrap(),
                )
            }
        });
        let service =
            ResponseCacheLayer::with_max_cacheable_body_bytes(Arc::clone(&cache), 16).layer(inner);

        let resp = service
            .oneshot(req(Method::GET, "/declared"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(take_body(resp).await, "small but claims to be huge");
        assert!(
            cache.get("/declared").is_none(),
            "a response declaring an oversized length must not be cached"
        );
    }

    /// 原响应头必须保留：MISS 与 HIT 都带 content-type（否则客户端误判类型）。
    #[tokio::test]
    async fn response_headers_are_preserved_on_miss_and_hit() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&hits);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(axum::http::header::CONTENT_TYPE, "application/json")
                        .header("x-origin", "v1")
                        .body(Body::from("{}"))
                        .unwrap(),
                )
            }
        });
        let service = ResponseCacheLayer::new(Arc::clone(&cache)).layer(inner);

        let miss = service
            .clone()
            .oneshot(req(Method::GET, "/typed"))
            .await
            .unwrap();
        assert_eq!(
            miss.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json"),
            "the origin response headers must reach the client on a miss"
        );

        let hit = service.oneshot(req(Method::GET, "/typed")).await.unwrap();
        assert_eq!(
            hit.headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json"),
            "a cache hit must restore the original content type"
        );
        assert_eq!(
            hit.headers().get("x-origin").and_then(|v| v.to_str().ok()),
            Some("v1"),
            "a cache hit must restore the original response headers"
        );
        assert_eq!(
            hit.headers().get("x-cache").and_then(|v| v.to_str().ok()),
            Some("HIT")
        );
    }

    /// 逐跳头不得随条目回放（连接级语义，跨请求无意义）。
    #[tokio::test]
    async fn hop_by_hop_headers_are_not_replayed() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&hits);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(axum::http::header::CONNECTION, "close")
                        .body(Body::from("body"))
                        .unwrap(),
                )
            }
        });
        let service = ResponseCacheLayer::new(Arc::clone(&cache)).layer(inner);
        let miss = service
            .clone()
            .oneshot(req(Method::GET, "/hop"))
            .await
            .unwrap();
        assert!(
            miss.headers().get(axum::http::header::CONNECTION).is_none(),
            "hop-by-hop headers must not be echoed on the write-back path either"
        );
        let hit = service.oneshot(req(Method::GET, "/hop")).await.unwrap();
        assert!(
            hit.headers().get(axum::http::header::CONNECTION).is_none(),
            "hop-by-hop headers must not be replayed from the cache"
        );
    }

    /// 带 Set-Cookie 的响应不得进共享缓存（cookie 属调用方私有）。
    #[tokio::test]
    async fn set_cookie_responses_are_not_cached() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let counter = Arc::clone(&hits);
        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&counter);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(axum::http::header::SET_COOKIE, "sid=abc")
                        .body(Body::from("private"))
                        .unwrap(),
                )
            }
        });
        let service = ResponseCacheLayer::new(Arc::clone(&cache)).layer(inner);

        let first = service
            .clone()
            .oneshot(req(Method::GET, "/cookie"))
            .await
            .unwrap();
        assert_eq!(
            first
                .headers()
                .get(axum::http::header::SET_COOKIE)
                .and_then(|v| v.to_str().ok()),
            Some("sid=abc"),
            "the cookie must still reach the caller that triggered it"
        );

        let second = service.oneshot(req(Method::GET, "/cookie")).await.unwrap();
        assert!(
            second.headers().get("x-cache").is_none(),
            "a response carrying Set-Cookie must not be served from the cache"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        assert!(
            cache.get("/cookie").is_none(),
            "a response carrying Set-Cookie must not be written to the cache"
        );
    }

    /// 自定义凭据头名（API key 用 "X-Tenant-Key" 等）必须被识别为带凭据。
    #[tokio::test]
    async fn custom_credential_header_bypasses_the_cache() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let hits = Arc::new(AtomicUsize::new(0));

        let counter = Arc::clone(&hits);
        let inner = tower::service_fn(move |_req: Request<Body>| {
            let c = Arc::clone(&counter);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok::<Response, std::io::Error>(
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from("secret"))
                        .unwrap(),
                )
            }
        });
        let service = ResponseCacheLayer::with_credential_header_names(
            Arc::clone(&cache),
            crate::config::AuthConfig::ApiKey {
                header_name: "X-Tenant-Key".to_string(),
                prefix: "tk-".to_string(),
                keys: vec![],
            }
            .credential_header_names(),
        )
        .layer(inner);

        let mut request = req(Method::GET, "/private");
        request
            .headers_mut()
            .insert("x-tenant-key", "tk-secret".parse().unwrap());
        let resp = service.oneshot(request).await.unwrap();
        assert!(
            resp.headers().get("x-cache").is_none(),
            "a request carrying a custom credential header must bypass the cache"
        );
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(
            cache.get("/private").is_none(),
            "a credentialed response must not be written to the cache"
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
                "{method} must bypass the cache layer"
            );
        }
        assert_eq!(
            hits.load(Ordering::SeqCst),
            4,
            "non-GET requests always go to the origin"
        );
        // The key must not appear in the cache
        let key = canonicalize_cache_key("/write");
        assert!(
            cache.get(&key).is_none(),
            "non-GET responses must not be written to the cache"
        );
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
                "4xx/5xx responses must not be cached; the next call still goes to the origin"
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
            assert_eq!(
                resp.status(),
                StatusCode::OK,
                "a corrupt entry on {uri} must fall through to the origin, not error"
            );
            assert_eq!(take_body(resp).await, "real");
            assert_eq!(
                hits.load(Ordering::SeqCst),
                1,
                "a corrupt entry on {uri} must go to the origin"
            );
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

    /// query 取值大小写敏感：路径归一不得波及 query。
    #[test]
    fn test_cache_key_preserves_query_case() {
        let req = Request::builder()
            .uri("/Search?q=Admin")
            .body(Body::empty())
            .unwrap();
        let key = cache_key_from_request(&req);
        assert_eq!(key, "/search?q=Admin");

        let other = Request::builder()
            .uri("/Search?q=admin")
            .body(Body::empty())
            .unwrap();
        assert_ne!(cache_key_from_request(&other), key);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let status = StatusCode::OK;
        let body = b"hello world".to_vec();
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("text/plain"),
        );
        let resp = deserialize_response(status, headers, Bytes::from(body.clone()), "HIT");
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("x-cache").unwrap(), "HIT");
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "text/plain",
            "the original response headers must survive the roundtrip"
        );
    }

    #[test]
    fn test_response_cache_layer_creation() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        let layer = ResponseCacheLayer::new(cache);
        let _service = layer.layer(tower::service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
        }));
    }

    /// 注：本例只验证缓存条目自身的存取编码（状态码 + 头 + 体），不经过
    /// `ResponseCacheMiddleware`；中间件的命中/回源行为由同文件
    /// `middleware_behavior_tests` 真实驱动验证。
    #[tokio::test]
    async fn cached_entry_roundtrips_through_store() {
        let cache: SharedCache = Arc::new(OxcacheSyncCache::new());
        // Pre-populate cache with a synthetic entry
        let key = canonicalize_cache_key("/test");
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        let entry = encode_entry(
            StatusCode::OK,
            &headers,
            &Bytes::from_static(b"cached body"),
        );

        cache.set(&key, entry);

        let cached = cache.get(&key).unwrap();
        let (status, decoded_headers, body) = decode_entry(&cached).expect("entry must decode");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            decoded_headers.get("content-type").unwrap(),
            "application/json"
        );
        assert_eq!(&body[..], b"cached body");
    }

    /// 旧格式条目（仅状态码 + 体，无格式标记）不得被误解析：解码失败即回源。
    #[test]
    fn legacy_entries_are_not_decoded() {
        let mut legacy = StatusCode::OK.as_u16().to_be_bytes().to_vec();
        legacy.extend_from_slice(b"legacy body");
        assert!(
            decode_entry(&legacy).is_none(),
            "an entry without the format marker must not decode"
        );
    }
}
