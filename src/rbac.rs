// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Endpoint-level role-based access control.
//!
//! `#[forge(auth(role = "admin"))]` attaches a declared role requirement to
//! the generated HTTP route. The macro wraps the route's `MethodRouter` with
//! [`require_role`]; requests whose [`AuthContext`] lacks every declared role
//! are rejected with **403 Forbidden** (never 401 — authentication happened
//! upstream in the global auth middleware).
//!
//! Fail-safe defaults:
//!
//! - No `AuthContext` extension present (auth middleware not installed) → 403.
//! - `security` feature disabled → the endpoint denies every request (403);
//!   role requirements can never be satisfied without an authentication stack.

/// Allowed roles for an endpoint; a request passes when its `AuthContext`
/// permissions contain at least one of them.
pub type Roles = &'static [&'static str];

/// 403 body shared by both implementations (stable shape for clients).
///
/// rendered through the unified error contract — carries the ambient
/// `trace_id` when the `context` feature is active.
fn forbidden(roles: Roles) -> axum::response::Response {
    let err = crate::error::unified::UnifiedError::new(
        "FORBIDDEN",
        format!("missing required role: {}", roles.join(", ")),
    );
    crate::error::unified::render_http(axum::http::StatusCode::FORBIDDEN, &err)
}

/// Wrap a route's `MethodRouter` with an endpoint-level role requirement.
///
/// Requires the `security` feature: the declared roles are matched against
/// the `AuthContext` inserted by the global auth middleware.
#[cfg(feature = "security")]
pub fn require_role(router: axum::routing::MethodRouter, roles: Roles) -> axum::routing::MethodRouter {
    use crate::security::AuthContext;
    router.layer(axum::middleware::from_fn(
        move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| async move {
            let authorized = req
                .extensions()
                .get::<AuthContext>()
                .map(|ctx| roles.iter().any(|r| ctx.has_permission(r)))
                .unwrap_or(false);
            if authorized {
                next.run(req).await
            } else {
                forbidden(roles)
            }
        },
    ))
}

/// Fail-safe fallback without the `security` feature: endpoints that declare
/// a role requirement deny every request (a role can never be verified).
#[cfg(not(feature = "security"))]
pub fn require_role(router: axum::routing::MethodRouter, roles: Roles) -> axum::routing::MethodRouter {
    router.layer(axum::middleware::from_fn(
        move |_req: axum::http::Request<axum::body::Body>, _next: axum::middleware::Next| async move {
            forbidden(roles)
        },
    ))
}

#[cfg(all(test, feature = "security"))]
mod tests {
    use super::*;
    use crate::security::{AuthContext, AuthMetadata};
    use axum::body::Body;
    use axum::response::IntoResponse;
    use tower::ServiceExt;

    async fn probe() -> impl IntoResponse {
        (axum::http::StatusCode::OK, "ok")
    }

    #[tokio::test]
    async fn missing_auth_context_denies() {
        // Direct require_role check via extension absence is covered by the
        // e2e suite; here we assert the deny helper shape.
        let resp = forbidden(&["admin"]);
        assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn require_role_passes_matching_context() {
        let method_router = axum::routing::MethodRouter::new().get(probe);
        let method_router = require_role(method_router, &["admin", "ops"]);
        let router = axum::Router::new().route(
            "/admin",
            method_router.layer(axum::middleware::from_fn(
                |mut req: axum::http::Request<Body>, next: axum::middleware::Next| async move {
                    req.extensions_mut().insert(AuthContext {
                        user_id: None,
                        permissions: vec!["ops".into()],
                        metadata: AuthMetadata::default(),
                    });
                    next.run(req).await
                },
            )),
        );
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/admin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn require_role_denies_non_matching_context() {
        let method_router = axum::routing::MethodRouter::new().get(probe);
        let method_router = require_role(method_router, &["admin"]);
        let router = axum::Router::new().route(
            "/admin",
            method_router.layer(axum::middleware::from_fn(
                |mut req: axum::http::Request<Body>, next: axum::middleware::Next| async move {
                    req.extensions_mut().insert(AuthContext {
                        user_id: None,
                        permissions: vec!["viewer".into()],
                        metadata: AuthMetadata::default(),
                    });
                    next.run(req).await
                },
            )),
        );
        let resp = router
            .oneshot(
                axum::http::Request::builder()
                    .uri("/admin")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    }
}
