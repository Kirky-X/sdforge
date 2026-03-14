// Copyright (c) 2026 Kirky.X
//! Direct oxcache integration for HTTP caching

use axum::{
    body::Body,
    extract::Request,
    http::{header::CACHE_CONTROL, HeaderValue},
};
use http::Response as HttpResponse;
use oxcache::cache::Cache;
use oxcache::error::CacheError as OxcacheError;
use sha2::Digest;
use std::sync::Arc;
use std::time::Duration;
use tower::{Layer, Service};

/// Cache error types used by SDForge cache integration.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Underlying oxcache error.
    #[error("Oxcache error: {source}")]
    Oxcache {
        /// Root cause from oxcache.
        #[from]
        source: OxcacheError,
    },
    /// Cache entry was not found for the provided key.
    #[error("Key not found: {key}")]
    NotFound {
        /// Cache key that was not found.
        key: String,
    },
    /// Failure to serialize or deserialize cached data.
    #[error("Serialization error: {message}")]
    Serialization {
        /// Serialization error message.
        message: String,
    },
    /// IO-related cache error.
    #[error("IO error: {reason}")]
    Io {
        /// IO error description.
        reason: String,
    },
    /// Cache entry expired based on TTL.
    #[error("TTL expired")]
    Expired,
}

const DEFAULT_TTL: u64 = 300;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Time-to-live (seconds) for cached entries.
    pub ttl: u64,
    /// Maximum cache capacity in bytes.
    pub max: usize,
    /// Allowed HTTP methods for caching.
    pub methods: Vec<String>,
    /// Allowed HTTP status codes for caching.
    pub statuses: Vec<u16>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: DEFAULT_TTL,
            max: 100 * 1024 * 1024, // 100MB
            methods: vec!["GET".into()],
            statuses: vec![200, 203, 204, 206, 300, 301, 404],
        }
    }
}

impl CacheConfig {
    /// Create new config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom parameters
    pub fn with_params(ttl: u64, max: usize, methods: Vec<String>, statuses: Vec<u16>) -> Self {
        Self {
            ttl,
            max,
            methods,
            statuses,
        }
    }
}

/// Cache middleware using oxcache directly
pub struct CacheMiddleware {
    config: Arc<CacheConfig>,
    cache: Arc<Cache<String, Vec<u8>>>,
}

impl CacheMiddleware {
    /// Convert our CacheError to oxcache error
    fn map_oxcache_error(error: OxcacheError) -> CacheError {
        CacheError::Oxcache { source: error }
    }
}

impl Clone for CacheMiddleware {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
        }
    }
}

impl CacheMiddleware {
    /// Create new cache middleware with default oxcache instance
    pub async fn new() -> Result<Self, CacheError> {
        let cache: Cache<String, Vec<u8>> = Cache::builder()
            .capacity(10_000) // Default capacity
            .build()
            .await
            .map_err(Self::map_oxcache_error)?;

        Ok(Self {
            config: Arc::new(CacheConfig::default()),
            cache: Arc::new(cache),
        })
    }

    /// Create with custom config
    pub async fn with_config(config: CacheConfig) -> Result<Self, CacheError> {
        let cache: Cache<String, Vec<u8>> = Cache::builder()
            .capacity((config.max / 1024).try_into().unwrap()) // Convert bytes to approximate item count
            .ttl(Duration::from_secs(config.ttl))
            .build()
            .await
            .map_err(Self::map_oxcache_error)?;

        Ok(Self {
            config: Arc::new(config),
            cache: Arc::new(cache),
        })
    }

    /// Create with custom config and cache instance
    pub fn with_config_and_cache(config: CacheConfig, cache: Cache<String, Vec<u8>>) -> Self {
        Self {
            config: Arc::new(config),
            cache: Arc::new(cache),
        }
    }

    /// Create with dependencies (for DI mode)
    pub fn with_dependencies(config: Arc<CacheConfig>, cache: Arc<Cache<String, Vec<u8>>>) -> Self {
        Self { config, cache }
    }

