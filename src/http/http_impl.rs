// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT

use super::*;
use crate::config::ConfigError;
use crate::core::Registration;
use axum::Router;
use axum::body::Body;
use uuid::Uuid;

/// Construct a `RateLimitLayer` from any `HttpRequestRateLimiter`.
///
/// Convenience helper: wraps `RateLimitLayer::new` so users can write
/// `Router::new().layer(rate_limit_layer(limiter))` without importing both
/// `RateLimitLayer` and the `HttpRequestRateLimiter` trait.
///
/// Only available when the `ratelimit-http` feature is enabled.
#[cfg(feature = "ratelimit-http")]
pub fn rate_limit_layer(
    limiter: std::sync::Arc<dyn crate::security::HttpRequestRateLimiter>,
) -> RateLimitLayer {
    RateLimitLayer::new(limiter)
}

/// Generate or extract request ID from request
pub(crate) fn get_or_generate_request_id(req: &axum::http::Request<Body>) -> String {
    req.headers()
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

impl HttpRoute {
    #[allow(missing_docs)]
    pub fn new(
        path: String,
        handler: MethodRouter,
        metadata: ApiMetadata,
        module_prefix: Option<String>,
    ) -> Self {
        Self {
            path,
            handler,
            metadata,
            module_prefix,
        }
    }

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

/// Resolve module prefix for a route path
///
/// This function checks if there's a module prefix available for the given route.
/// In practice, the macro generates inline path resolution, but this provides
/// a runtime fallback for dynamic path construction.
pub(crate) fn resolve_route_path(base_path: &str, module_prefix: Option<&str>) -> String {
    match module_prefix {
        Some(prefix) if !prefix.is_empty() => {
            // Remove leading slash from prefix if present
            let clean_prefix = prefix.trim_start_matches('/');
            format!("/{}/{}", clean_prefix, &base_path[1..])
        }
        _ => base_path.to_string(),
    }
}

pub(crate) fn apply_security_headers(router: Router) -> Router {
    SecurityHeaders::default().apply(router)
}

/// Prevent linker from optimizing away inventory registrations
/// Uses reference iteration to ensure symbols are preserved
#[cfg(feature = "mcp")]
#[inline(never)]
pub(crate) fn preserve_mcp_inventory() {
    // Iterate and count - this forces linker to keep symbols
    let _count = inventory::iter::<crate::mcp::McpToolRegistration>().count();
    let _ = _count; // Suppress unused variable warning
}

/// Prevent linker from optimizing away WebSocket inventory registrations
#[cfg(feature = "websocket")]
#[inline(never)]
pub(crate) fn preserve_websocket_inventory() {
    let _count = inventory::iter::<crate::websocket::WebSocketRoute>().count();
    let _ = _count;
}

/// Prevent linker from optimizing away gRPC inventory registrations
#[cfg(feature = "grpc")]
#[inline(never)]
pub(crate) fn preserve_grpc_inventory() {
    let _count = inventory::iter::<crate::grpc::GrpcRouteRegistration>().count();
    let _ = _count;
}

/// Build HTTP router from registered routes
///
/// This function collects all routes registered via `inventory::submit!`
/// and builds a complete Axum router for serving HTTP requests.
/// Routes are automatically prefixed with their module prefix if available.
///
/// # Returns
/// A basic Axum router with all registered routes
///
/// # Note
/// This is the simplest router building function. For production use,
/// consider using `build_with_config()` for middleware support.
pub fn build() -> Router {
    // Force inventory collection to prevent linker optimization
    // Using inline(never) functions to ensure symbols are preserved
    #[cfg(feature = "mcp")]
    preserve_mcp_inventory();

    #[cfg(feature = "websocket")]
    preserve_websocket_inventory();

    #[cfg(feature = "grpc")]
    preserve_grpc_inventory();

    let mut router = Router::new();

    // First, collect registrations with function pointers
    let registrations: Vec<_> = inventory::iter::<RouteRegistration>().collect();

    // Map registrations to HttpRoute instances via their create functions
    let mut routes: Vec<_> = registrations
        .iter()
        .map(|registration| registration.create())
        .collect();

    // Also collect direct HttpRoute registrations
    for route in inventory::iter::<HttpRoute>() {
        routes.push(route.clone());
    }

    // Compute the full path (with module prefix) for each route, then
    // deduplicate by full path. Without this, registering the same path
    // via both RouteRegistration and a direct HttpRoute (or twice within
    // either source) would cause Axum to panic with "Cannot register
    // duplicate route" at runtime.
    //
    // Resolution order: later registrations win. Direct HttpRoute entries
    // are appended after RouteRegistration entries, so they take precedence
    // — which matches the expectation that explicit registrations override
    // macro-generated ones.
    let mut seen: std::collections::HashMap<String, HttpRoute> =
        std::collections::HashMap::with_capacity(routes.len());
    for route in routes {
        let prefix = route.module_prefix.as_deref();
        let full_path = resolve_route_path(&route.path, prefix);
        seen.insert(full_path, route);
    }

    // Sort by full path for deterministic router construction order.
    let mut deduped: Vec<_> = seen.into_iter().collect();
    deduped.sort_by(|a, b| a.0.cmp(&b.0));

    for (full_path, route) in deduped {
        router = router.route(&full_path, route.handler);
    }

    router
}

/// Build HTTP router with version redirect middleware
///
/// This function builds a router with automatic version redirect support.
/// Requests to `/api/{path}` without a version are redirected to `/api/v1/{path}`.
///
/// # Returns
/// An Axum router with version redirect middleware applied
///
/// # Note
/// Use this when you want automatic version fallback for unversioned API requests.
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
///
/// # Note
/// This is the recommended function for production use. It applies security headers,
/// CORS, rate limiting, compression, and timeout middleware based on the config.
pub fn build_with_config(config: &crate::config::AppConfig) -> Result<Router, ConfigError> {
    #[cfg(feature = "security")]
    use std::sync::Arc;

    const DEFAULT_BODY_LIMIT: usize = 10 * 1024 * 1024;

    let mut router = build();

    // Apply request ID middleware (first to ensure all requests have an ID)
    router = router.layer(axum::middleware::from_fn(
        |mut req: axum::http::Request<Body>, next: axum::middleware::Next| async move {
            let request_id = get_or_generate_request_id(&req);
            // Safely insert request ID header — fall back to a static placeholder
            // if the value contains non-ASCII characters (prevents panic on malformed client input)
            let header_value = axum::http::HeaderValue::from_str(&request_id)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("invalid-request-id"));
            req.headers_mut().insert(
                axum::http::header::HeaderName::from_static(X_REQUEST_ID),
                header_value.clone(),
            );
            let mut response = next.run(req).await;
            response.headers_mut().insert(
                axum::http::header::HeaderName::from_static(X_REQUEST_ID),
                header_value,
            );
            response
        },
    ));

    // Apply global body limit
    router = router.layer(tower_http::limit::RequestBodyLimitLayer::new(
        DEFAULT_BODY_LIMIT,
    ));

    // Apply response compression
    router = router.layer(tower_http::compression::CompressionLayer::new());

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
    router = apply_security_headers(router);

    // Apply authentication middleware
    #[cfg(feature = "security")]
    {
        use crate::config::AuthConfig;
        use crate::security::{AppApiKeyAuth, AuthContext, AuthError, BearerAuth, auth_middleware};
        use axum::http::HeaderValue;

        let auth_config = &config.authentication;

        if let AuthConfig::ApiKey {
            header_name,
            prefix,
        } = auth_config
        {
            let auth = Arc::new(AppApiKeyAuth::new());
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

                    // Use trusted-proxy-aware IP extraction (vuln-0001 fix):
                    // direct header reads allow IP spoofing; extract_client_ip_core
                    // only trusts X-Forwarded-For / X-Real-IP from trusted proxies.
                    let client_ip = crate::security::extract_client_ip_core(req)
                        .unwrap_or_else(|| "unknown".to_string());

                    // Security fix: Validate prefix is not empty before checking
                    // This prevents authentication bypass when prefix is empty
                    if prefix.is_empty() {
                        return Err(AuthError::MissingAuth);
                    }

                    if header_value.starts_with(&prefix) {
                        let key = &header_value[prefix.len()..];
                        if let Some(permissions) = auth.validate_key(key, &client_ip) {
                            Ok(AuthContext {
                                user_id: Some(AppApiKeyAuth::key_id(key)),
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
        } else if let AuthConfig::Jwt { secret, .. } = auth_config {
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
        }
        // OAuth2 is already checked before this block
        // None is handled by doing nothing
    }

    // Note: 日志初始化已移除，由使用方通过 sdforge::inklog 直接管理
    Ok(router)
}
