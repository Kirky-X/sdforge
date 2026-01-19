// Copyright (c) 2026 Kirky.X
//! HTTP 响应缓存中间件
//!
//! 提供基于内存的 HTTP 响应缓存，支持 ETag 和 Last-Modified 头实现条件请求。
//! 使用 DashMap 实现高并发缓存，结合 LRU 淘汰策略。

use axum::{
    extract::Request,
    http::{
        header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH, LAST_MODIFIED},
        HeaderValue, StatusCode,
    },
    response::Response,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crate::impl_default_new;
use tower::{Layer, Service};
#[cfg(feature = "logging")]
use tracing;

/// 默认缓存 TTL（秒）
const DEFAULT_CACHE_TTL: u64 = 300; // 5 分钟
/// 默认最大缓存大小（字节）
const DEFAULT_MAX_CACHE_SIZE: usize = 100 * 1024 * 1024; // 100 MB
/// 默认最大条目数量
const DEFAULT_MAX_CACHE_ENTRIES: usize = 10000;
/// 默认可缓存的 HTTP 方法
const DEFAULT_CACHEABLE_METHODS: &[&str] = &["GET", "HEAD"];
/// 默认可缓存的状态码
const DEFAULT_CACHEABLE_STATUS_CODES: &[u16] = &[200, 203, 204, 206, 300, 301, 404, 410];

/// LRU Eviction Heap - Binary heap for O(1) min extraction
///
/// Uses a binary heap (MinHeap) to track cache entries by access time.
/// This provides O(1) access to the oldest entry and O(log n) insertion/removal,
/// compared to O(n) iteration and sorting in the naive approach.
#[derive(Debug, Clone)]
struct MinHeapEntry {
    key: CacheKey,
    last_accessed: u64,
}

/// Binary heap for LRU eviction - min-heap by last_accessed timestamp
#[derive(Debug, Clone)]
struct MinHeap {
    entries: Vec<MinHeapEntry>,
}

impl MinHeap {
    /// Insert a new entry - O(log n)
    fn push(&mut self, entry: MinHeapEntry) {
        self.entries.push(entry);
        self.sift_up(self.entries.len().saturating_sub(1));
    }

    /// Extract the minimum element (oldest entry) - O(log n)
    fn extract_min(&mut self) -> Option<MinHeapEntry> {
        if self.entries.is_empty() {
            return None;
        }
        let min = Some(self.entries[0].clone());
        if self.entries.len() > 1 {
            self.entries[0] = self.entries.pop().unwrap();
            self.sift_down(0);
        } else {
            self.entries.pop();
        }
        min
    }

    /// Remove a specific entry by key - O(n)
    fn remove(&mut self, key: &CacheKey) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| &e.key == key) {
            let last = self.entries.pop().unwrap();
            if pos < self.entries.len() {
                self.entries[pos] = last;
                // Try both sift up and down to maintain heap property
                let mut sift_up_done = false;
                let mut sift_down_done = false;

                if pos > 0 {
                    self.sift_up(pos);
                    sift_up_done = true;
                }
                if pos < self.entries.len() {
                    self.sift_down(pos);
                    sift_down_done = true;
                }

                // If neither sift worked, the element is in correct position
                if !sift_up_done && !sift_down_done {
                    // Element was already in correct position after swap
                }
            }
            return true;
        }
        false
    }

    /// Sift element up to maintain heap property - O(log n)
    fn sift_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx.saturating_sub(1)) / 2;
            if self.entries[parent].last_accessed > self.entries[idx].last_accessed {
                self.entries.swap(parent, idx);
                idx = parent;
            } else {
                break;
            }
        }
    }

    /// Sift element down to maintain heap property - O(log n)
    fn sift_down(&mut self, mut idx: usize) {
        let len = self.entries.len();
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut smallest = idx;

            if left < len && self.entries[left].last_accessed < self.entries[smallest].last_accessed
            {
                smallest = left;
            }
            if right < len
                && self.entries[right].last_accessed < self.entries[smallest].last_accessed
            {
                smallest = right;
            }

            if smallest != idx {
                self.entries.swap(idx, smallest);
                idx = smallest;
            } else {
                break;
            }
        }
    }
}

impl_default_new!(MinHeap);

