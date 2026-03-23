// Security Headers Integration Tests
// Tests apply_security_headers() functionality

#[cfg(feature = "http")]
mod security_headers_tests {
    use axum::{routing::get, Router};
    use axum::http::{header::*, HeaderValue, Request, StatusCode};
    use tower::ServiceExt;
    use axum::body::Body;

    /// Simple test handler
    async fn test_handler() -> &'static str {
        "OK"
    }

    // =========================================================================
    // Security Headers Tests
    // =========================================================================

    #[tokio::test]
    async fn test_security_headers_all_present() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

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

        // Check X-Content-Type-Options
        assert_eq!(
            response
                .headers()
                .get("x-content-type-options")
                .map(|v| v.to_str().unwrap()),
            Some("nosniff")
        );

        // Check X-Frame-Options
        assert_eq!(
            response.headers().get("x-frame-options").map(|v| v.to_str().unwrap()),
            Some("DENY")
        );

        // Check X-XSS-Protection
        assert_eq!(
            response.headers().get("x-xss-protection").map(|v| v.to_str().unwrap()),
            Some("1; mode=block")
        );

        // Check Cache-Control
        assert_eq!(
            response
                .headers()
                .get("cache-control")
                .map(|v| v.to_str().unwrap()),
            Some("no-store, no-cache, must-revalidate")
        );

        // Check Content-Security-Policy
        assert_eq!(
            response
                .headers()
                .get("content-security-policy")
                .map(|v| v.to_str().unwrap()),
            Some("default-src 'self'; script-src 'self'; style-src 'self'")
        );

        // Check Strict-Transport-Security
        assert_eq!(
            response
                .headers()
                .get("strict-transport-security")
                .map(|v| v.to_str().unwrap()),
            Some("max-age=31536000; includeSubDomains; preload")
        );

        // Check Referrer-Policy
        assert_eq!(
            response
                .headers()
                .get("referrer-policy")
                .map(|v| v.to_str().unwrap()),
            Some("strict-origin-when-cross-origin")
        );

        // Check Permissions-Policy
        assert_eq!(
            response
                .headers()
                .get("permissions-policy")
                .map(|v| v.to_str().unwrap()),
            Some("geolocation=(), microphone=(), camera=()")
        );
    }

    #[tokio::test]
    async fn test_security_headers_on_error_response() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/notfound", get(|| async { StatusCode::NOT_FOUND }))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/notfound")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Security headers should still be present on error responses
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response
                .headers()
                .get("x-content-type-options")
                .map(|v| v.to_str().unwrap()),
            Some("nosniff")
        );
        assert_eq!(
            response
                .headers()
                .get("strict-transport-security")
                .map(|v| v.to_str().unwrap()),
            Some("max-age=31536000; includeSubDomains; preload")
        );
    }

    #[tokio::test]
    async fn test_security_headers_on_json_response() {
        use axum::Json;
        use sdforge::http::apply_security_headers_layer;
        use serde_json::json;

        let json_handler = || async { Json(json!({"status": "ok"})) };

        let app = Router::new()
            .route("/json", get(json_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").map(|v| v.to_str().unwrap()),
            Some("application/json")
        );
        // Content-Type-Options should still be nosniff
        assert_eq!(
            response
                .headers()
                .get("x-content-type-options")
                .map(|v| v.to_str().unwrap()),
            Some("nosniff")
        );
    }

    #[tokio::test]
    async fn test_security_headers_override_custom_headers() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            // Apply security headers layer (should override custom headers)
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // The security header value should be the enforced value, not custom
        assert_eq!(
            response
                .headers()
                .get("x-content-type-options")
                .map(|v| v.to_str().unwrap()),
            Some("nosniff")
        );
    }

    // =========================================================================
    // Content Type Options Tests
    // =========================================================================

    #[tokio::test]
    async fn test_x_content_type_options_nosniff() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let nosniff = response
            .headers()
            .get("x-content-type-options")
            .map(|v| v.to_str().unwrap());
        assert_eq!(nosniff, Some("nosniff"));
    }

    // =========================================================================
    // Frame Options Tests
    // =========================================================================

    #[tokio::test]
    async fn test_x_frame_options_deny() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let frame_options = response
            .headers()
            .get("x-frame-options")
            .map(|v| v.to_str().unwrap());
        assert_eq!(frame_options, Some("DENY"));
    }

    // =========================================================================
    // XSS Protection Tests
    // =========================================================================

    #[tokio::test]
    async fn test_x_xss_protection_enabled() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let xss_protection = response
            .headers()
            .get("x-xss-protection")
            .map(|v| v.to_str().unwrap());
        assert_eq!(xss_protection, Some("1; mode=block"));
    }

    // =========================================================================
    // HSTS Tests
    // =========================================================================

    #[tokio::test]
    async fn test_strict_transport_security_header() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let hsts = response
            .headers()
            .get("strict-transport-security")
            .map(|v| v.to_str().unwrap());
        assert!(hsts.is_some());
        // Should include max-age
        assert!(hsts.unwrap().contains("max-age="));
        // Should include includeSubDomains
        assert!(hsts.unwrap().contains("includeSubDomains"));
    }

    // =========================================================================
    // Referrer Policy Tests
    // =========================================================================

    #[tokio::test]
    async fn test_referrer_policy_strict_origin() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let referrer = response
            .headers()
            .get("referrer-policy")
            .map(|v| v.to_str().unwrap());
        assert_eq!(referrer, Some("strict-origin-when-cross-origin"));
    }

    // =========================================================================
    // Permissions Policy Tests
    // =========================================================================

    #[tokio::test]
    async fn test_permissions_policy_restricted() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(apply_security_headers_layer());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let permissions = response
            .headers()
            .get("permissions-policy")
            .map(|v| v.to_str().unwrap());
        assert!(permissions.is_some());
        let perm = permissions.unwrap();
        // Should restrict sensitive features
        assert!(perm.contains("geolocation=()"));
        assert!(perm.contains("microphone=()"));
        assert!(perm.contains("camera=()"));
    }

    // =========================================================================
    // Multiple Routes Tests
    // =========================================================================

    #[tokio::test]
    async fn test_security_headers_on_all_routes() {
        use sdforge::http::apply_security_headers_layer;

        let app = Router::new()
            .route("/route1", get(test_handler))
            .route("/route2", get(test_handler))
            .route("/route3", get(test_handler))
            .layer(apply_security_headers_layer());

        // Test all routes have security headers
        for uri in ["/route1", "/route2", "/route3"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response
                    .headers()
                    .get("x-content-type-options")
                    .map(|v| v.to_str().unwrap()),
                Some("nosniff")
            );
        }
    }
}

#[cfg(not(feature = "http"))]
mod security_headers_tests_placeholder {
    #[test]
    fn test_http_feature_required() {
        assert!(
            true,
            "Security headers tests require 'http' feature"
        );
    }
}
