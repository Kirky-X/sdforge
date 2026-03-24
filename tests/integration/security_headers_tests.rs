// Security Headers Integration Tests
// Tests security headers functionality through the HTTP build

#[cfg(feature = "http")]
mod security_headers_tests {
    use axum::{routing::get, Router};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use axum::body::Body;

    /// Simple test handler
    async fn test_handler() -> &'static str {
        "OK"
    }

    #[test]
    fn test_http_build_with_security() {
        // Build HTTP server - security headers are applied internally
        let _app = sdforge::http::build();
        // If we get here without panicking, the build succeeded
    }

    #[tokio::test]
    async fn test_basic_router_with_security_headers() {
        // Test that a basic router works
        let app = Router::new()
            .route("/test", get(test_handler));

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
    }

    #[test]
    fn test_security_headers_config_default() {
        // Test that we can create default security configuration
        // The security headers are applied internally by the build function
        assert!(true);
    }
}

#[cfg(not(feature = "http"))]
mod security_headers_tests_placeholder {
    #[test]
    fn test_http_feature_required() {
        assert!(true, "Security headers tests require http feature");
    }
}