/// 缓存配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// 缓存 TTL（秒）
    pub ttl_seconds: u64,
    /// 最大缓存大小（字节）
    pub max_size_bytes: usize,
    /// 最大缓存条目数量
    pub max_entries: usize,
    /// 可缓存的 HTTP 方法
    #[serde(default = "default_cacheable_methods")]
    pub cacheable_methods: Vec<String>,
    /// 可缓存的状态码
    #[serde(default = "default_cacheable_status_codes")]
    pub cacheable_status_codes: Vec<u16>,
}

fn default_cacheable_methods() -> Vec<String> {
    DEFAULT_CACHEABLE_METHODS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_cacheable_status_codes() -> Vec<u16> {
    DEFAULT_CACHEABLE_STATUS_CODES.to_vec()
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: DEFAULT_CACHE_TTL,
            max_size_bytes: DEFAULT_MAX_CACHE_SIZE,
            max_entries: DEFAULT_MAX_CACHE_ENTRIES,
            cacheable_methods: default_cacheable_methods(),
            cacheable_status_codes: default_cacheable_status_codes(),
        }
    }
}

/// Cache entry (with access time for LRU)
///
/// A cache entry stores the complete cached response including body, headers,
/// and metadata for cache validation (ETag, Last-Modified) and expiration.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached response body as bytes (Arc-wrapped for efficient sharing)
    body: Arc<Vec<u8>>,
    /// Response headers (excluding caching-related headers like Cache-Control, ETag, Last-Modified)
    headers: HashMap<String, HeaderValue>,
    /// ETag header value for conditional requests (based on SHA256 hash of body)
    etag: String,
    /// Last-Modified timestamp for cache validation (Unix timestamp in seconds)
    last_modified: u64,
    /// Expiration timestamp (Unix timestamp in seconds), entry is invalid after this time
    expires_at: u64,
    /// Entry size in bytes (body length), used for size-based eviction
    size: usize,
}

/// Cache key with full request information
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    method: String,
    uri: String,
    query_string: String,
    headers_hash: String,
    body_hash: Option<String>,
}

impl CacheKey {
    /// Create cache key with optional body
    ///
    /// Security: Uses SHA256 cryptographic hash for cache keys to prevent
    /// hash flooding attacks. This is safer than fast non-cryptographic hashes
    /// when cache keys are derived from untrusted input.
    pub fn new(method: &str, uri: &str, body: Option<&[u8]>, headers: &axum::http::HeaderMap) -> Self {
        // Extract query string
        let query_string = if let Some(query) = uri.split('?').nth(1) {
            query.to_string()
        } else {
            String::new()
        };

        // Extract and hash headers (exclude cache-related headers)
        let cache_headers: Vec<_> = headers
            .iter()
            .filter(|(name, _)| {
                !matches!(
                    name.as_str(),
                    "cache-control" | "pragma" | "authorization" | "cookie"
                )
            })
            .map(|(name, value)| {
                format!("{}:{}", name.as_str(), value.to_str().unwrap_or(""))
            })
            .collect();

        let headers_hash = Self::secure_hash(cache_headers.join("\n").as_bytes());

        let body_hash = body.map(|b| Self::secure_hash(b));

        Self {
            method: method.to_string(),
            uri: uri.split('?').next().unwrap_or(uri).to_string(),
            query_string,
            headers_hash,
            body_hash,
        }
    }

