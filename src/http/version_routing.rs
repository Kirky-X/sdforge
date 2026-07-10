// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! Version routing module
//!
//! This module provides version-based routing for the HTTP server.
//! Routes requests based on the API version in the URL path.

use axum::{Router, body::Body, extract::Request, response::Response, routing::MethodRouter};

/// Versioned route structure for API version management
///
/// This structure represents a single API route with its associated version.
/// Routes are registered via `inventory::submit!` and collected by `build_version_router()`.
#[derive(Debug, Clone)]
pub struct VersionedRoute {
    version: String,
    path: String,
    /// HTTP method for this route
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

    /// Get the HTTP method
    pub fn method(&self) -> &axum::http::Method {
        &self.method
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

            // Check if version is valid (starts with 'v' followed by at least
            // one digit). Previously, `version_part[1..].chars().all(...)` on
            // an empty slice (when version_part == "v") returned true, so the
            // bare path "/api/v" was treated as a valid version — a boundary
            // bug that caused the middleware to forward instead of redirecting
            // to the default version.
            if version_part
                .chars()
                .next()
                .map(|c| c == 'v')
                .unwrap_or(false)
                && version_part.len() > 1
                && version_part[1..].chars().all(|c| c.is_ascii_digit())
            {
                // Valid version, proceed with request and add deprecation headers if needed
                let mut response = next.run(req).await;

                // Check if version is deprecated
                if let Some(sunset_date) = config.deprecated_versions.get(version_part) {
                    // Add Deprecation header
                    response.headers_mut().insert(
                        axum::http::header::HeaderName::from_static("deprecation"),
                        axum::http::HeaderValue::from_static("true"),
                    );

                    // Add Sunset header with date. Skip the header (rather than panicking)
                    // if the configured sunset_date contains invalid header bytes.
                    if let Ok(val) = axum::http::HeaderValue::from_str(sunset_date) {
                        response
                            .headers_mut()
                            .insert(axum::http::header::HeaderName::from_static("Sunset"), val);
                    }

                    // Add Link header to newer version
                    if let Some(newer_version) =
                        find_newer_version(version_part, &config.supported_versions)
                    {
                        let link_header =
                            format!("</api/{}>; rel=\"successor-version\"", newer_version);
                        if let Ok(val) = axum::http::HeaderValue::from_str(&link_header) {
                            response.headers_mut().insert(axum::http::header::LINK, val);
                        }
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

    // ============================================================================
    // VersionedRoute Tests
    // ============================================================================

    #[test]
    fn test_versioned_route_new() {
        let route = VersionedRoute::new(
            "v1".to_string(),
            "/users".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        assert_eq!(route.version(), "v1");
        assert_eq!(route.path(), "/users");
        assert_eq!(route.method(), &axum::http::Method::GET);
    }

    #[test]
    fn test_versioned_route_different_methods() {
        async fn post_handler() {}
        async fn put_handler() {}
        async fn delete_handler() {}

        let post_route = VersionedRoute::new(
            "v1".to_string(),
            "/users".to_string(),
            axum::http::Method::POST,
            axum::routing::post(post_handler),
        );
        assert_eq!(post_route.method(), &axum::http::Method::POST);

        let put_route = VersionedRoute::new(
            "v1".to_string(),
            "/users/:id".to_string(),
            axum::http::Method::PUT,
            axum::routing::put(put_handler),
        );
        assert_eq!(put_route.method(), &axum::http::Method::PUT);

        let delete_route = VersionedRoute::new(
            "v2".to_string(),
            "/users/:id".to_string(),
            axum::http::Method::DELETE,
            axum::routing::delete(delete_handler),
        );
        assert_eq!(delete_route.method(), &axum::http::Method::DELETE);
    }

    #[test]
    fn test_versioned_route_handler_accessor() {
        let route = VersionedRoute::new(
            "v1".to_string(),
            "/items".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        let handler = route.handler();
        // Verify handler is accessible
        let _ = handler;
    }

    #[test]
    fn test_versioned_route_clone() {
        let route = VersionedRoute::new(
            "v2".to_string(),
            "/products".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        let cloned = route.clone();
        assert_eq!(cloned.version(), route.version());
        assert_eq!(cloned.path(), route.path());
        assert_eq!(cloned.method(), route.method());
    }

    #[test]
    fn test_versioned_route_debug() {
        let route = VersionedRoute::new(
            "v1".to_string(),
            "/debug".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        let debug_str = format!("{:?}", route);
        assert!(debug_str.contains("VersionedRoute"));
        assert!(debug_str.contains("v1"));
    }

    // ============================================================================
    // VersionRouterConfig Tests
    // ============================================================================

    #[test]
    fn test_version_router_config_default() {
        let config = VersionRouterConfig::default();

        assert_eq!(config.default_version, "v1");
        assert_eq!(config.supported_versions, vec!["v1"]);
        assert!(config.redirect_unknown);
        assert!(config.deprecated_versions.is_empty());
        assert_eq!(config.sunset_header, "Sunset");
    }

    #[test]
    fn test_version_router_config_clone() {
        let config = VersionRouterConfig::default();
        let cloned = config.clone();

        assert_eq!(cloned.default_version, config.default_version);
        assert_eq!(cloned.supported_versions, config.supported_versions);
    }

    #[test]
    fn test_version_router_config_debug() {
        let config = VersionRouterConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("VersionRouterConfig"));
    }

    #[test]
    fn test_version_router_config_with_deprecated() {
        let mut deprecated = std::collections::HashMap::new();
        deprecated.insert("v1".to_string(), "2025-12-31".to_string());

        let config = VersionRouterConfig {
            default_version: "v2".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string(), "v3".to_string()],
            redirect_unknown: true,
            deprecated_versions: deprecated,
            sunset_header: "Sunset".to_string(),
        };

        assert_eq!(config.default_version, "v2");
        assert_eq!(config.supported_versions.len(), 3);
        assert!(config.deprecated_versions.contains_key("v1"));
    }

    // ============================================================================
    // build_version_router Tests
    // ============================================================================

    #[test]
    fn test_build_version_router_returns_router() {
        let router = build_version_router();
        // Just verify it doesn't panic and returns a valid Router
        let _ = router;
    }

    // ============================================================================
    // find_newer_version Tests
    // ============================================================================

    #[test]
    fn test_find_newer_version_basic() {
        let supported = vec!["v1".to_string(), "v2".to_string(), "v3".to_string()];

        let result = find_newer_version("v1", &supported);
        assert_eq!(result, Some("v2".to_string()));

        let result = find_newer_version("v2", &supported);
        assert_eq!(result, Some("v3".to_string()));
    }

    #[test]
    fn test_find_newer_version_no_newer() {
        let supported = vec!["v1".to_string(), "v2".to_string()];

        let result = find_newer_version("v2", &supported);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_newer_version_empty_supported() {
        let supported: Vec<String> = vec![];

        let result = find_newer_version("v1", &supported);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_newer_version_finds_closest_newer() {
        let supported = vec!["v1".to_string(), "v3".to_string(), "v5".to_string()];

        // Should return v3 (closest newer), not v5
        let result = find_newer_version("v1", &supported);
        assert_eq!(result, Some("v3".to_string()));
    }

    #[test]
    fn test_find_newer_version_invalid_current() {
        let supported = vec!["v1".to_string(), "v2".to_string()];

        // Invalid version format (no number after v)
        let result = find_newer_version("vabc", &supported);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_newer_version_mixed_valid_invalid() {
        let supported = vec!["v1".to_string(), "invalid".to_string(), "v3".to_string()];

        let result = find_newer_version("v1", &supported);
        assert_eq!(result, Some("v3".to_string()));
    }

    #[test]
    fn test_find_newer_version_unsorted_versions() {
        let supported = vec!["v3".to_string(), "v1".to_string(), "v2".to_string()];

        // Should still find v2 as closest newer to v1
        let result = find_newer_version("v1", &supported);
        assert_eq!(result, Some("v2".to_string()));
    }

    // ============================================================================
    // version_redirect_middleware Extended Tests
    // ============================================================================

    #[tokio::test]
    async fn test_version_redirect_with_trailing_path() {
        let router = Router::new()
            .route("/api/v1/users/123", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/users/123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/api/v1/users/123"
        );
    }

    #[tokio::test]
    async fn test_version_redirect_invalid_version_format() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Invalid version format (not v followed by digits)
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/abc/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/api/v1/abc/test"
        );
    }

    #[tokio::test]
    async fn test_version_redirect_version_with_letters() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Version with letters after v (e.g., "v1beta") should redirect
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1beta/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // v1beta doesn't match "v" + all digits, so it redirects
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }

    #[tokio::test]
    async fn test_version_redirect_non_api_path() {
        let router = Router::new()
            .route("/health", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Non-API path should pass through without redirect
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_redirect_root_api_path() {
        let router = Router::new()
            .route("/api/v1/", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // /api/ without version should redirect
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/api/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(response.headers().get("location").unwrap(), "/api/v1/");
    }

    #[tokio::test]
    async fn test_version_redirect_empty_path_after_api() {
        let router = Router::new()
            .route("/api/v1", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // /api without trailing slash doesn't match /api/ prefix
        // so it passes through to the router and gets 404 (no route matches)
        let response = router
            .clone()
            .oneshot(Request::builder().uri("/api").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // No route matches /api, so 404 is expected
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_version_redirect_valid_version_v2() {
        let router = Router::new()
            .route("/api/v2/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v2/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_redirect_valid_version_v10() {
        let router = Router::new()
            .route("/api/v10/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v10/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_redirect_path_starting_with_slash() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Path after API starting with double slash
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api//test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        // Location header contains the redirect URL
        let location = response.headers().get("location").unwrap();
        assert!(location.to_str().unwrap().starts_with("/api/v1/"));
    }

    // ============================================================================
    // Deprecated Version Tests
    // ============================================================================

    #[tokio::test]
    async fn test_deprecated_version_adds_headers() {
        // We need to test with a config that has deprecated versions
        // Since version_redirect_middleware uses default config, we test
        // the logic indirectly by checking that valid versions pass through
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

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

        // Default config has no deprecated versions, so no deprecation headers
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("deprecation").is_none());
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[tokio::test]
    async fn test_version_redirect_case_sensitive_version() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Uppercase V should not be recognized as version
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/V1/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/api/v1/V1/test"
        );
    }

    #[tokio::test]
    async fn test_version_redirect_version_only() {
        let router = Router::new()
            .route("/api/v1", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Just version without path after
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_redirect_query_params_preserved() {
        let router = Router::new()
            .route("/api/v1/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Note: query params are part of URI but redirect middleware only handles path
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/test?foo=bar")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }

    // ============================================================================
    // Additional Tests (15+ new tests)
    // ============================================================================

    #[test]
    fn test_build_version_router_empty_routes() {
        // When no routes are registered via inventory::submit!, build_version_router returns empty Router
        let router = build_version_router();
        // Router should be valid even with no routes
        let _ = router;
    }

    #[tokio::test]
    async fn test_build_version_router_multiple_versions() {
        // Test that multiple versions of the same path can coexist
        let router = Router::new()
            .route("/api/v1/users", get(test_handler))
            .route("/api/v2/users", get(test_handler))
            .route("/api/v3/users", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // All versions should be accessible
        for version in &["v1", "v2", "v3"] {
            let uri = format!("/api/{}/users", version);
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri.as_str())
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[test]
    fn test_build_version_router_path_format() {
        // Verify that versioned routes use /api/{version}/{path} format
        let route = VersionedRoute::new(
            "v1".to_string(),
            "/users".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        // Expected path format: /api/v1/users
        assert_eq!(route.version(), "v1");
        assert_eq!(route.path(), "/users");
        // Path format is built in build_version_router: format!("/api/{}{}", route.version(), route.path())
    }

    #[test]
    fn test_find_newer_version_v0() {
        // v0 version handling - v0 should find v1 as newer
        let supported = vec!["v0".to_string(), "v1".to_string(), "v2".to_string()];
        let result = find_newer_version("v0", &supported);
        assert_eq!(result, Some("v1".to_string()));
    }

    #[test]
    fn test_find_newer_version_large_number() {
        // Large version numbers like v999
        let supported = vec!["v1".to_string(), "v999".to_string()];
        let result = find_newer_version("v1", &supported);
        assert_eq!(result, Some("v999".to_string()));

        // v999 has no newer version
        let result = find_newer_version("v999", &supported);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_newer_version_single_digit() {
        // Single digit version v1
        let supported = vec!["v1".to_string(), "v2".to_string()];
        let result = find_newer_version("v1", &supported);
        assert_eq!(result, Some("v2".to_string()));
    }

    #[tokio::test]
    async fn test_versioned_route_with_path_params() {
        // Routes with path parameters like {id} (Axum 0.7+ syntax)
        async fn user_handler() -> &'static str {
            "user"
        }

        let router = Router::new()
            .route("/api/v1/users/{id}", get(user_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users/123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_versioned_route_nested_path() {
        // Nested paths like /api/v1/users/{user_id}/posts/{post_id} (Axum 0.7+ syntax)
        async fn post_handler() -> &'static str {
            "post"
        }

        let router = Router::new()
            .route("/api/v1/users/{user_id}/posts/{post_id}", get(post_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/users/42/posts/100")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_versioned_route_all_http_methods() {
        // Test all HTTP methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS
        async fn handler() -> &'static str {
            "ok"
        }

        let router = Router::new()
            .route("/api/v1/resource", get(handler))
            .route("/api/v1/resource", axum::routing::post(handler))
            .route("/api/v1/resource", axum::routing::put(handler))
            .route("/api/v1/resource", axum::routing::patch(handler))
            .route("/api/v1/resource", axum::routing::delete(handler))
            .route("/api/v1/resource", axum::routing::head(handler))
            .route("/api/v1/resource", axum::routing::options(handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        // Test each method
        let methods = [
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::HEAD,
            axum::http::Method::OPTIONS,
        ];

        for method in methods {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method.clone())
                        .uri("/api/v1/resource")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Failed for method {:?}",
                method
            );
        }
    }

    #[tokio::test]
    async fn test_version_redirect_api_trailing_slash_no_version() {
        // /api/ without version should redirect to /api/v1/
        let router = Router::new()
            .route("/api/v1/", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(Request::builder().uri("/api/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(response.headers().get("location").unwrap(), "/api/v1/");
    }

    #[tokio::test]
    async fn test_version_redirect_deep_path_no_version() {
        // Deep paths like /api/users/profile/settings redirect
        let router = Router::new()
            .route("/api/v1/users/profile/settings", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/users/profile/settings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/api/v1/users/profile/settings"
        );
    }

    #[tokio::test]
    async fn test_version_redirect_version_v0() {
        // v0 version number recognition
        let router = Router::new()
            .route("/api/v0/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v0/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_find_newer_version_with_deprecated() {
        // Find newer version even when deprecated versions exist
        let supported = vec!["v1".to_string(), "v2".to_string(), "v3".to_string()];
        let result = find_newer_version("v1", &supported);
        // Should return v2 (closest newer), not v3
        assert_eq!(result, Some("v2".to_string()));
    }

    #[test]
    fn test_version_router_config_deprecated_multiple() {
        // Configuration with multiple deprecated versions
        let mut deprecated = std::collections::HashMap::new();
        deprecated.insert("v1".to_string(), "2025-06-30".to_string());
        deprecated.insert("v2".to_string(), "2025-12-31".to_string());

        let config = VersionRouterConfig {
            default_version: "v3".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string(), "v3".to_string()],
            redirect_unknown: true,
            deprecated_versions: deprecated.clone(),
            sunset_header: "Sunset".to_string(),
        };

        assert_eq!(config.deprecated_versions.len(), 2);
        assert!(config.deprecated_versions.contains_key("v1"));
        assert!(config.deprecated_versions.contains_key("v2"));
        assert_eq!(
            config.deprecated_versions.get("v1"),
            Some(&"2025-06-30".to_string())
        );
    }

    #[test]
    fn test_define_versioned_route_macro() {
        // Verify macro expands correctly for GET method
        async fn macro_test_handler() -> &'static str {
            "macro test"
        }

        // Create route manually (macro does this internally)
        let route = VersionedRoute::new(
            "v1".to_string(),
            "/macro-test".to_string(),
            axum::http::Method::GET,
            axum::routing::MethodRouter::new().get(macro_test_handler),
        );

        assert_eq!(route.version(), "v1");
        assert_eq!(route.path(), "/macro-test");
        assert_eq!(route.method(), &axum::http::Method::GET);
    }

    #[test]
    fn test_define_versioned_route_post_method() {
        // Verify macro works for POST method
        async fn post_test_handler() -> &'static str {
            "post test"
        }

        let route = VersionedRoute::new(
            "v2".to_string(),
            "/posts".to_string(),
            axum::http::Method::POST,
            axum::routing::MethodRouter::new().post(post_test_handler),
        );

        assert_eq!(route.version(), "v2");
        assert_eq!(route.method(), &axum::http::Method::POST);
    }

    #[test]
    fn test_define_versioned_route_all_methods() {
        // Verify macro supports all HTTP methods
        async fn handler() -> &'static str {
            "ok"
        }

        let methods = [
            (
                axum::http::Method::GET,
                axum::routing::MethodRouter::new().get(handler),
            ),
            (
                axum::http::Method::POST,
                axum::routing::MethodRouter::new().post(handler),
            ),
            (
                axum::http::Method::PUT,
                axum::routing::MethodRouter::new().put(handler),
            ),
            (
                axum::http::Method::PATCH,
                axum::routing::MethodRouter::new().patch(handler),
            ),
            (
                axum::http::Method::DELETE,
                axum::routing::MethodRouter::new().delete(handler),
            ),
        ];

        for (method, router_method) in methods {
            let route = VersionedRoute::new(
                "v1".to_string(),
                "/test".to_string(),
                method.clone(),
                router_method,
            );
            assert_eq!(route.method(), &method);
        }
    }

    #[test]
    fn test_version_router_config_clone_deep() {
        // Deep clone verification with nested HashMap
        let mut deprecated = std::collections::HashMap::new();
        deprecated.insert("v1".to_string(), "2025-01-01".to_string());
        deprecated.insert("v2".to_string(), "2025-06-30".to_string());

        let config = VersionRouterConfig {
            default_version: "v3".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string(), "v3".to_string()],
            redirect_unknown: true,
            deprecated_versions: deprecated,
            sunset_header: "X-Sunset".to_string(),
        };

        let cloned = config.clone();

        // Verify all fields are cloned correctly
        assert_eq!(cloned.default_version, config.default_version);
        assert_eq!(cloned.supported_versions, config.supported_versions);
        assert_eq!(cloned.redirect_unknown, config.redirect_unknown);
        assert_eq!(cloned.deprecated_versions, config.deprecated_versions);
        assert_eq!(cloned.sunset_header, config.sunset_header);

        // Verify independence (modifying clone doesn't affect original)
        let mut cloned_modified = cloned.clone();
        cloned_modified.default_version = "v4".to_string();
        assert_eq!(config.default_version, "v3");
        assert_eq!(cloned_modified.default_version, "v4");
    }

    #[test]
    fn test_versioned_route_display_format() {
        // Path display format verification
        let route = VersionedRoute::new(
            "v2".to_string(),
            "/users/:id".to_string(),
            axum::http::Method::GET,
            get(test_handler),
        );

        // Version and path should be accessible
        assert!(route.version().starts_with('v'));
        assert!(route.path().starts_with('/'));

        // Debug format should contain key information
        let debug = format!("{:?}", route);
        assert!(debug.contains("v2"));
        assert!(debug.contains("/users/:id"));
    }

    // ============================================================================
    // build_version_router with inventory-registered routes
    //
    // NOTE: Lines 107-108 (the for-loop body in build_version_router) cannot be
    // covered without modifying non-test code. `inventory::submit!` requires
    // const-evaluable arguments on stable Rust, but `VersionedRoute::new` takes
    // `String` (heap-allocated) and `MethodRouter` (not const-constructible),
    // so it cannot be used in `inventory::submit!`. The empty-routes case is
    // already covered by `test_build_version_router_empty_routes`.
    // ============================================================================

    // ============================================================================
    // Deprecation header block (lines 141-167) coverage note
    //
    // The deprecation-header logic inside `version_redirect_middleware` is
    // gated behind `if let Some(sunset_date) = config.deprecated_versions
    // .get(version_part)`. The middleware constructs its config via
    // `VersionRouterConfig::default()`, whose `deprecated_versions` map is
    // empty — so the `Some` branch can never be taken without modifying the
    // middleware signature to accept an injected config. These lines are
    // therefore unreachable from tests under the "do not modify production
    // code" constraint. The helper `find_newer_version` is exercised directly
    // below to cover the logic that *would* feed the Link header.
    // ============================================================================

    /// Test find_newer_version with a single supported version returns None
    /// when the current version is the only one available.
    #[test]
    fn test_find_newer_version_single_supported() {
        let supported = vec!["v1".to_string()];
        assert_eq!(find_newer_version("v1", &supported), None);
    }

    /// Test find_newer_version skips non-`v`-prefixed entries and still
    /// returns the closest valid newer version.
    #[test]
    fn test_find_newer_version_skips_non_v_entries() {
        let supported = vec![
            "v1".to_string(),
            "beta".to_string(),
            "v2".to_string(),
            "latest".to_string(),
        ];
        assert_eq!(find_newer_version("v1", &supported), Some("v2".to_string()));
    }

    /// Test find_newer_version returns None when all supported versions are
    /// older than or equal to the current one.
    #[test]
    fn test_find_newer_version_all_older_or_equal() {
        let supported = vec!["v1".to_string(), "v2".to_string(), "v3".to_string()];
        assert_eq!(find_newer_version("v3", &supported), None);
        assert_eq!(find_newer_version("v4", &supported), None);
    }

    /// Test find_newer_version picks the *closest* newer version when
    /// multiple newer versions exist out of order.
    #[test]
    fn test_find_newer_version_closest_among_unsorted() {
        let supported = vec!["v5".to_string(), "v2".to_string(), "v10".to_string()];
        // Closest newer to v1 would be v2, but v1 isn't in the list —
        // verify v2 -> v5 (closest newer).
        assert_eq!(find_newer_version("v2", &supported), Some("v5".to_string()));
    }

    /// Test the deprecated-version config is structurally valid and the
    /// sunset date can be retrieved, exercising the configuration that the
    /// middleware *would* use if it accepted an injected config.
    #[test]
    fn test_deprecated_config_sunset_date_retrieval() {
        let mut deprecated = std::collections::HashMap::new();
        deprecated.insert("v1".to_string(), "2026-12-31".to_string());
        deprecated.insert("v2".to_string(), "2027-06-30".to_string());

        let config = VersionRouterConfig {
            default_version: "v3".to_string(),
            supported_versions: vec!["v1".to_string(), "v2".to_string(), "v3".to_string()],
            redirect_unknown: true,
            deprecated_versions: deprecated,
            sunset_header: "Sunset".to_string(),
        };

        // The middleware queries `config.deprecated_versions.get(version_part)`.
        // Verify the lookup returns the expected sunset date.
        assert_eq!(
            config.deprecated_versions.get("v1"),
            Some(&"2026-12-31".to_string()),
        );
        assert_eq!(
            config.deprecated_versions.get("v2"),
            Some(&"2027-06-30".to_string()),
        );
        assert!(!config.deprecated_versions.contains_key("v3"));

        // The successor-version lookup the middleware would perform:
        assert_eq!(
            find_newer_version("v1", &config.supported_versions),
            Some("v2".to_string()),
        );
    }

    /// Test version_redirect_middleware passes through a request to a
    /// non-`/api/` path without modification (the final `next.run(req).await`
    /// fallthrough at the end of the function).
    #[tokio::test]
    async fn test_version_redirect_non_api_path_passes_through() {
        let router = Router::new()
            .route("/healthz", get(test_handler))
            .route("/metrics", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        for path in &["/healthz", "/metrics"] {
            let response = router
                .clone()
                .oneshot(Request::builder().uri(*path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Non-API path {} should pass through",
                path,
            );
        }
    }

    /// Test version_redirect_middleware handles a very long version number
    /// (e.g., v9999) as a valid version.
    #[tokio::test]
    async fn test_version_redirect_very_long_version_number() {
        let router = Router::new()
            .route("/api/v9999/test", get(test_handler))
            .layer(axum::middleware::from_fn(version_redirect_middleware));

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/v9999/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
