// Security Middleware Integration Tests
// Tests auth_middleware and rate_limit_middleware with actual HTTP requests

#[cfg(feature = "security")]
mod security_tests {
    use axum::{routing::get, Router};
    use sdforge::security::{
        AppApiKeyAuth, AppRateLimiter, AuthContext, AuthError, AuthResult, RateLimitConfig,
    };
    use std::sync::Arc;

    /// Test handler for authenticated endpoints
    async fn protected_handler() -> &'static str {
        "Secret data"
    }

    /// Test handler for rate-limited endpoints
    async fn rate_limited_handler() -> &'static str {
        "Rate limited data"
    }

    /// Simple extract_auth function for testing
    fn extract_test_auth(
        req: &axum::http::Request<axum::body::Body>,
    ) -> AuthResult<AuthContext> {
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..];
                if token == "valid-token" {
                    Ok(AuthContext::new(
                        Some("user123".to_string()),
                        vec!["read".to_string()],
                        Default::default(),
                    ))
                } else {
                    Err(AuthError::InvalidToken)
                }
            }
            Some(_) => Err(AuthError::MissingAuth),
            None => Err(AuthError::MissingAuth),
        }
    }

    /// Helper to build a test request
    fn build_request(method: &str, path: &str) -> axum::http::Request<axum::body::Body> {
        let mut req = axum::http::Request::builder()
            .method(method)
            .uri(path);
        req
    }

    // =========================================================================
    // Authentication Middleware Tests
    // =========================================================================

    #[tokio::test]
    async fn test_auth_middleware_valid_token() {
        use sdforge::security::auth_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let auth = Arc::new(AppApiKeyAuth::new());
        let extract_auth = |req: &Request<Body>| {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = &header[7..];
                    if token == "valid-token" {
                        Ok(AuthContext::new(
                            Some("user123".to_string()),
                            vec!["read".to_string()],
                            Default::default(),
                        ))
                    } else {
                        Err(AuthError::InvalidToken)
                    }
                }
                _ => Err(AuthError::MissingAuth),
            }
        };

        let middleware = auth_middleware(auth, extract_auth);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer valid-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_invalid_token() {
        use sdforge::security::auth_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let auth = Arc::new(AppApiKeyAuth::new());
        let extract_auth = |req: &Request<Body>| {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = &header[7..];
                    if token == "valid-token" {
                        Ok(AuthContext::new(
                            Some("user123".to_string()),
                            vec!["read".to_string()],
                            Default::default(),
                        ))
                    } else {
                        Err(AuthError::InvalidToken)
                    }
                }
                _ => Err(AuthError::MissingAuth),
            }
        };

        let middleware = auth_middleware(auth, extract_auth);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer invalid-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_token() {
        use sdforge::security::auth_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let auth = Arc::new(AppApiKeyAuth::new());
        let extract_auth = |_req: &Request<Body>| Err(AuthError::MissingAuth);

        let middleware = auth_middleware(auth, extract_auth);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // =========================================================================
    // Rate Limiting Middleware Tests
    // =========================================================================

    #[tokio::test]
    async fn test_rate_limit_middleware_allowed() {
        use sdforge::security::rate_limit_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let config = RateLimitConfig {
            max_requests: 100,
            window: std::time::Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = Arc::new(AppRateLimiter::new(config));

        let middleware = rate_limit_middleware(limiter);

        let app = Router::new()
            .route("/api", get(rate_limited_handler))
            .layer(axum::middleware::from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().contains_key("X-RateLimit-Limit"));
        assert!(response.headers().contains_key("X-RateLimit-Remaining"));
    }

    #[tokio::test]
    async fn test_rate_limit_middleware_exceeded() {
        use sdforge::security::rate_limit_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        // Very restrictive config
        let config = RateLimitConfig {
            max_requests: 1,
            window: std::time::Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = Arc::new(AppRateLimiter::new(config));

        let middleware = rate_limit_middleware(limiter);

        let app = Router::new()
            .route("/api", get(rate_limited_handler))
            .layer(axum::middleware::from_fn(middleware));

        // First request should succeed
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response1.status(), StatusCode::OK);

        // Second request should be rate limited
        let response2 = app
            .oneshot(
                Request::builder()
                    .uri("/api")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response2.headers().contains_key("X-RateLimit-Limit"));
        assert!(response2.headers().contains_key("Retry-After"));
    }

    // =========================================================================
    // Combined Auth and Rate Limit Middleware Tests
    // =========================================================================

    #[tokio::test]
    async fn test_auth_then_rate_limit_middleware() {
        use sdforge::security::{auth_middleware, rate_limit_middleware};
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let auth = Arc::new(AppApiKeyAuth::new());
        let auth_extract = |req: &Request<Body>| {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok());

            match auth_header {
                Some(header) if header.starts_with("Bearer ") => {
                    let token = &header[7..];
                    if token == "valid-token" {
                        Ok(AuthContext::new(
                            Some("user123".to_string()),
                            vec!["read".to_string()],
                            Default::default(),
                        ))
                    } else {
                        Err(AuthError::InvalidToken)
                    }
                }
                _ => Err(AuthError::MissingAuth),
            }
        };

        let config = RateLimitConfig {
            max_requests: 100,
            window: std::time::Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = Arc::new(AppRateLimiter::new(config));

        let auth_mw = auth_middleware(auth, auth_extract);
        let rate_limit_mw = rate_limit_middleware(limiter);

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(axum::middleware::from_fn(rate_limit_mw))
            .layer(axum::middleware::from_fn(auth_mw));

        // Valid token should pass
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer valid-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[tokio::test]
    async fn test_auth_with_empty_body() {
        use sdforge::security::auth_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let auth = Arc::new(AppApiKeyAuth::new());
        let extract_auth = |_req: &Request<Body>| {
            Ok(AuthContext::new(
                Some("test-user".to_string()),
                vec![],
                Default::default(),
            ))
        };

        let middleware = auth_middleware(auth, extract_auth);

        let app = Router::new()
            .route("/empty", get(protected_handler))
            .layer(axum::middleware::from_fn(middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/empty")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_rate_limit_different_ips() {
        use sdforge::security::rate_limit_middleware;
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        // Very restrictive config - 1 request per window
        let config = RateLimitConfig {
            max_requests: 1,
            window: std::time::Duration::from_secs(60),
            include_headers: true,
        };
        let limiter = Arc::new(AppRateLimiter::new(config));

        let middleware = rate_limit_middleware(limiter);

        let app = Router::new()
            .route("/api", get(rate_limited_handler))
            .layer(axum::middleware::from_fn(middleware));

        // Request from IP1 - should succeed
        let response1 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api")
                    .header("X-Real-IP", "192.168.1.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response1.status(), StatusCode::OK);

        // Request from IP2 - should also succeed (different IP = different rate limit)
        let response2 = app
            .oneshot(
                Request::builder()
                    .uri("/api")
                    .header("X-Real-IP", "192.168.1.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response2.status(), StatusCode::OK);
    }
}

#[cfg(not(feature = "security"))]
mod security_tests_placeholder {
    #[test]
    fn test_security_feature_required() {
        assert!(
            true,
            "Security middleware tests require 'security' feature"
        );
    }
}
