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
use tower::{Layer, Service};

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

/// 缓存条目（带访问时间用于 LRU）
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 响应体
    body: Vec<u8>,
    /// 响应头
    headers: HashMap<String, HeaderValue>,
    /// ETag
    etag: String,
    /// Last-Modified 时间戳
    last_modified: u64,
    /// 过期时间戳
    expires_at: u64,
    /// 最近访问时间（用于 LRU）
    last_accessed: u64,
    /// 条目大小（字节）
    size: usize,
}

/// 缓存键
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    method: String,
    uri: String,
    body_hash: String,
}

/// 缓存中间件
#[derive(Clone)]
pub struct CacheMiddleware {
    config: CacheConfig,
    cache: Arc<DashMap<CacheKey, CacheEntry>>,
    current_size: Arc<AtomicUsize>,
    entry_count: Arc<AtomicUsize>,
    access_order: Arc<DashMap<CacheKey, u64>>, // 用于 LRU 排序
}

impl CacheMiddleware {
    /// 创建新的缓存中间件
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            cache: Arc::new(DashMap::new()),
            current_size: Arc::new(AtomicUsize::new(0)),
            entry_count: Arc::new(AtomicUsize::new(0)),
            access_order: Arc::new(DashMap::new()),
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
            .expect("System time is before Unix epoch")
            .as_secs()
    }

    /// 生成缓存键
    #[inline]
    pub fn generate_cache_key(method: &str, uri: &str, body: &[u8]) -> CacheKey {
        let mut hasher = Sha256::new();
        hasher.update(body);
        let body_hash = format!("{:x}", hasher.finalize());

        CacheKey {
            method: method.to_string(),
            uri: uri.to_string(),
            body_hash,
        }
    }

    /// 检查是否应该缓存响应
    #[inline]
    pub fn should_cache(&self, method: &str, status: u16) -> bool {
        self.config.cacheable_methods.contains(&method.to_string())
            && self.config.cacheable_status_codes.contains(&status)
    }

    /// 检查缓存是否过期
    #[inline]
    fn is_expired(&self, expires_at: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch")
            .as_secs();
        now > expires_at
    }

    /// 获取当前时间戳（用于访问时间）
    #[inline]
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch")
            .as_secs()
    }

    /// 执行 LRU 淘汰 - 删除最旧的条目
    fn evict_lru(&self, min_needed: usize) {
        // 收集所有条目及其访问时间
        let mut entries: Vec<_> = self
            .access_order
            .iter()
            .map(|r| (r.key().clone(), *r.value()))
            .collect();

        // 按访问时间排序（最旧的在前）
        entries.sort_by_key(|(_, time)| *time);

        // 删除条目直到有足够空间
        let mut freed = 0;
        for (key, _) in entries {
            if self.current_size.load(Ordering::Relaxed) + min_needed <= self.config.max_size_bytes
                && self.entry_count.load(Ordering::Relaxed) <= self.config.max_entries
            {
                break;
            }

            if let Some((_, entry)) = self.cache.remove(&key) {
                self.access_order.remove(&key);
                let size = entry.body.len();
                self.current_size.fetch_sub(size, Ordering::Relaxed);
                self.entry_count.fetch_sub(1, Ordering::Relaxed);
                freed += size;

                if freed >= min_needed {
                    break;
                }
            }
        }
    }

    /// 清除过期缓存和超出限制的缓存（调用 LRU 淘汰）
    fn cleanup_and_evict(&self, needed: usize) {
        // 先清理过期条目
        let now = CacheMiddleware::now();
        let mut keys_to_remove = Vec::new();

        for entry in self.cache.iter() {
            if now > entry.expires_at {
                keys_to_remove.push(entry.key().clone());
            }
        }

        for key in keys_to_remove {
            if let Some((_, entry)) = self.cache.remove(&key) {
                self.access_order.remove(&key);
                self.current_size.fetch_sub(entry.size, Ordering::Relaxed);
                self.entry_count.fetch_sub(1, Ordering::Relaxed);
            }
        }

        // 如果空间不足，执行 LRU 淘汰
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

    /// 更新访问时间（LRU）
    fn update_access_time(&self, key: &CacheKey) {
        let now = CacheMiddleware::now();
        self.access_order.insert(key.clone(), now);
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
        let cache_key = CacheMiddleware::generate_cache_key(&method, &uri, &[]);

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
                let mut response = Response::new(axum::body::Body::from(entry.body.clone()));

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
                    Err(_) => {
                        // 响应体太大，不缓存
                        let response = Response::from_parts(parts, axum::body::Body::empty());
                        return Ok(response);
                    }
                };

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
                    body: body_bytes.clone(),
                    headers,
                    etag: etag.clone(),
                    last_modified,
                    expires_at,
                    last_accessed: CacheMiddleware::now(),
                    size: body_bytes.len(),
                };

                // 存储到缓存
                let entry_size = body_bytes.len();

                // 检查大小限制（使用新的 LRU 机制）
                middleware.enforce_size_limit(entry_size);

                // 再次检查大小限制后尝试插入
                let current_size = middleware.current_size.load(Ordering::Relaxed);
                if current_size + entry_size <= middleware.config.max_size_bytes {
                    middleware.cache.insert(cache_key.clone(), entry.clone());
                    middleware
                        .access_order
                        .insert(cache_key, CacheMiddleware::now());
                    middleware
                        .current_size
                        .fetch_add(entry_size, Ordering::Relaxed);
                    middleware.entry_count.fetch_add(1, Ordering::Relaxed);
                }

                // 构建响应
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
        let key1 = CacheMiddleware::generate_cache_key("GET", "/api/users", b"");
        let key2 = CacheMiddleware::generate_cache_key("GET", "/api/users", b"");
        assert_eq!(key1, key2);

        let key3 =
            CacheMiddleware::generate_cache_key("POST", "/api/users", b"{\"name\":\"test\"}");
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
}