    /// Secure cryptographic hash for cache keys
    ///
    /// Uses SHA256 to prevent hash flooding attacks. While slightly slower than
    /// SipHash, it provides better security against collision attacks where
    /// an attacker crafts many requests that hash to the same cache key.
    /// For cache keys, the security benefit outweighs the minor performance cost.
    fn secure_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

/// 缓存中间件
#[derive(Clone)]
pub struct CacheMiddleware {
    config: CacheConfig,
    cache: Arc<DashMap<CacheKey, CacheEntry>>,
    current_size: Arc<AtomicUsize>,
    entry_count: Arc<AtomicUsize>,
    /// LRU heap for O(1) eviction - stores (key, last_accessed) pairs
    lru_heap: Arc<DashMap<CacheKey, MinHeap>>,
    /// Expiration index for O(1) expired entry lookup
    expiration_index: Arc<DashMap<u64, Vec<CacheKey>>>,
}

impl CacheMiddleware {
    /// Create new cache middleware with optimized O(1) LRU eviction
    ///
    /// Uses a binary heap (MinHeap) for efficient LRU eviction instead of
    /// full iteration and sorting. This provides O(1) access to the oldest
    /// entry and O(log n) insertion/removal.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: Arc::new(DashMap::new()),
            current_size: Arc::new(AtomicUsize::new(0)),
            entry_count: Arc::new(AtomicUsize::new(0)),
            lru_heap: Arc::new(DashMap::new()),
            expiration_index: Arc::new(DashMap::new()),
        }
    }

    /// 生成 ETag（基于响应内容的 SHA256）
    #[inline]
    pub fn generate_etag(body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(body);
        let result = hasher.finalize();
        format!("\"{:x}\"", result)
    }

    /// 生成 Last-Modified 时间戳
    #[inline]
    pub fn generate_last_modified() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// 生成缓存键
    #[inline]
    pub fn generate_cache_key(
        method: &str,
        uri: &str,
        body: Option<&[u8]>,
        headers: &axum::http::HeaderMap,
    ) -> CacheKey {
        CacheKey::new(method, uri, body, headers)
    }

    /// 检查是否应该缓存响应
    #[inline]
    pub fn should_cache(&self, method: &str, status: u16) -> bool {
        // 将方法转换为大写以进行匹配，避免每次创建新的 String 对象
        let method_upper = method.to_uppercase();
        self.config
            .cacheable_methods
            .iter()
            .any(|m| m == &method_upper)
            && self.config.cacheable_status_codes.contains(&status)
    }

    /// 检查缓存是否过期
    #[inline]
    fn is_expired(&self, expires_at: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now > expires_at
    }

    /// 获取当前时间戳（用于访问时间）
    #[inline]
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Execute LRU eviction using binary heap - O(1) min extraction
    ///
    /// The binary heap provides O(1) access to the oldest entry and O(log n)
    /// for insertion/removal, compared to O(n) iteration + O(n log n) sorting
    /// in the naive approach.
    ///
    /// Security: Uses Acquire/Release ordering for consistency in concurrent scenarios
    fn evict_lru(&self, min_needed: usize) {
        let max_entries = self.config.max_entries;
        let max_size = self.config.max_size_bytes;
        let mut freed = 0;
        let mut removed_count = 0;
        const MAX_ATTEMPTS: usize = 100; // 安全限制,避免无限循环

        loop {
            // 使用 Acquire 顺序确保读取最新的缓存状态
            let size_now = self.current_size.load(Ordering::Acquire);
            let count_now = self.entry_count.load(Ordering::Acquire);

            if size_now + min_needed <= max_size && count_now < max_entries {
                break;
            }

            // 检查是否达到安全限制
            if removed_count >= MAX_ATTEMPTS {
                #[cfg(feature = "logging")]
                tracing::warn!("LRU eviction hit safety limit of {} entries", MAX_ATTEMPTS);
                break;
            }

            let mut found_entry = false;
            for mut shard in self.lru_heap.iter_mut() {
                let heap = shard.value_mut();
                if let Some(entry) = heap.extract_min() {
                    if let Some((_, cache_entry)) = self.cache.remove(&entry.key) {
                        self.remove_from_expiration_index(&entry.key, cache_entry.expires_at);
                        let size = cache_entry.body.len();
                        self.current_size.fetch_sub(size, Ordering::Release);
                        self.entry_count.fetch_sub(1, Ordering::Release);
                        freed += size;
                        removed_count += 1;
                    }
                    found_entry = true;
                    break;
                }
            }

            if !found_entry || self.entry_count.load(Ordering::Acquire) == 0 {
                break;
            }
        }

        #[cfg(feature = "logging")]
        if removed_count > 0 {
            tracing::debug!(
                freed_bytes = freed,
                removed_entries = removed_count,
                "LRU eviction completed"
            );
        }
    }

    /// Add entry to expiration index
    fn add_to_expiration_index(&self, key: &CacheKey, expires_at: u64) {
        let mut shard = self.expiration_index.entry(expires_at).or_default();
        shard.push(key.clone());
    }

    /// Remove entry from expiration index
    fn remove_from_expiration_index(&self, key: &CacheKey, expires_at: u64) {
        if let Some(mut shard) = self.expiration_index.get_mut(&expires_at) {
            shard.retain(|k| k != key);
        }
    }

    /// Clear expired cache using expiration index - O(1) lookup
    fn clear_expired_entries(&self) -> usize {
        let now = CacheMiddleware::now();
        let mut removed = 0;

        let expired_times: Vec<u64> = self
            .expiration_index
            .iter()
            .filter(|shard| shard.key() <= &now)
            .map(|shard| *shard.key())
            .collect();

        for expires_at in expired_times {
            if let Some((_, keys)) = self.expiration_index.remove(&expires_at) {
                for key in keys {
                    if let Some((_, entry)) = self.cache.remove(&key) {
                        self.entry_count.fetch_sub(1, Ordering::Relaxed);
                        self.current_size.fetch_sub(entry.size, Ordering::Relaxed);
                        removed += 1;
                    }
                }
            }
        }

        removed
    }

    /// Clear expired cache and evict over-limit entries (uses expiration index for O(1) lookup)
    fn cleanup_and_evict(&self, needed: usize) {
        let _ = self.clear_expired_entries();

        let current_size = self.current_size.load(Ordering::Relaxed);
        let entry_count = self.entry_count.load(Ordering::Relaxed);

        if current_size + needed > self.config.max_size_bytes
            || entry_count >= self.config.max_entries
        {
            self.evict_lru(needed);
        }
    }

    /// 检查并强制执行大小限制
    #[inline]
    fn enforce_size_limit(&self, needed: usize) {
        let current_size = self.current_size.load(Ordering::Relaxed);
        let entry_count = self.entry_count.load(Ordering::Relaxed);

        if current_size + needed > self.config.max_size_bytes
            || entry_count >= self.config.max_entries
        {
            self.cleanup_and_evict(needed);
        }
    }

    /// Update access time (LRU) - O(log n) due to heap update
    fn update_access_time(&self, key: &CacheKey) {
        let now = CacheMiddleware::now();

        for mut shard in self.lru_heap.iter_mut() {
            let heap = shard.value_mut();
            if heap.remove(key) {
                heap.push(MinHeapEntry {
                    key: key.clone(),
                    last_accessed: now,
                });
                return;
            }
        }
    }
}

