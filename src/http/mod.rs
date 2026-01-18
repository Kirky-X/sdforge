// Copyright (c) 2026 Kirky.X
//! HTTP server implementation

use crate::core::ApiMetadata;
use axum::body::Body;
use axum::routing::MethodRouter;
use axum::Router;

pub mod version_routing;

pub use version_routing::{
    build_version_router, version_redirect_middleware, VersionRouterConfig, VersionedRoute,
};

/// HTTP route registration
#[derive(Debug, Clone)]
pub struct HttpRoute {
    /// Route path (may contain module prefix placeholders)
    pub path: String,
    /// Handler function (includes method routing)
    pub handler: MethodRouter,
    /// API metadata
    pub metadata: ApiMetadata,
    /// Module prefix (if any) - used for route grouping
    pub module_prefix: Option<String>,
}

impl HttpRoute {
    /// Get route path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get handler
    pub fn handler(&self) -> &MethodRouter {
        &self.handler
    }

    /// Get metadata
    pub fn metadata(&self) -> &ApiMetadata {
        &self.metadata
    }

    /// Get module prefix
    pub fn module_prefix(&self) -> Option<&str> {
        self.module_prefix.as_deref()
    }
}

/// Route registration for runtime initialization
///
/// This struct stores a function pointer that creates an HttpRoute at runtime.
/// This allows routes to be registered without requiring const-compatible types.
#[derive(Debug, Clone)]
pub struct RouteRegistration {
    /// API name
    pub name: &'static str,
    /// API version
    pub version: &'static str,
    /// Function that creates the HttpRoute at runtime
    pub register_fn: fn() -> HttpRoute,
}

inventory::collect!(HttpRoute);
inventory::collect!(RouteRegistration);

