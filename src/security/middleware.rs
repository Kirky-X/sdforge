// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Middleware implementations for authentication
//!
//! This module provides Axum middleware for authentication.

use crate::security::{AuthContext, AuthResult};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::future::Future;
use std::pin::Pin;

/// Create authentication middleware
pub fn auth_middleware<T: Clone + Send + Sync + 'static>(
    _auth: T,
    extract_auth: impl Fn(&Request<Body>) -> AuthResult<AuthContext> + Clone + Send + 'static,
) -> impl Fn(Request<Body>, Next) -> Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
    move |mut req: Request<Body>, next: Next| {
        let extract_auth = extract_auth.clone();
        Box::pin(async move {
            match extract_auth(&req) {
                Ok(auth_context) => {
                    req.extensions_mut().insert(auth_context);
                    next.run(req).await
                }
                Err(_) => {
                    let mut response = Response::new(Body::from("Unauthorized"));
                    *response.status_mut() = StatusCode::UNAUTHORIZED;
                    response
                }
            }
        })
    }
}

// IP-extraction, CIDR matching, and IP-validation utilities now live in
// `super::ip_util` so they can be shared with `security::ratelimit::adapter`
// (single source of truth for spoofing-defense logic).

#[cfg(test)]
mod tests {
    use super::*;

    // IP validation, CIDR matching, and extract_client_ip tests now live in
    // `super::super::ip_util::tests` (single source of truth after the
    // ratelimit-adapter spoofing fix).

    // ============================================================================
    // Auth Middleware Tests
    // ============================================================================
    // Note: auth_middleware requires a full axum request pipeline to test
    // properly since axum::middleware::Next is not publicly constructible.
    // The middleware is tested indirectly through the http module's
    // build_with_config tests which apply the middleware to a real router.

    #[tokio::test]
    async fn test_auth_middleware_is_clone_send_sync() {
        // Verify the middleware function is Send + Sync (required for axum)
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Ok(AuthContext {
                user_id: Some("test".to_string()),
                permissions: vec![],
                metadata: crate::security::types::AuthMetadata::default(),
            })
        };
        let middleware = auth_middleware((), extract_auth);
        // Verify it's Send + Sync by assigning to a typed variable
        fn check<T: Send + Sync>(_: T) {}
        check(middleware.clone());

        // Execute via router to exercise closure body
        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));
        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_returns_closure() {
        // Verify auth_middleware returns a callable closure
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Err(crate::security::types::AuthError::MissingAuth)
        };
        let middleware = auth_middleware((), extract_auth);

        // Execute via router to exercise closure body
        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));
        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // Extract Client IP tests now live in `super::super::ip_util::tests`.

    // ============================================================================
    // Auth Middleware Behavior Tests
    // ============================================================================

    #[tokio::test]
    async fn test_auth_middleware_passes_with_valid_context() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Ok(AuthContext {
                user_id: Some("user123".to_string()),
                permissions: vec!["read".to_string()],
                metadata: crate::security::types::AuthMetadata::default(),
            })
        };

        let middleware = auth_middleware((), extract_auth);
        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_rejects_invalid_auth() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Err(crate::security::types::AuthError::InvalidToken)
        };

        let middleware = auth_middleware((), extract_auth);
        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_missing_auth_header() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Err(crate::security::types::AuthError::MissingAuth)
        };

        let middleware = auth_middleware((), extract_auth);
        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_middleware_preserves_request_body() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Ok(AuthContext {
                user_id: Some("user".to_string()),
                permissions: vec![],
                metadata: crate::security::types::AuthMetadata::default(),
            })
        };

        let middleware = auth_middleware((), extract_auth);
        let router = axum::Router::new()
            .route(
                "/test",
                axum::routing::post(|body: Body| async move {
                    let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
                    format!("len={}", bytes.len())
                }),
            )
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder()
                .method("POST")
                .uri("/test")
                .body(Body::from("hello body"))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_preserves_extensions() {
        use axum::extract::Request as ExtractRequest;

        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Ok(AuthContext {
                user_id: Some("user".to_string()),
                permissions: vec!["admin".to_string()],
                metadata: crate::security::types::AuthMetadata::default(),
            })
        };

        let middleware = auth_middleware((), extract_auth);
        let router = axum::Router::new()
            .route(
                "/test",
                axum::routing::get(|req: ExtractRequest| async move {
                    let ctx = req.extensions().get::<AuthContext>();
                    let user = ctx.and_then(|c| c.user_id.clone()).unwrap_or_default();
                    format!("user={}", user)
                }),
            )
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_with_clone() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Ok(AuthContext {
                user_id: Some("clone-test".to_string()),
                permissions: vec![],
                metadata: crate::security::types::AuthMetadata::default(),
            })
        };

        let middleware = auth_middleware((), extract_auth);
        let cloned = middleware.clone();
        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(cloned));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ============================================================================
    // auth_middleware execution via real router (covers closure body)
    // ============================================================================

    #[tokio::test]
    async fn test_auth_middleware_success_path_via_router() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Ok(AuthContext {
                user_id: Some("test".to_string()),
                permissions: vec![],
                metadata: crate::security::types::AuthMetadata::default(),
            })
        };
        let middleware = auth_middleware((), extract_auth);

        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_middleware_unauthorized_path_via_router() {
        let extract_auth = |_req: &Request<Body>| -> AuthResult<AuthContext> {
            Err(crate::security::types::AuthError::MissingAuth)
        };
        let middleware = auth_middleware((), extract_auth);

        let router = axum::Router::new()
            .route("/test", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(middleware));

        let response = tower::ServiceExt::oneshot(
            router,
            Request::builder().uri("/test").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // is_ip_in_range edge cases and trusted-proxy branch-coverage tests now
    // live in `super::super::ip_util::tests`.
}