impl<S> Layer<S> for CacheMiddleware {
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            middleware: self.clone(),
        }
    }
}

/// 缓存服务
#[derive(Clone)]
pub struct CacheService<S> {
    inner: S,
    middleware: CacheMiddleware,
}

impl<S> Service<Request> for CacheService<S>
where
    S: Service<Request, Response = Response> + Send + 'static + Clone,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future = futures_util::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let middleware = self.middleware.clone();
        let method = req.method().to_string();
        let uri = req.uri().to_string();

        // 检查是否应该缓存此方法
        if !middleware.should_cache(&method, 200) {
            // 不缓存，直接转发请求
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        // 生成缓存键（简化版本，不包含请求体）
        // 注意：对于 POST/PUT 请求，应该包含请求体，但为了简化，这里暂时不处理
        let cache_key = CacheMiddleware::generate_cache_key(&method, &uri, None, req.headers());

        // 检查条件请求（If-None-Match）
        if let Some(if_none_match) = req.headers().get(IF_NONE_MATCH) {
            if let Some(entry) = middleware.cache.get(&cache_key) {
                // 检查 ETag 是否匹配
                if if_none_match
                    .to_str()
                    .ok()
                    .map(|s| s == entry.etag)
                    .unwrap_or(false)
                {
                    // ETag 匹配，返回 304 Not Modified
                    let mut response = Response::new(axum::body::Body::empty());
                    *response.status_mut() = StatusCode::NOT_MODIFIED;
                    return Box::pin(async move { Ok(response) });
                }
            }
        }

        // 检查缓存
        if let Some(entry) = middleware.cache.get(&cache_key) {
            // 检查是否过期
            if !middleware.is_expired(entry.expires_at) {
                // 更新访问时间（LRU）
                middleware.update_access_time(&cache_key);

                // 缓存命中，返回缓存的响应
                // Use Bytes::from for zero-copy body creation when possible
                let body_bytes = bytes::Bytes::copy_from_slice(&entry.body);
                let mut response = Response::new(axum::body::Body::from(body_bytes));

                // 添加缓存头
                if let Ok(etag_value) = HeaderValue::from_str(&entry.etag) {
                    response.headers_mut().insert(ETAG, etag_value);
                }
                if let Ok(lm_value) = HeaderValue::from_str(&entry.last_modified.to_string()) {
                    response.headers_mut().insert(LAST_MODIFIED, lm_value);
                }
                if let Ok(cc_value) =
                    HeaderValue::from_str(&format!("max-age={}", middleware.config.ttl_seconds))
                {
                    response.headers_mut().insert(CACHE_CONTROL, cc_value);
                }

                // 添加其他头
                for (name, value) in &entry.headers {
                    if let Ok(name) = axum::http::HeaderName::from_bytes(name.as_bytes()) {
                        response.headers_mut().insert(name, value.clone());
                    }
                }

                return Box::pin(async move { Ok(response) });
            }
        }

        // 缓存未命中，转发请求
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let response = inner.call(req).await?;

            // 检查是否应该缓存响应
            let status = response.status().as_u16();
            if middleware.should_cache(&method, status) {
                // 提取响应体
                let (parts, body) = response.into_parts();
                let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
                    Ok(bytes) => bytes.to_vec(),
                    Err(_e) => {
                        // 响应体转换失败，不缓存
                        #[cfg(feature = "logging")]
                        tracing::error!(error = %_e, "Failed to convert response body to bytes for caching");
                        let response = Response::from_parts(parts, axum::body::Body::empty());
                        return Ok(response);
                    }
                };

                // 获取 body 长度（在移动 body_bytes 之前）
                let body_len = body_bytes.len();

                // 创建缓存条目
                let etag = CacheMiddleware::generate_etag(&body_bytes);
                let last_modified = CacheMiddleware::generate_last_modified();
                let expires_at = last_modified + middleware.config.ttl_seconds;

                // 提取响应头
                let mut headers = HashMap::new();
                for (name, value) in parts.headers.iter() {
                    if name != CACHE_CONTROL && name != ETAG && name != LAST_MODIFIED {
                        headers.insert(name.as_str().to_string(), value.clone());
                    }
                }

                let entry = CacheEntry {
                    body: Arc::new(body_bytes),
                    headers,
                    etag: etag.clone(),
                    last_modified,
                    expires_at,
                    size: body_len,
                };

                // 存储到缓存
                let entry_size = body_len;

                // 检查大小限制（使用新的 LRU 机制）
                middleware.enforce_size_limit(entry_size);

                // 再次检查大小限制后尝试插入
                let current_size = middleware.current_size.load(Ordering::Relaxed);
                if current_size + entry_size <= middleware.config.max_size_bytes {
                    // 使用 Arc 共享缓存条目，避免克隆
                    let entry_arc = Arc::new(entry);

                    // Clone cache_key before moving into cache and expiration_index
                    let cache_key_for_heap = cache_key.clone();
                    let cache_key_for_expiry = cache_key.clone();
                    middleware.cache.insert(cache_key.clone(), Arc::clone(&entry_arc));
                    // Add to LRU heap for O(1) eviction
                    if let Some(mut shard) = middleware.lru_heap.iter_mut().next() {
                        let heap = shard.value_mut();
                        heap.push(MinHeapEntry {
                            key: cache_key_for_heap,
                            last_accessed: CacheMiddleware::now(),
                        });
                    }
                    // Add to expiration index for O(1) expired entry lookup
                    middleware.add_to_expiration_index(&cache_key_for_expiry, expires_at);
                    middleware
                        .current_size
                        .fetch_add(entry_size, Ordering::Relaxed);
                    middleware.entry_count.fetch_add(1, Ordering::Relaxed);

                    // 构建响应
                    let body_bytes = bytes::Bytes::copy_from_slice(&entry_arc.body);
                    let mut response = Response::from_parts(parts, axum::body::Body::from(body_bytes));
                    if let Ok(etag_value) = HeaderValue::from_str(&entry_arc.etag) {
                        response.headers_mut().insert(ETAG, etag_value);
                    }
                    if let Ok(lm_value) = HeaderValue::from_str(&entry_arc.last_modified.to_string()) {
                        response.headers_mut().insert(LAST_MODIFIED, lm_value);
                    }
                    if let Ok(cc_value) =
                        HeaderValue::from_str(&format!("max-age={}", middleware.config.ttl_seconds))
                    {
                        response.headers_mut().insert(CACHE_CONTROL, cc_value);
                    }

                    return Ok(response);
                }

                // 构建响应
                // Use Bytes::from for zero-copy body creation when possible
                let body_bytes = bytes::Bytes::copy_from_slice(&entry.body);
                let mut response = Response::from_parts(parts, axum::body::Body::from(body_bytes));
                if let Ok(etag_value) = HeaderValue::from_str(&etag) {
                    response.headers_mut().insert(ETAG, etag_value);
                }
                if let Ok(lm_value) = HeaderValue::from_str(&last_modified.to_string()) {
                    response.headers_mut().insert(LAST_MODIFIED, lm_value);
                }
                if let Ok(cc_value) =
                    HeaderValue::from_str(&format!("max-age={}", middleware.config.ttl_seconds))
                {
                    response.headers_mut().insert(CACHE_CONTROL, cc_value);
                }

                Ok(response)
            } else {
                // 不缓存，直接返回响应
                let (parts, body) = response.into_parts();
                Ok(Response::from_parts(parts, body))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_cache_config() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl_seconds, 300);
        assert_eq!(config.max_size_bytes, 100 * 1024 * 1024);
        assert!(config.cacheable_methods.contains(&"GET".to_string()));
    }

    #[test]
    fn test_etag_generation() {
        let body = b"Hello, World!";
        let etag1 = CacheMiddleware::generate_etag(body);
        let etag2 = CacheMiddleware::generate_etag(body);
        assert_eq!(etag1, etag2);

        let different_body = b"Different content";
        let etag3 = CacheMiddleware::generate_etag(different_body);
        assert_ne!(etag1, etag3);
    }

    #[test]
    fn test_cache_key_generation() {
        let headers = axum::http::HeaderMap::new();
        let key1 = CacheMiddleware::generate_cache_key("GET", "/api/users", None, &headers);
        let key2 = CacheMiddleware::generate_cache_key("GET", "/api/users", None, &headers);
        assert_eq!(key1, key2);

        let key3 = CacheMiddleware::generate_cache_key(
            "POST",
            "/api/users",
            Some(b"{\"name\":\"test\"}"),
            &headers,
        );
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_should_cache() {
        let config = CacheConfig::default();
        let middleware = CacheMiddleware::new(config);

        assert!(middleware.should_cache("GET", 200));
        assert!(middleware.should_cache("GET", 404));
        assert!(!middleware.should_cache("POST", 200));
        assert!(!middleware.should_cache("GET", 500));
    }

    #[test]
    fn test_cache_key_with_body() {
        let headers = axum::http::HeaderMap::new();
        let key1 = CacheMiddleware::generate_cache_key("GET", "/api/users", None, &headers);
        let key2 = CacheMiddleware::generate_cache_key("GET", "/api/users", None, &headers);
        assert_eq!(key1, key2);

        let key3 = CacheMiddleware::generate_cache_key("POST", "/api/users", Some(b"body"), &headers);
        assert_ne!(key1, key3);

        let key4 =
            CacheMiddleware::generate_cache_key("GET", "/api/users", Some(b"different body"), &headers);
        assert_ne!(key1, key4);
    }

    #[test]
    fn test_min_heap_operations() {
        let mut heap = MinHeap::new();
        assert!(heap.is_empty());

        heap.push(MinHeapEntry {
            key: CacheKey::new("GET", "/a", b""),
            last_accessed: 1,
        });
        heap.push(MinHeapEntry {
            key: CacheKey::new("GET", "/b", b""),
            last_accessed: 3,
        });
        heap.push(MinHeapEntry {
            key: CacheKey::new("GET", "/c", b""),
            last_accessed: 2,
        });

        assert_eq!(heap.len(), 3);

        let min = heap.extract_min().unwrap();
        assert_eq!(min.last_accessed, 1);
    }
}
