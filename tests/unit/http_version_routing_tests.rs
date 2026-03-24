#[cfg(feature = "http")]
mod version_routing_tests {
    use sdforge::http::version_routing::{
        build_version_router, version_redirect_middleware, VersionRouterConfig, VersionedRoute,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "test response"
    }

    #[test]
    fn test_version_router_config_default() {
        let config = VersionRouterConfig::default();
        assert_eq!(config.default_version, "v1");
        assert_eq!(config.supported_versions, vec!["v1"]);
        assert!(config.redirect_unknown);
        assert!(config.deprecated_versions.is_empty());
    }

    #[test]
    fn test_version_router_config_custom() {
        let config = VersionRouterConfig {
            default_version: "v2".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string()],
            redirect_unknown: false,
            deprecated_versions: std::collections::HashMap::new(),
            sunset_header: "Sunset".to_string(),
        };
        assert_eq!(config.default_version, "v2");
        assert_eq!(config.supported_versions.len(), 2);
        assert!(!config.redirect_unknown);
    }

    #[test]
    fn test_version_router_config_with_deprecated() {
        let mut deprecated = std::collections::HashMap::new();
        deprecated.insert("v1".to_string(), "2025-01-01".to_string());

        let config = VersionRouterConfig {
            default_version: "v2".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string()],
            redirect_unknown: true,
            deprecated_versions: deprecated,
            sunset_header: "Sunset".to_string(),
        };

        assert!(config.deprecated_versions.contains_key("v1"));
        assert_eq!(config.deprecated_versions.get("v1"), Some(&"2025-01-01".to_string()));
    }

    #[test]
    fn test_versioned_route_creation() {
        // Use MethodRouter (get()) instead of Router
        let route = VersionedRoute::new(
            "v1".to_string(),
            "/test".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        assert_eq!(route.version(), "v1");
        assert_eq!(route.path(), "/test");
    }

    #[tokio::test]
    async fn test_version_redirect_middleware_redirects_unknown_version() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .body(Body::empty())
                    .expect("Failed to build request"),
            )
            .await
            .expect("Failed to handle request");

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        let location = response.headers().get("location").expect("Location header missing");
        assert_eq!(location, "/api/v1/test");
    }

    #[tokio::test]
    async fn test_version_redirect_middleware_passes_valid_version() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/v1/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_redirect_middleware_invalid_version_redirects() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/invalid/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }

    #[tokio::test]
    async fn test_build_version_router_returns_router() {
        let router = build_version_router();
        // Verify we can call build_version_router and get a valid router
        assert!(!format!("{:?}", router).is_empty());
    }

    #[tokio::test]
    async fn test_version_redirect_non_api_path_passes_through() {
        let router = Router::new()
            .route("/health", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_redirect_root_api_redirects() {
        let router = Router::new()
            .route("/api/v1/", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }
}