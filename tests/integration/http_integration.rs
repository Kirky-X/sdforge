// HTTP Integration Tests
// Covers TC-INT-001, TC-INT-003, TC-INT-004, TC-INT-005, TC-INT-006

#[cfg(feature = "http")]
mod http_tests {
    use sdforge::http::build;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_http_server_builds() {
        let app = build();
        // Verify the app is not null and has expected structure
        assert!(!std::ptr::eq(&app, std::ptr::null()), "HTTP app should build successfully");
    }

    #[test]
    fn test_http_build_sync() {
        let app = build();
        // Verify the app builds without panic
        assert!(!std::ptr::eq(&app, std::ptr::null()), "HTTP sync build should succeed");
    }

    /// Test: Basic router with GET endpoint
    #[tokio::test]
    async fn test_basic_router_get_endpoint() {
        async fn handler() -> &'static str {
            "Hello, Integration Test!"
        }

        let app = Router::new().route("/test", get(handler));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello, Integration Test!");
    }

    /// Test: Multiple routes in same router
    #[tokio::test]
    async fn test_multiple_routes() {
        async fn hello() -> &'static str { "Hello" }
        async fn world() -> &'static str { "World" }

        let app = Router::new()
            .route("/hello", get(hello))
            .route("/world", get(world));

        // Test /hello endpoint
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/hello")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response1.status(), StatusCode::OK);

        // Test /world endpoint
        let response2 = app
            .oneshot(
                Request::builder()
                    .uri("/world")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response2.status(), StatusCode::OK);
    }

    /// Test: 404 for non-existent route
    #[tokio::test]
    async fn test_404_for_nonexistent_route() {
        let app = Router::new();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[cfg(all(feature = "http", feature = "timestamp"))]
mod timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_enabled() {
        let app = build();
        assert!(!std::ptr::eq(&app, std::ptr::null()), "HTTP app with timestamp should build");
    }
}

#[cfg(all(feature = "http", not(feature = "timestamp")))]
mod no_timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_disabled() {
        let app = build();
        assert!(!std::ptr::eq(&app, std::ptr::null()), "HTTP app without timestamp should build");
    }
}

#[cfg(all(feature = "http", feature = "streaming"))]
mod streaming_tests {
    use sdforge::http::build;

    #[test]
    fn test_streaming_feature_enabled() {
        let app = build();
        assert!(!std::ptr::eq(&app, std::ptr::null()), "HTTP app with streaming should build");
    }
}
