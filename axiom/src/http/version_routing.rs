// Copyright (c) 2026 Kirky.X
//! Version routing module
//!
//! This module provides version-based routing for the HTTP server.
//! Routes requests based on the API version in the URL path.

use axum::{body::Body, extract::Request, response::Response, routing::MethodRouter, Router};

/// Versioned route configuration
#[derive(Debug, Clone)]
pub struct VersionedRoute {
    /// Version prefix (e.g., "v1", "v2")
    pub version: String,
    /// Route path (without version prefix)
    pub path: String,
    /// HTTP method
    pub method: axum::http::Method,
    /// Handler function
    pub handler: MethodRouter,
}

/// Version routing configuration
#[derive(Debug, Clone)]
pub struct VersionRouterConfig {
    /// Default version if none specified
    pub default_version: String,
    /// Supported versions
    pub supported_versions: Vec<String>,
    /// Enable version redirect (redirect /api/foo to /api/v1/foo)
    pub redirect_unknown: bool,
}

impl Default for VersionRouterConfig {
    fn default() -> Self {
        Self {
            default_version: "v1".to_string(),
            supported_versions: vec!["v1".to_string()],
            redirect_unknown: true,
        }
    }
}

inventory::collect!(VersionedRoute);

/// Build a versioned router for API routes
///
/// This function collects all routes registered via `inventory::submit!` with
/// their associated versions and builds an Axum router with properly versioned
/// paths in the format `/api/{version}/{path}`.
///
/// # Returns
/// An Axum Router with all versioned API routes registered
pub fn build_version_router() -> Router {
    let mut router = Router::new();

    // Collect all versioned routes
    for route in inventory::iter::<VersionedRoute> {
        let path = format!("/api/{}{}", route.version, route.path);
        router = router.route(&path, route.handler.clone());
    }

    router
}

/// Version redirect middleware
pub async fn version_redirect_middleware(
    req: Request<Body>,
    next: axum::middleware::Next,
) -> Response {
    let uri = req.uri().path().to_string();

    // Check if path starts with /api/ and has a version
    if let Some(path_after_api) = uri.strip_prefix("/api/") {
        // Check if it has a version (v1, v2, etc.)
        if path_after_api.starts_with("v") {
            let end_of_version = path_after_api.find('/').unwrap_or(path_after_api.len());
            let version_part = &path_after_api[..end_of_version];

            // Check if version is valid (starts with v followed by digits)
            if version_part
                .chars()
                .next()
                .map(|c| c == 'v')
                .unwrap_or(false)
                && version_part[1..].chars().all(|c| c.is_ascii_digit())
            {
                // Valid version, proceed with request
                return next.run(req).await;
            }
        }

        // No version or invalid version - redirect to default version
        let config = VersionRouterConfig::default();
        let default_version = &config.default_version;
        let path_without_version = if path_after_api.starts_with('/') {
            path_after_api.to_string()
        } else {
            format!("/{}", path_after_api)
        };
        let new_uri = format!("/api/{}{}", default_version, path_without_version);

        let mut response = Response::new(Body::empty());
        *response.status_mut() = axum::http::StatusCode::MOVED_PERMANENTLY;
        response.headers_mut().insert(
            axum::http::header::LOCATION,
            axum::http::HeaderValue::from_str(&new_uri)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("/")),
        );
        return response;
    }

    // Not an API path, proceed with request
    next.run(req).await
}

/// Create a versioned route helper macro
#[macro_export]
macro_rules! define_versioned_route {
    (version: $version:expr, path: $path:expr, method: $method:ident, handler: $handler:ident) => {
        ::inventory::submit!(axiom::http::version_routing::VersionedRoute {
            version: $version.to_string(),
            path: $path.to_string(),
            method: ::axum::http::Method::$method,
            handler: ::axum::routing::MethodRouter::new().$method($handler),
        });
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "test response"
    }

    #[tokio::test]
    async fn test_version_redirect() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Test redirect from /api/test to /api/v1/test
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/test")
                    .body(Body::empty())
                    .expect("Failed to build request"),
            )
            .await
            .expect("Failed to handle request");

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            response
                .headers()
                .get("location")
                .expect("Location header not found"),
            "/api/v1/test"
        );
    }

    #[tokio::test]
    async fn test_valid_version_passes() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Test valid version passes through
        let response = router
            .clone()
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
}
