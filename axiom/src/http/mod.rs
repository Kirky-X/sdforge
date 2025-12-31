//! HTTP server implementation

use crate::core::ApiMetadata;
use axum::routing::MethodRouter;
use axum::Router;

pub mod version_routing;

pub use version_routing::{VersionedRoute, VersionRouterConfig, build_version_router, version_redirect_middleware};

/// HTTP route registration
#[derive(Debug, Clone)]
pub struct HttpRoute {
    /// Route path
    pub path: &'static str,
    /// HTTP method
    pub method: axum::http::Method,
    /// Handler function
    pub handler: MethodRouter,
    /// API metadata
    pub metadata: ApiMetadata,
}

inventory::collect!(HttpRoute);

/// Build HTTP router from registered routes
///
/// This function collects all routes registered via `inventory::submit!`
/// and builds a complete Axum router for serving HTTP requests.
#[allow(dead_code)]
pub fn build() -> Router {
    let mut router = Router::new();

    // Collect all registered routes
    for route in inventory::iter::<HttpRoute> {
        router = router.route(route.path, route.handler.clone());
    }

    router
}