    /// Generate ETag header value for response body
    pub(crate) fn etag(data: &[u8]) -> String {
        format!(
            "\"{:x}\"",
            sha2::Sha256::new().chain_update(data).finalize()
        )
    }

    /// Check if a response is cacheable by method and status.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method name.
    /// * `status` - HTTP response status code.
    ///
    /// # Returns
    ///
    /// Returns true if the response is eligible for caching.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use sdforge::cache::CacheConfig;
    ///
    /// let config = CacheConfig::default();
    /// let can_cache = config.methods.contains(&"GET".to_string());
    /// let _ = can_cache;
    /// ```
    pub(crate) fn can_cache(&self, method: &str, status: u16) -> bool {
        self.config.methods.iter().any(|m| m == method) && self.config.statuses.contains(&status)
    }
}

impl<S> Layer<S> for CacheMiddleware
where
    S: Service<Request, Response = HttpResponse<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Service = CacheService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheService {
            inner,
            middleware: Arc::new(self.clone()),
        }
    }
}

#[derive(Clone)]
/// Cache service wrapper that applies CacheMiddleware.
pub struct CacheService<S> {
    inner: S,
    middleware: Arc<CacheMiddleware>,
}

impl<S> Service<Request> for CacheService<S>
where
    S: Service<Request, Response = HttpResponse<Body>> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = HttpResponse<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let middleware = self.middleware.clone();
        let cache = middleware.cache.clone();
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let headers = request.headers().clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if !middleware.can_cache(&method, 200) {
                return inner.call(request).await;
            }

            let key = format!("{}:{}:{:?}", method, uri, headers);

            // Check if cached response exists
            if let Ok(Some(cached)) = cache.get_bytes(&key).await {
                let etag = CacheMiddleware::etag(&cached);
                let mut response = HttpResponse::new(Body::from(cached));

                if let Ok(v) = HeaderValue::from_str(&etag) {
                    response.headers_mut().insert(http::header::ETAG, v);
                }
                if let Ok(v) = HeaderValue::from_str(&format!("max-age={}", middleware.config.ttl))
                {
                    response.headers_mut().insert(CACHE_CONTROL, v);
                }
                return Ok(response);
            }

            // Execute the original request
            let response = inner.call(request).await?;
            let status = response.status().as_u16();

            if middleware.can_cache(&method, status) {
                let (parts, body) = response.into_parts();
                let body_bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
                    Ok(bytes) => bytes.to_vec(),
                    Err(_) => {
                        return Ok(HttpResponse::from_parts(parts, Body::empty()));
                    }
                };

                let etag = CacheMiddleware::etag(&body_bytes);

                // Store response in cache
                let _ = cache
                    .set_bytes(&key, body_bytes.clone(), Some(middleware.config.ttl))
                    .await;

                let mut response = HttpResponse::from_parts(parts, Body::from(body_bytes));

                if let Ok(v) = HeaderValue::from_str(&etag) {
                    response.headers_mut().insert(http::header::ETAG, v);
                }
                if let Ok(v) = HeaderValue::from_str(&format!("max-age={}", middleware.config.ttl))
                {
                    response.headers_mut().insert(CACHE_CONTROL, v);
                }
                return Ok(response);
            }

            let (parts, body) = response.into_parts();
            Ok(HttpResponse::from_parts(parts, body))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ============================================================================
    // CacheError Tests
    // ============================================================================

    #[test]
    fn test_cache_error_display() {
        let error = CacheError::NotFound {
            key: "test_key".to_string(),
        };
        assert_eq!(error.to_string(), "Key not found: test_key");
    }

    #[test]
    fn test_cache_error_serialization() {
        let error = CacheError::Serialization {
            message: "Failed to serialize".to_string(),
        };
        assert!(error.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_cache_error_io() {
        let error = CacheError::Io {
            reason: "Disk full".to_string(),
        };
        assert_eq!(error.to_string(), "IO error: Disk full");
    }

    #[test]
    fn test_cache_error_expired() {
        let error = CacheError::Expired;
        assert_eq!(error.to_string(), "TTL expired");
    }

    // ============================================================================
    // CacheConfig Tests
    // ============================================================================

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl, 300);
        assert_eq!(config.max, 100 * 1024 * 1024);
        assert_eq!(config.methods.len(), 1);
        assert_eq!(config.methods[0], "GET");
        assert_eq!(config.statuses.len(), 7);
        assert!(config.statuses.contains(&200));
    }

    #[test]
    fn test_cache_config_new() {
        let config = CacheConfig::new();
        assert_eq!(config.ttl, 300);
        assert_eq!(config.max, 100 * 1024 * 1024);
    }

    #[test]
    fn test_cache_config_with_params() {
        let config = CacheConfig::with_params(
            600,
            50 * 1024 * 1024,
            vec!["GET".into(), "HEAD".into()],
            vec![200, 301, 404],
        );
        assert_eq!(config.ttl, 600);
        assert_eq!(config.max, 50 * 1024 * 1024);
        assert_eq!(config.methods.len(), 2);
        assert_eq!(config.statuses.len(), 3);
    }

    #[test]
    fn test_cache_config_clone() {
        let config = CacheConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.ttl, config.ttl);
        assert_eq!(cloned.max, config.max);
    }

    // ============================================================================
    // CacheMiddleware Tests
    // ============================================================================

    #[test]
    fn test_etag_generation() {
        let data = b"test data for etag";
        let etag = CacheMiddleware::etag(data);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert!(etag.len() > 2);
    }

    #[test]
    fn test_etag_different_data() {
        let data1 = b"data one";
        let data2 = b"data two";
        let etag1 = CacheMiddleware::etag(data1);
        let etag2 = CacheMiddleware::etag(data2);
        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_etag_same_data() {
        let data = b"same data";
        let etag1 = CacheMiddleware::etag(data);
        let etag2 = CacheMiddleware::etag(data);
        assert_eq!(etag1, etag2);
    }

    #[test]
    fn test_etag_empty_data() {
        let data = b"";
        let etag = CacheMiddleware::etag(data);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
    }

    // ============================================================================
    // can_cache Tests
    // ============================================================================

    fn create_test_middleware() -> CacheMiddleware {
        // Create a middleware for testing can_cache
        // Note: We use a simple in-memory cache for unit tests
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = rt.block_on(async {
            Cache::<String, Vec<u8>>::builder()
                .capacity(10)
                .build()
                .await
                .unwrap()
        });

        CacheMiddleware {
            config: Arc::new(CacheConfig::default()),
            cache: Arc::new(cache),
        }
    }

    #[test]
    fn test_can_cache_get_200() {
        let middleware = create_test_middleware();
        assert!(middleware.can_cache("GET", 200));
    }

    #[test]
    fn test_can_cache_post_200() {
        let middleware = create_test_middleware();
        // POST is not in default methods
        assert!(!middleware.can_cache("POST", 200));
    }

    #[test]
    fn test_can_cache_get_404() {
        let middleware = create_test_middleware();
        // 404 is in default statuses
        assert!(middleware.can_cache("GET", 404));
    }

    #[test]
    fn test_can_cache_get_500() {
        let middleware = create_test_middleware();
        // 500 is not in default statuses
        assert!(!middleware.can_cache("GET", 500));
    }

    #[test]
    fn test_can_cache_head_200() {
        let middleware = create_test_middleware();
        // HEAD is not in default methods
        assert!(!middleware.can_cache("HEAD", 200));
    }

    #[test]
    fn test_can_cache_custom_methods() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = rt.block_on(async {
            Cache::<String, Vec<u8>>::builder()
                .capacity(10)
                .build()
                .await
                .unwrap()
        });

        let config = CacheConfig::with_params(
            300,
            100 * 1024 * 1024,
            vec!["GET".into(), "POST".into(), "HEAD".into()],
            vec![200, 201, 204],
        );
        let middleware = CacheMiddleware {
            config: Arc::new(config),
            cache: Arc::new(cache),
        };

        assert!(middleware.can_cache("GET", 200));
        assert!(middleware.can_cache("POST", 200));
        assert!(middleware.can_cache("HEAD", 200));
        assert!(!middleware.can_cache("PUT", 200));
        assert!(middleware.can_cache("GET", 201));
        assert!(!middleware.can_cache("GET", 500));
    }

    // ============================================================================
    // CacheMiddleware Clone Tests
    // ============================================================================

    #[test]
    fn test_middleware_clone() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = rt.block_on(async {
            Cache::<String, Vec<u8>>::builder()
                .capacity(10)
                .build()
                .await
                .unwrap()
        });

        let middleware = CacheMiddleware {
            config: Arc::new(CacheConfig::default()),
            cache: Arc::new(cache),
        };
        let cloned = middleware.clone();
        assert_eq!(
            Arc::strong_count(&middleware.config),
            Arc::strong_count(&cloned.config)
        );
    }

    // ============================================================================
    // CacheConfig Validation Tests
    // ============================================================================

    #[test]
    fn test_cache_config_zero_ttl() {
        let config = CacheConfig::with_params(0, 100 * 1024 * 1024, vec!["GET".into()], vec![200]);
        assert_eq!(config.ttl, 0);
    }

    #[test]
    fn test_cache_config_large_ttl() {
        let config =
            CacheConfig::with_params(86400, 100 * 1024 * 1024, vec!["GET".into()], vec![200]);
        assert_eq!(config.ttl, 86400);
    }

    #[test]
    fn test_cache_config_zero_max() {
        let config = CacheConfig::with_params(300, 0, vec!["GET".into()], vec![200]);
        assert_eq!(config.max, 0);
    }

    #[test]
    fn test_cache_config_empty_methods() {
        let config = CacheConfig::with_params(300, 100 * 1024 * 1024, vec![], vec![200]);
        assert!(config.methods.is_empty());
    }

    #[test]
    fn test_cache_config_empty_statuses() {
        let config = CacheConfig::with_params(300, 100 * 1024 * 1024, vec!["GET".into()], vec![]);
        assert!(config.statuses.is_empty());
    }

    // ============================================================================
    // CacheError Conversion Tests
    // ============================================================================

    #[test]
    fn test_map_oxcache_error() {
        use oxcache::error::CacheError;
        let oxcache_err = CacheError::NotFound("test".to_string());
        let our_err = CacheMiddleware::map_oxcache_error(oxcache_err);
        // map_oxcache_error wraps oxcache errors in our CacheError::Oxcache variant
        assert!(our_err.to_string().contains("Oxcache error"));
    }

    // ============================================================================
    // ETag Format Tests
    // ============================================================================

    #[test]
    fn test_etag_format_valid() {
        let data = b"test";
        let etag = CacheMiddleware::etag(data);
        // ETag format should be "sha256hash"
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
        assert!(!etag.contains('\n'));
        assert!(!etag.contains('\r'));
    }

    #[test]
    fn test_etag_deterministic() {
        let data = b"deterministic test";
        let etag1 = CacheMiddleware::etag(data);
        let etag2 = CacheMiddleware::etag(data);
        assert_eq!(etag1, etag2);
    }

    #[test]
    fn test_etag_large_data() {
        let data = vec![b'a'; 10000];
        let etag = CacheMiddleware::etag(&data);
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));
    }

    // ============================================================================
    // Edge Case Tests
    // ============================================================================

    #[test]
    fn test_cache_config_all_common_statuses() {
        let config = CacheConfig::with_params(
            300,
            100 * 1024 * 1024,
            vec!["GET".into()],
            vec![
                200, 201, 202, 203, 204, 206, 300, 301, 302, 304, 307, 404, 410,
            ],
        );
        assert_eq!(config.statuses.len(), 13);
    }

    #[test]
    fn test_cache_config_all_methods() {
        let config = CacheConfig::with_params(
            300,
            100 * 1024 * 1024,
            vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "DELETE".into(),
                "HEAD".into(),
                "OPTIONS".into(),
                "PATCH".into(),
            ],
            vec![200],
        );
        assert_eq!(config.methods.len(), 7);
    }
}
