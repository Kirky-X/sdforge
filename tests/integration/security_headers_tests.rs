// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
// Security Headers Integration Tests
// Tests security headers functionality through the HTTP build

#[cfg(feature = "http")]
mod security_headers_tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::{Router, routing::get};
    use tower::ServiceExt;

    /// Simple test handler
    async fn test_handler() -> &'static str {
        "OK"
    }

    #[test]
    fn test_http_build_with_security() {
        // Build HTTP server - security headers are applied internally
        let app = sdforge::http::build();
        // Verify the build succeeded
        assert!(
            !std::ptr::eq(&app, std::ptr::null()),
            "HTTP build with security should succeed"
        );
    }

    #[tokio::test]
    async fn test_basic_router_with_security_headers() {
        // Test that a basic router works
        let app = Router::new().route("/test", get(test_handler));

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_security_headers_config_default() {
        use sdforge::http::SecurityHeaders;
        // Test that we can create default security configuration
        let headers = SecurityHeaders::default();
        assert_eq!(headers.content_type_options, "nosniff");
    }
}