/// Resolve module prefix for a route path
///
/// This function checks if there's a module prefix available for the given route.
/// In practice, the macro generates inline path resolution, but this provides
/// a runtime fallback for dynamic path construction.
fn resolve_route_path(base_path: &str, module_prefix: Option<&str>) -> String {
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

    // First, collect registrations with function pointers
    let registrations: Vec<_> = inventory::iter::<RouteRegistration>().collect();

    // Sort by name and version for deterministic order
    let mut routes: Vec<_> = registrations
        .iter()
        .map(|registration| (registration.register_fn)())
        .collect();

    // Also collect direct HttpRoute registrations
    for route in inventory::iter::<HttpRoute>() {
        routes.push(route.clone());
    }

    // Sort routes by path for deterministic order
    routes.sort_by_key(|r| r.path.clone());

    // Build router with routes
    for route in routes {
        let prefix = route.module_prefix.as_deref();
        let full_path = resolve_route_path(&route.path, prefix);
        router = router.route(&full_path, route.handler);
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
pub fn build_with_config(
    config: &crate::config::AppConfig,
) -> Result<Router, crate::config::ConfigError> {
    #[cfg(feature = "security")]
    use crate::security::{rate_limit_middleware, RateLimitConfig, RateLimiter};
    #[cfg(feature = "security")]
    use std::sync::Arc;

    const DEFAULT_BODY_LIMIT: usize = 10 * 1024 * 1024;

    let mut router = build();

    router = router.layer(tower_http::limit::RequestBodyLimitLayer::new(
        DEFAULT_BODY_LIMIT,
    ));

    // Use configurable request timeout from server config
    let timeout_secs = config.server.request_timeout_secs;
    router = router.layer(tower_http::timeout::TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        std::time::Duration::from_secs(timeout_secs),
    ));

    // Apply CORS
    if let Some(cors) = &config.server.cors {
        let cors_layer = crate::config::build_cors_layer(cors)?;
        router = router.layer(cors_layer);
    }

    // Apply security headers
    router = router.layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    ));
    router = router.layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("DENY"),
    ));
    router = router.layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::header::X_XSS_PROTECTION,
        axum::http::HeaderValue::from_static("1; mode=block"),
    ));
    router = router.layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    ));
    // Security fix: Add Content-Security-Policy header to prevent XSS attacks
    router = router.layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
        axum::http::header::CONTENT_SECURITY_POLICY,
        axum::http::HeaderValue::from_static("default-src 'self'"),
    ));

    // Apply rate limiting middleware
    #[cfg(feature = "security")]
    if let Some(rate_limit) = &config.rate_limit {
        let rate_config = RateLimitConfig::try_from(rate_limit.clone())?;
        let limiter = Arc::new(RateLimiter::new(Some(rate_config)));
        let middleware = rate_limit_middleware(limiter);
        router = router.layer(axum::middleware::from_fn(middleware));
    }

    // Apply authentication middleware
    #[cfg(feature = "security")]
    if let Some(auth_config) = &config.authentication {
        use crate::security::{auth_middleware, ApiKeyAuth, AuthContext, AuthError, BearerAuth};
        use axum::http::HeaderValue;

        if let crate::config::AuthConfig::ApiKey {
            header_name,
            prefix,
        } = auth_config
        {
            let auth = Arc::new(ApiKeyAuth::new());
            let auth_clone = auth.clone();
            let header_name = header_name.clone();
            let prefix = prefix.clone();
            let extract_auth =
                move |req: &axum::http::Request<Body>| -> Result<AuthContext, AuthError> {
                    // Get header value; if missing or malformed, return auth error immediately
                    let header_value = match req
                        .headers()
                        .get(&header_name)
                        .and_then(|v: &HeaderValue| v.to_str().ok())
                    {
                        Some(value) => value,
                        None => return Err(AuthError::MissingAuth),
                    };

                    let client_ip = req
                        .headers()
                        .get("x-forwarded-for")
                        .or_else(|| req.headers().get("x-real-ip"))
                        .and_then(|v: &HeaderValue| v.to_str().ok())
                        .unwrap_or("unknown")
                        .to_string();

                    // Security fix: Validate prefix is not empty before checking
                    // This prevents authentication bypass when prefix is empty
                    if prefix.is_empty() {
                        return Err(AuthError::MissingAuth);
                    }

                    if header_value.starts_with(&prefix) {
                        let key = &header_value[prefix.len()..];
                        if let Some(permissions) = auth.validate_key(key, &client_ip) {
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
            let extract_auth =
                move |req: &axum::http::Request<Body>| -> Result<AuthContext, AuthError> {
                    // Validate authorization header is present and properly formatted
                    // Empty or malformed headers must be rejected to prevent authentication bypass
                    let header_value = match req
                        .headers()
                        .get("authorization")
                        .and_then(|v: &HeaderValue| v.to_str().ok())
                    {
                        Some(value) => value,
                        None => return Err(AuthError::MissingAuth),
                    };

                    // Validate Bearer token format and non-empty token
                    // Note: "".strip_prefix("Bearer ") returns Some("") - empty token must be rejected
                    let token = match header_value.strip_prefix("Bearer ") {
                        Some(token) if !token.is_empty() => token,
                        _ => return Err(AuthError::InvalidToken),
                    };

                    if let Some(context) = auth.validate_token(token) {
                        Ok(context)
                    } else {
                        Err(AuthError::InvalidToken)
                    }
                };
            let middleware = auth_middleware(auth_clone, extract_auth);
            router = router.layer(axum::middleware::from_fn(middleware));
        } else {
            return Err(crate::config::ConfigError::ValidationError(
                "OAuth2 not yet implemented".into(),
            ));
        }
    }

    // Initialize logging
    #[cfg(feature = "logging")]
    if let Some(logging) = &config.logging {
        let _ = crate::config::init_logging(logging);
    }

    // Apply cache middleware
    #[cfg(feature = "cache")]
    {
        #[allow(unused_imports)]
        use crate::cache::CacheMiddleware;

        // Use default cache config for now
        // In the future, this could be configurable via config file
        let cache_config = crate::cache::CacheConfig::default();
        let cache_middleware = CacheMiddleware::new(cache_config);
        router = router.layer(cache_middleware);
    }

    Ok(router)
}

/// Build HTTP router with hot reload support
///
/// This function builds a router with configuration hot reload support.
/// When the configuration file is modified, the router will automatically
/// reload and apply the new configuration.
///
/// # Arguments
/// * `config_path` - Path to the configuration file (YAML or JSON)
///
/// # Returns
/// A tuple containing:
/// * The configured Axum router
/// * A `ConfigWatcher` that manages configuration updates
/// * A `RecommendedWatcher` that watches for file changes
///
/// # Example
/// ```ignore
/// use std::path::PathBuf;
///
/// let config_path = PathBuf::from("config.yaml");
/// let (router, config_watcher, file_watcher) =
///     sdforge::http::build_with_hot_reload(&config_path).unwrap();
///
/// // Keep file_watcher alive to maintain file watching
/// tokio::spawn(async move {
///     while let Ok(event) = config_watcher.event_receiver.recv().await {
///         println!("Config event: {:?}", event);
///     }
/// });
/// ```
#[allow(dead_code)]
#[cfg(feature = "hot-reload")]
pub async fn build_with_hot_reload(
    config_path: &std::path::Path,
) -> Result<
    (
        Router,
        crate::config::hot_reload::ConfigWatcher,
        notify::RecommendedWatcher,
    ),
    Box<dyn std::error::Error>,
> {
    use crate::config::hot_reload::ConfigWatcher;
    use std::path::PathBuf;

    let config_path = PathBuf::from(config_path);
    let (config_watcher, _event_receiver) = ConfigWatcher::new(config_path.clone())?;
    let file_watcher = config_watcher.watch()?;

    // Build router with current configuration
    let config = config_watcher.get().await;
    let router = build_with_config(&config)?;

    Ok((router, config_watcher, file_watcher))
}
