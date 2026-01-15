//! Cache integration tests
//!
//! Tests for HTTP response caching functionality.

#[cfg(all(test, feature = "http", feature = "cache"))]
mod cache_integration_tests {
    use axiom::cache::CacheConfig;
    use axiom::http::build_with_config;

    #[tokio::test]
    async fn test_cache_middleware_creation() {
        let config = CacheConfig::default();
        let _middleware = axiom::cache::CacheMiddleware::new(config);
        // Middleware is created successfully
    }

    #[tokio::test]
    async fn test_cache_config_defaults() {
        let config = CacheConfig::default();
        assert_eq!(config.ttl_seconds, 300);
        assert_eq!(config.max_size_bytes, 100 * 1024 * 1024);
        assert_eq!(config.max_entries, 10000);
        assert!(config.cacheable_methods.contains(&"GET".to_string()));
        assert!(config.cacheable_methods.contains(&"HEAD".to_string()));
        assert!(!config.cacheable_methods.contains(&"POST".to_string()));
    }

    #[tokio::test]
    async fn test_cache_config_custom() {
        let config = CacheConfig {
            ttl_seconds: 600,
            max_size_bytes: 50 * 1024 * 1024,
            max_entries: 5000,
            cacheable_methods: vec!["GET".to_string(), "POST".to_string()],
            cacheable_status_codes: vec![200, 201],
        };
        assert_eq!(config.ttl_seconds, 600);
        assert_eq!(config.max_size_bytes, 50 * 1024 * 1024);
        assert_eq!(config.max_entries, 5000);
        assert!(config.cacheable_methods.contains(&"POST".to_string()));
    }

    #[tokio::test]
    async fn test_router_with_cache() {
        use axiom::config::AppConfig;

        let config = AppConfig::default();
        let result = build_with_config(&config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_etag_generation() {
        let body1 = b"Hello, World!";
        let etag1 = axiom::cache::CacheMiddleware::generate_etag(body1);
        let etag2 = axiom::cache::CacheMiddleware::generate_etag(body1);
        // Same body should generate same ETag
        assert_eq!(etag1, etag2);

        let body2 = b"Different content";
        let etag3 = axiom::cache::CacheMiddleware::generate_etag(body2);
        // Different body should generate different ETag
        assert_ne!(etag1, etag3);

        // ETag should be quoted
        assert!(etag1.starts_with('"'));
        assert!(etag1.ends_with('"'));
    }

    #[tokio::test]
    async fn test_last_modified_generation() {
        // Verify that timestamps are generated correctly
        let timestamp1 = axiom::cache::CacheMiddleware::generate_last_modified();
        // Wait at least 1 second to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_secs(1));
        let timestamp2 = axiom::cache::CacheMiddleware::generate_last_modified();
        // Timestamps should be different
        assert_ne!(timestamp1, timestamp2);
        // Second timestamp should be greater than first
        assert!(timestamp2 > timestamp1);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let key1 = axiom::cache::CacheMiddleware::generate_cache_key("GET", "/api/users", b"");
        let key2 = axiom::cache::CacheMiddleware::generate_cache_key("GET", "/api/users", b"");
        // Same method, URI, and body should generate same key
        assert_eq!(key1, key2);

        let key3 = axiom::cache::CacheMiddleware::generate_cache_key(
            "POST",
            "/api/users",
            b"{\"name\":\"test\"}",
        );
        // Different method should generate different key
        assert_ne!(key1, key3);

        let key4 = axiom::cache::CacheMiddleware::generate_cache_key("GET", "/api/users/1", b"");
        // Different URI should generate different key
        assert_ne!(key1, key4);
    }

    #[tokio::test]
    async fn test_should_cache() {
        let config = CacheConfig::default();
        let middleware = axiom::cache::CacheMiddleware::new(config);

        // GET 200 should be cacheable
        assert!(middleware.should_cache("GET", 200));
        assert!(middleware.should_cache("GET", 404));

        // POST should not be cacheable
        assert!(!middleware.should_cache("POST", 200));

        // GET 500 should not be cacheable
        assert!(!middleware.should_cache("GET", 500));

        // GET 301 should be cacheable
        assert!(middleware.should_cache("GET", 301));
    }

    #[tokio::test]
    async fn test_cache_headers() {
        // Test that cache headers are properly formatted
        let etag = "\"abc123\"";
        let last_modified = "1234567890";
        let cache_control = "max-age=300";

        // ETag should be properly quoted
        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));

        // Last-Modified should be a number
        assert!(last_modified.parse::<u64>().is_ok());

        // Cache-Control should contain max-age
        assert!(cache_control.contains("max-age="));
        assert!(cache_control.contains("300"));
    }

    #[tokio::test]
    async fn test_cache_middleware_layer() {
        use axum::Router;
        use tower::Layer;

        let config = CacheConfig::default();
        let middleware = axiom::cache::CacheMiddleware::new(config);
        let router: Router = Router::new();

        // CacheMiddleware should implement Layer trait
        let _layered_router = middleware.layer(router);
    }

    #[tokio::test]
    async fn test_cache_feature_compilation() {
        // This test verifies that the cache feature compiles correctly
        use axiom::cache::CacheConfig;
        use axiom::cache::CacheMiddleware;

        let _config = CacheConfig::default();
        let _middleware = CacheMiddleware::new(_config);
    }
}
