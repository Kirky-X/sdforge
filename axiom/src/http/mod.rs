//! HTTP server implementation

use crate::core::ApiMetadata;
use axum::routing::MethodRouter;
use axum::Router;

pub mod version_routing;

pub use version_routing::{VersionedRoute, VersionRouterConfig, build_version_router, version_redirect_middleware};

/// HTTP route registration
#[derive(Debug, Clone)]
pub struct HttpRoute {
    /// Route path (may contain module prefix placeholders)
    pub path: &'static str,
    /// HTTP method
    pub method: axum::http::Method,
    /// Handler function
    pub handler: MethodRouter,
    /// API metadata
    pub metadata: ApiMetadata,
    /// Module prefix (if any) - used for route grouping
    pub module_prefix: Option<&'static str>,
}

inventory::collect!(HttpRoute);

/// Resolve module prefix for a route path
/// 
/// This function checks if there's a module prefix available for the given route.
/// In practice, the macro generates inline path resolution, but this provides
/// a runtime fallback for dynamic path construction.
fn resolve_route_path(base_path: &'static str, module_prefix: Option<&'static str>) -> String {
    match module_prefix {
        Some(prefix) if !prefix.is_empty() => {
            // Remove leading slash from prefix if present
            let clean_prefix = prefix.trim_start_matches('/');
            format!("/{}/{}", clean_prefix, &base_path[1..])
        }
        _ => base_path.to_string(),
    }
}

/// Build HTTP router from registered routes
///
/// This function collects all routes registered via `inventory::submit!`
/// and builds a complete Axum router for serving HTTP requests.
/// Routes are automatically prefixed with their module prefix if available.
#[allow(dead_code)]
pub fn build() -> Router {
    let mut router = Router::new();

    // Group routes by module prefix for cleaner routing
    let mut prefix_groups: std::collections::HashMap<Option<&'static str>, Vec<&HttpRoute>> = 
        std::collections::HashMap::new();

    // Collect all registered routes and group by prefix
    for route in inventory::iter::<HttpRoute> {
        prefix_groups
            .entry(route.module_prefix)
            .or_default()
            .push(route);
    }

    // Build router with route groups
    for (prefix, routes) in prefix_groups {
        for route in routes {
            // Resolve the full path with module prefix
            let full_path = resolve_route_path(route.path, prefix);
            router = router.route(&full_path, route.handler.clone());
        }
    }

    router
}

/// Build HTTP router with version redirect middleware
///
/// This function builds a router with automatic version redirect support.
/// Requests to `/api/{path}` without a version are redirected to `/api/v1/{path}`.
#[allow(dead_code)]
pub fn build_with_redirect() -> Router {
    let router = build();
    router.layer(axum::middleware::from_fn(version_redirect_middleware))
}