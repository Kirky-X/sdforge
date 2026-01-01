//! HTTP server implementation

use crate::core::ApiMetadata;
use axum::routing::MethodRouter;
use axum::Router;
use axum::body::Body;

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

/// Build HTTP router with configuration
///
/// This function builds a router with configuration-driven middleware and settings.
/// Applies CORS, authentication, rate limiting, and logging based on the provided config.
///
/// # Arguments
/// * `config` - The application configuration
///
/// # Returns
/// A configured Axum router with all middleware applied
#[allow(dead_code)]
pub fn build_with_config(config: &crate::config::AppConfig) -> Result<Router, crate::config::ConfigError> {
    use crate::security::{RateLimiter, RateLimitConfig, rate_limit_middleware};
    use std::sync::Arc;
    use std::convert::TryFrom;
    
    let mut router = build();
    
    // Apply CORS
    if let Some(cors) = &config.server.cors {
        let cors_layer = crate::config::build_cors_layer(cors)?;
        router = router.layer(cors_layer);
    }
    
    // Apply rate limiting middleware
    if let Some(rate_limit) = &config.rate_limit {
        let rate_config = RateLimitConfig::try_from(rate_limit.clone())?;
        let limiter = Arc::new(RateLimiter::new(Some(rate_config)));
        let middleware = rate_limit_middleware(limiter);
        router = router.layer(axum::middleware::from_fn(middleware));
    }
    
    // Apply authentication middleware
    if let Some(auth_config) = &config.authentication {
        use crate::security::{ApiKeyAuth, BearerAuth, AuthContext, AuthError, auth_middleware};
        use axum::http::HeaderValue;
        
        if let crate::config::AuthConfig::ApiKey { header_name, prefix } = auth_config {
            let auth = Arc::new(ApiKeyAuth::new());
            let auth_clone = auth.clone();
            let header_name = header_name.clone();
            let prefix = prefix.clone();
            let extract_auth = move |req: &axum::http::Request<Body>| -> Result<AuthContext, AuthError> {
                let header_value = req.headers().get(&header_name)
                    .and_then(|v: &HeaderValue| v.to_str().ok())
                    .unwrap_or("");
                
                if header_value.starts_with(&prefix) {
                    let key = &header_value[prefix.len()..];
                    if let Some(permissions) = auth.validate_key(key) {
                        Ok(AuthContext {
                            user_id: Some(key.to_string()),
                            permissions,
                            metadata: crate::security::AuthMetadata::default(),
                        })
                    } else {
                        Err(AuthError::MissingAuth)
                    }
                } else {
                    Err(AuthError::MissingAuth)
                }
            };
            let middleware = auth_middleware(auth_clone, extract_auth);
            router = router.layer(axum::middleware::from_fn(middleware));
        } else if let crate::config::AuthConfig::Jwt { secret, .. } = auth_config {
            let auth = Arc::new(BearerAuth::new(secret));
            let auth_clone = auth.clone();
            let extract_auth = move |req: &axum::http::Request<Body>| -> Result<AuthContext, AuthError> {
                let header_value = req.headers().get("authorization")
                    .and_then(|v: &HeaderValue| v.to_str().ok())
                    .unwrap_or("");
                
                if let Some(token) = header_value.strip_prefix("Bearer ") {
                    if let Some(context) = auth.validate_token(token) {
                        Ok(context)
                    } else {
                        Err(AuthError::InvalidToken)
                    }
                } else {
                    Err(AuthError::MissingAuth)
                }
            };
            let middleware = auth_middleware(auth_clone, extract_auth);
            router = router.layer(axum::middleware::from_fn(middleware));
        } else {
            return Err(crate::config::ConfigError::ValidationError("OAuth2 not yet implemented".into()));
        }
    }
    
    // Initialize logging
    #[cfg(feature = "logging")]
    if let Some(logging) = &config.logging {
        crate::config::init_logging(logging);
    }
    
    Ok(router)
}