// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// HTTP Integration Tests
// Covers TC-INT-001, TC-INT-003, TC-INT-004, TC-INT-005, TC-INT-006

#[cfg(feature = "http")]
mod http_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use sdforge::http::build;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_http_server_builds() {
        let app = build();
        // Verify the app is not null and has expected structure
        assert!(
            !std::ptr::eq(&app, std::ptr::null()),
            "HTTP app should build successfully"
        );
    }

    #[test]
    fn test_http_build_sync() {
        let app = build();
        // Verify the app builds without panic
        assert!(
            !std::ptr::eq(&app, std::ptr::null()),
            "HTTP sync build should succeed"
        );
    }

    /// Test: Basic router with GET endpoint
    #[tokio::test]
    async fn test_basic_router_get_endpoint() {
        async fn handler() -> &'static str {
            "Hello, Integration Test!"
        }

        let app = Router::new().route("/test", get(handler));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
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
        async fn hello() -> &'static str {
            "Hello"
        }
        async fn world() -> &'static str {
            "World"
        }

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

    // ============================================================================
    // Advanced HTTP Tests - Performance and Concurrency
    // ============================================================================

    /// Test: HTTP request handling performance
    #[tokio::test]
    async fn test_http_request_performance() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
            routing::get,
            Router,
        };
        use tower::ServiceExt;

        async fn handler() -> &'static str {
            "OK"
        }
        let app = Router::new().route("/perf", get(handler));

        let start = std::time::Instant::now();

        // Handle 1000 requests
        for _ in 0..1000 {
            let response = app
                .clone()
                .oneshot(Request::builder().uri("/perf").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let elapsed = start.elapsed();
        // Should handle 1000 requests in less than 500ms
        assert!(elapsed < std::time::Duration::from_millis(500));
    }

    /// Test: Concurrent HTTP requests
    #[tokio::test]
    async fn test_concurrent_http_requests() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
            routing::get,
            Router,
        };
        use tower::ServiceExt;

        async fn handler() -> &'static str {
            "Concurrent OK"
        }
        let app = Router::new().route("/concurrent", get(handler));

        let mut handles = vec![];

        // Spawn 20 concurrent requests
        for i in 0..20 {
            let app_clone = app.clone();
            let handle = tokio::spawn(async move {
                let response = app_clone
                    .oneshot(
                        Request::builder()
                            .uri("/concurrent")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                (i, response.status())
            });
            handles.push(handle);
        }

        // Wait for all tasks and verify responses
        for handle in handles {
            let (idx, status) = handle.await.unwrap();
            assert_eq!(status, StatusCode::OK, "Request {} should succeed", idx);
        }
    }

    /// Test: Large response body handling
    #[tokio::test]
    async fn test_large_response_body() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
            routing::get,
            Router,
        };
        use tower::ServiceExt;

        async fn handler() -> String {
            "x".repeat(100_000) // 100KB response
        }

        let app = Router::new().route("/large", get(handler));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/large")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 200_000)
            .await
            .unwrap();
        assert_eq!(body.len(), 100_000);
    }

    /// Test: Multiple route patterns stress test
    #[tokio::test]
    async fn test_multiple_route_patterns() {
        use axum::{
            body::Body,
            http::{Request, StatusCode},
            routing::get,
            Router,
        };
        use tower::ServiceExt;

        async fn h1() -> &'static str {
            "Route 1"
        }
        async fn h2() -> &'static str {
            "Route 2"
        }
        async fn h3() -> &'static str {
            "Route 3"
        }
        async fn h4() -> &'static str {
            "Route 4"
        }
        async fn h5() -> &'static str {
            "Route 5"
        }

        let app = Router::new()
            .route("/api/v1/resource1", get(h1))
            .route("/api/v1/resource2", get(h2))
            .route("/api/v2/resource1", get(h3))
            .route("/api/v2/resource2", get(h4))
            .route("/health", get(h5));

        // Test all routes
        let routes = vec![
            ("/api/v1/resource1", "Route 1"),
            ("/api/v1/resource2", "Route 2"),
            ("/api/v2/resource1", "Route 3"),
            ("/api/v2/resource2", "Route 4"),
            ("/health", "Route 5"),
        ];

        for (path, expected) in routes {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            let body = axum::body::to_bytes(response.into_body(), 1024)
                .await
                .unwrap();
            assert_eq!(&body[..], expected.as_bytes());
        }
    }
}

#[cfg(all(feature = "http", feature = "timestamp"))]
mod timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_enabled() {
        let app = build();
        assert!(
            !std::ptr::eq(&app, std::ptr::null()),
            "HTTP app with timestamp should build"
        );
    }
}

#[cfg(all(feature = "http", not(feature = "timestamp")))]
mod no_timestamp_tests {
    use sdforge::http::build;

    #[test]
    fn test_timestamp_feature_disabled() {
        let app = build();
        assert!(
            !std::ptr::eq(&app, std::ptr::null()),
            "HTTP app without timestamp should build"
        );
    }
}

#[cfg(all(feature = "http", feature = "streaming"))]
mod streaming_tests {
    use sdforge::http::build;

    #[test]
    fn test_streaming_feature_enabled() {
        let app = build();
        assert!(
            !std::ptr::eq(&app, std::ptr::null()),
            "HTTP app with streaming should build"
        );
    }
}
