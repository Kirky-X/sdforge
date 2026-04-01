// Copyright (c) 2026 Kirky.X
//! Version routing module
//!
//! This module provides version-based routing for the HTTP server.
//! Routes requests based on the API version in the URL path.

use axum::{body::Body, extract::Request, response::Response, routing::MethodRouter, Router};

/// Versioned route structure for API version management
///
/// This structure represents a single API route with its associated version.
/// Routes are registered via `inventory::submit!` and collected by `build_version_router()`.
#[derive(Debug, Clone)]
pub struct VersionedRoute {
    version: String,
    path: String,
    /// HTTP method for this route (reserved for future method-based routing)
    #[allow(dead_code)]
    method: axum::http::Method,
    handler: MethodRouter,
}

impl VersionedRoute {
    /// Create a new versioned route
    ///
    /// # Arguments
    /// * `version` - API version (e.g., "v1")
    /// * `path` - Route path (e.g., "/users/:id")
    /// * `method` - HTTP method
    /// * `handler` - Route handler
    pub fn new(
        version: String,
        path: String,
        method: axum::http::Method,
        handler: MethodRouter,
    ) -> Self {
        Self {
            version,
            path,
            method,
            handler,
        }
    }

    /// Get the API version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the route path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the route handler
    pub fn handler(&self) -> &MethodRouter {
        &self.handler
    }
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
    /// Deprecated versions with sunset dates (version -> sunset_date)
    pub deprecated_versions: std::collections::HashMap<String, String>,
    /// Sunset warning header name
    pub sunset_header: String,
}

impl Default for VersionRouterConfig {
    fn default() -> Self {
        Self {
            default_version: "v1".to_string(),
            supported_versions: vec!["v1".to_string()],
            redirect_unknown: true,
            deprecated_versions: std::collections::HashMap::new(),
            sunset_header: "Sunset".to_string(),
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
        let path = format!("/api/{}{}", route.version(), route.path());
        router = router.route(&path, route.handler().clone());
    }

    router
}

/// Version redirect middleware with deprecation support
pub async fn version_redirect_middleware(
    req: Request<Body>,
    next: axum::middleware::Next,
) -> Response {
    let uri = req.uri().path().to_string();
    let config = VersionRouterConfig::default();

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
                // Valid version, proceed with request and add deprecation headers if needed
                let mut response = next.run(req).await;

                // Check if version is deprecated
                if let Some(sunset_date) = config.deprecated_versions.get(version_part) {
                    // Add Deprecation header
                    response.headers_mut().insert(
                        axum::http::header::HeaderName::from_static("deprecation"),
                        axum::http::HeaderValue::from_str("true").unwrap(),
                    );

                    // Add Sunset header with date
                    response.headers_mut().insert(
                        axum::http::header::HeaderName::from_static("Sunset"),
                        axum::http::HeaderValue::from_str(sunset_date).unwrap(),
                    );

                    // Add Link header to newer version
                    if let Some(newer_version) =
                        find_newer_version(version_part, &config.supported_versions)
                    {
                        let link_header =
                            format!("</api/{}>; rel=\"successor-version\"", newer_version);
                        response.headers_mut().insert(
                            axum::http::header::LINK,
                            axum::http::HeaderValue::from_str(&link_header).unwrap(),
                        );
                    }
                }

                return response;
            }
        }

        // No version or invalid version - redirect to default version
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

/// Find the next newer version
fn find_newer_version(current: &str, supported: &[String]) -> Option<String> {
    let current_num = current[1..].parse::<u32>().ok()?;
    let mut newer: Option<String> = None;

    for version in supported {
        if let Some(num) = version
            .strip_prefix('v')
            .and_then(|v| v.parse::<u32>().ok())
        {
            if num > current_num
                && (newer.is_none()
                    || num < newer.as_ref().and_then(|v| v[1..].parse::<u32>().ok())?)
            {
                newer = Some(version.clone());
            }
        }
    }

    newer
}

/// Create a versioned route helper macro
#[macro_export]
macro_rules! define_versioned_route {
    (version: $version:expr, path: $path:expr, method: $method:ident, handler: $handler:ident) => {
        ::inventory::submit!(sdforge::http::version_routing::VersionedRoute::new(
            $version.to_string(),
            $path.to_string(),
            ::axum::http::Method::$method,
            ::axum::routing::MethodRouter::new().$method($handler),
        ));
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
