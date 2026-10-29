// Copyright (c) 2026 Kirky.X🌠
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
/// `Router::new.layer(rate_limit_layer(limiter))` without importing both
/// `RateLimitLayer` and the `HttpRequestRateLimiter` trait.
///
/// Only available when the `ratelimit-http` feature is enabled.
#[cfg(feature = "ratelimit-http")]
pub fn rate_limit_layer(
    limiter: std::sync::Arc<dyn crate::security::HttpRequestRateLimiter>,
) -> RateLimitLayer {
    RateLimitLayer::new(limiter)
}

/// Generate or extract request ID from request.
///
/// A blank `x-request-id` header falls through to UUID generation, matching
/// the `context` middleware's handling of empty inbound ids.
// 非 context 态由下方 request-id 中间件消费，context 态调用点被 cfg 排除——
// allow 谓词须与调用点门控同步。
#[cfg_attr(feature = "context", allow(dead_code))]
pub(crate) fn get_or_generate_request_id(req: &axum::http::Request<Body>) -> String {
    req.headers()
        .get(X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
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
            let clean_prefix = prefix.trim_start_matches('/');
            // 防御性切片——此前 `base_path[1..]` 假定首字符
            // 必为 '/'，空串会越界 panic、多字节首字符会 panic（非 char
            // boundary）、无前导斜杠会静默丢首字符。改用 strip_prefix 语义。
            let path_without_slash = base_path.strip_prefix('/').unwrap_or(base_path);
            format!("/{}/{}", clean_prefix, path_without_slash)
        }
        _ => base_path.to_string(),
    }
}

pub(crate) fn apply_security_headers(
    router: Router,
    config: &crate::config::SecurityConfig,
) -> Router {
    crate::http::security_headers::apply_configured_headers(router, config)
}

/// True when an inventory-registered route already occupies `path`
/// (module-prefix resolved).
///
/// Built-in probe/metrics mounting skips such paths to avoid axum
/// duplicate-route panics. Paths owned by a router the caller merges in are
/// not visible here — axum exposes no way to enumerate a router's routes;
/// `router_path_taken` covers that gap.
// 消费方：health 探针挂载、metrics 挂载、health 门控测试——
// 两 feature 皆关时无消费方，不参与编译。
#[cfg(any(feature = "health", feature = "metrics"))]
pub fn route_path_taken(path: &str) -> bool {
    use crate::core::Registration;
    let mut taken = false;
    for registration in inventory::iter::<RouteRegistration>() {
        let route = registration.create();
        let full = resolve_route_path(route.path(), route.module_prefix());
        if full == path {
            taken = true;
            break;
        }
    }
    if !taken {
        for route in inventory::iter::<HttpRoute>() {
            let full = resolve_route_path(route.path(), route.module_prefix());
            if full == path {
                taken = true;
                break;
            }
        }
    }
    taken
}

/// True when `router` already serves `GET path`.
///
/// axum 的 `Router` 不提供路由枚举 API（只有 `has_routes`），无法在注册前
/// 读出已注册的路径；这里用「试注册」判定：在路由器副本上注册一条 GET
/// 路由，与既有 GET 路由重叠时 axum 会 panic（"Overlapping method route"），
/// 据此认定该路径已被占用。试注册期间临时静音 panic hook，避免 axum 内部
/// panic 的噪声落到 stderr。
#[cfg(any(feature = "health", feature = "metrics"))]
pub(crate) fn router_path_taken(router: &Router, path: &str) -> bool {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let probe = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        router.clone().route(path, axum::routing::get(|| async {}))
    }));
    std::panic::set_hook(previous);
    probe.is_err()
}

/// Mount a built-in route at `path` unless the path is already served.
///
/// Two kinds of owners are skipped, both letting the caller's route win —
/// mounting a duplicate path would make axum panic at build time
/// ("Overlapping method route"):
///
/// * inventory-registered routes ([`route_path_taken`]);
/// * the router the caller merged in ([`router_path_taken`]).
#[cfg(any(feature = "health", feature = "metrics"))]
pub(crate) fn mount_builtin_route(
    router: Router,
    path: &'static str,
    handler: axum::routing::MethodRouter,
) -> Router {
    if route_path_taken(path) {
        return router;
    }
    if router_path_taken(&router, path) {
        log::warn!(
            "{}",
            crate::i18n::t("http-builtin-route-skipped", &[("path", path.to_string())])
        );
        return router;
    }
    router.route(path, handler)
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
/// consider using `build_with_config_ext` for middleware support.
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
    //
    // CRITICAL: dedup must group by path but MERGE MethodRouters, never
    // discard entries. Keying by path alone silently dropped distinct
    // methods on the same path (e.g. GET list + POST create on
    // `/api/x/resources`), turning the dropped method into 405 responses.
    // `MethodRouter::merge` keeps per-method "later wins" semantics while
    // preserving coexisting methods on one path.
    let mut seen: std::collections::HashMap<String, MethodRouter> =
        std::collections::HashMap::with_capacity(routes.len());
    for route in routes {
        let prefix = route.module_prefix.as_deref();
        let full_path = resolve_route_path(&route.path, prefix);
        match seen.get_mut(&full_path) {
            Some(existing) => {
                *existing = std::mem::take(existing).merge(route.handler);
            }
            None => {
                seen.insert(full_path, route.handler);
            }
        }
    }

    // Sort by full path for deterministic router construction order.
    let mut deduped: Vec<_> = seen.into_iter().collect();
    deduped.sort_by(|a, b| a.0.cmp(&b.0));

    for (full_path, handler) in deduped {
        router = router.route(&full_path, handler);
    }

    // Unmatched routes answer with the unified error envelope instead of an
    // empty body, so `Accept-Language` can select the message language.
    router = router.fallback(route_not_found_fallback);

    router
}

/// Resolve the request locale from `Accept-Language` (bundled en/zh only).
///
/// Without the `i18n` feature the header parser is not compiled; callers then
/// fall back to the process locale.
#[cfg(feature = "i18n")]
fn locale_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    crate::i18n::locale_from_headers(headers)
}

#[cfg(not(feature = "i18n"))]
fn locale_from_headers(_headers: &axum::http::HeaderMap) -> Option<String> {
    None
}

/// Translate a framework key for the request's locale, falling back to the
/// process locale when the header carries no usable entry.
fn localize_for_headers(
    headers: &axum::http::HeaderMap,
    key: &str,
    args: &[(&str, String)],
) -> String {
    match locale_from_headers(headers) {
        Some(locale) => crate::i18n::translate_for(&locale, key, args),
        None => crate::i18n::t(key, args),
    }
}

/// Fallback for unmatched routes: the unified error envelope
/// (`{"code":"NOT_FOUND","message":…}`) with the message localized from the
/// request's `Accept-Language` (process locale without the `i18n` feature).
async fn route_not_found_fallback(req: axum::extract::Request) -> axum::response::Response {
    let message = localize_for_headers(req.headers(), "http-route-not-found", &[]);
    let err = crate::error::unified::UnifiedError::new("NOT_FOUND", message);
    crate::error::unified::render_http(axum::http::StatusCode::NOT_FOUND, &err)
}

/// Scope the request's `Accept-Language` locale over the whole pipeline so
/// framework strings produced by `i18n::t` (401/403/429 bodies, handler-level
/// messages) follow the request language instead of the process locale.
#[cfg(feature = "i18n")]
async fn request_locale_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    match locale_from_headers(req.headers()) {
        Some(locale) => crate::i18n::scope_request_locale(locale, next.run(req)).await,
        None => next.run(req).await,
    }
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
///
/// # Security Warning
///
/// 与 [`build`] 一样，此便捷函数**不挂载任何安全/配置中间件**（无 CORS、
/// security headers、body limit、timeout、auth）。仅版本重定向中间件会
/// 被叠加。生产用途请使用 [`build_with_config_ext`]（先并路由再套中间件栈）。
pub fn build_with_redirect() -> Router {
    let router = build();
    router.layer(axum::middleware::from_fn(version_redirect_middleware))
}

/// Build HTTP router with configuration, merging application routes first.
///
/// The only configuration-driven HTTP entry point: the caller's router is
/// merged **before** the middleware stack is applied. axum's `Router::layer`
/// only wraps routes registered before it, so a router that already carries
/// the stack cannot accept later merges without bypassing authentication,
/// rate limiting and security headers — no such entry point is exposed.
///
/// # Arguments
/// * `base` - application routes to merge into the framework router
/// * `config` - The application configuration
///
/// # Returns
/// A configured Axum router with all middleware applied
///
/// # Panics
/// Panics if `base` defines its own fallback (`Router::fallback`): axum
/// refuses to merge two routers that both carry a fallback, and the framework
/// router installs the unified not-found fallback.
///
/// A `base` that claims a built-in probe path (`GET /healthz` or `GET /readyz`
/// with the `health` feature, `GET /metrics` with the `metrics` feature) does
/// not panic: the caller's route takes precedence and the built-in probe is
/// skipped with a warning.
pub fn build_with_config_ext(
    base: Router,
    config: &crate::config::SdForgeConfig,
) -> Result<Router, ConfigError> {
    build_with_config_inner(base, config, None)
}

/// Build HTTP router with configuration, merging application routes first
/// **and** installing the configured rate limiter.
///
/// Combines [`build_with_config_ext`] (application routes merged before the
/// middleware stack is applied, so they receive authentication, rate limiting
/// and security headers) with the asynchronous `security.rate_limit` limiter
/// build and install. This is the entry point for applications that have
/// their own routes *and* rely on the configured rate limit.
///
/// # Arguments
/// * `base` - application routes to merge into the framework router
/// * `config` - The application configuration
///
/// # Returns
/// A configured Axum router with all middleware applied, including the
/// rate limiter when `security.rate_limit` is set.
///
/// # Errors
/// Returns a [`ConfigError::ValidationError`] when the limiteron
/// configuration fails to build.
///
/// # Panics
/// Panics under the same conditions as [`build_with_config_ext`] (own
/// fallback).
#[cfg(feature = "ratelimit-http")]
pub async fn build_with_config_ext_rate_limited(
    base: Router,
    config: &crate::config::SdForgeConfig,
) -> Result<Router, ConfigError> {
    let limiter = build_rate_limiter(config).await?;
    build_with_config_inner(base, config, limiter)
}

/// Build the configured HTTP rate limiter, if any.
///
/// The limiter is constructed asynchronously (`limiteron::Governor`), so it
/// cannot be built inside the synchronous router builders.
#[cfg(feature = "ratelimit-http")]
async fn build_rate_limiter(
    config: &crate::config::SdForgeConfig,
) -> Result<Option<std::sync::Arc<dyn crate::security::HttpRequestRateLimiter>>, ConfigError> {
    use std::sync::Arc;

    let limiter = config
        .build_rate_limiter()
        .await
        .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
    Ok(
        limiter
            .map(|adapter| Arc::new(adapter) as Arc<dyn crate::security::HttpRequestRateLimiter>),
    )
}

/// Rate-limiter argument of [`build_with_config_inner`].
///
/// Without the `ratelimit-http` feature there is no HTTP limiter to install,
/// so the argument degenerates to an empty optional.
#[cfg(feature = "ratelimit-http")]
type RateLimiterArg = Option<std::sync::Arc<dyn crate::security::HttpRequestRateLimiter>>;
#[cfg(not(feature = "ratelimit-http"))]
type RateLimiterArg = Option<()>;

/// Shared body of the `build_with_config*` entry points.
///
/// `base` is merged before any layer is applied so that application routes
/// receive the same middleware stack as the framework routes.
fn build_with_config_inner(
    base: Router,
    config: &crate::config::SdForgeConfig,
    #[allow(unused_variables)] rate_limiter: RateLimiterArg,
) -> Result<Router, ConfigError> {
    #[cfg(feature = "security")]
    use std::sync::Arc;

    // Fail fast on nonsensical server limits (e.g. max_body_size = 0 rejects
    // every request with a body; request_timeout_secs = 0 times out every
    // request) instead of wiring middleware that bricks the app.
    config.server.validate()?;

    // Fail closed on auth configs the build cannot honor: without the
    // `security` feature no auth middleware exists, so an AuthConfig
    // requesting ApiKey/Jwt would silently produce an unauthenticated router.
    #[cfg(not(feature = "security"))]
    config.authentication.require_security_feature()?;

    // 应用路由在叠层之前并入：axum 的 `Router::layer` 只作用于调用它之前
    // 已注册的路由，之后 merge 进来的路由将绕过整条中间件栈（含鉴权）。
    let mut router = build().merge(base);

    // request metrics middleware (count / latency / status per route
    // template). Installed early so every route from build is measured;
    // the /metrics endpoint itself is mounted after the auth layer below and
    // therefore not self-recorded.
    #[cfg(feature = "metrics")]
    {
        router = router.layer(axum::middleware::from_fn(crate::metrics::record_middleware));
    }

    // request-span middleware (OTLP export via sdforge::otel).
    #[cfg(feature = "otel")]
    {
        router = router.layer(axum::middleware::from_fn(crate::otel::otel_span_middleware));
    }

    // ETag/If-None-Match conditional requests for GET responses.
    #[cfg(feature = "etag")]
    {
        router = router.layer(axum::middleware::from_fn(
            crate::http::etag::etag_middleware,
        ));
    }

    // processor pre/post hook pipeline (active when hooks installed).
    #[cfg(feature = "hooks")]
    {
        router = router.layer(axum::middleware::from_fn(crate::hooks::hooks_middleware));
    }

    // Request ID middleware (first to ensure all requests have an ID).
    // with the `context` feature the richer context middleware
    // (request_id + trace_id + task-local scope + response echo) subsumes it.
    #[cfg(not(feature = "context"))]
    {
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
    }
    #[cfg(feature = "context")]
    {
        router = router.layer(axum::middleware::from_fn(
            crate::context::context_middleware,
        ));
    }

    // Apply global body limit（来自 ServerConfig::max_body_size，
    // 此前硬编码 10MB 且配置结构无对应字段，运维无法调整）
    router = router.layer(tower_http::limit::RequestBodyLimitLayer::new(
        config.server.max_body_size,
    ));

    // Idempotency replay protection（feature = idempotency，enabled 默认 false；
    // 仅携带 Idempotency-Key 的 POST/PUT/PATCH 参与，其余零开销透行）
    #[cfg(feature = "idempotency")]
    if config.server.idempotency.enabled {
        use std::sync::Arc as _idempotency_arc;
        let store = config
            .server
            .idempotency
            .store
            .clone()
            .unwrap_or_else(|| _idempotency_arc::new(crate::cache::IdempotencyStore::new()));
        let ttl = config.server.idempotency.ttl_secs;
        let inflight_ttl = config.server.idempotency.inflight_ttl_secs;
        let max_bytes = config.server.idempotency.max_response_bytes;
        router = router.layer(axum::middleware::from_fn(move |req, next| {
            crate::http::idempotency::idempotency_middleware(
                _idempotency_arc::clone(&store),
                ttl,
                inflight_ttl,
                max_bytes,
                req,
                next,
            )
        }));
    }

    router = router.layer(tower_http::compression::CompressionLayer::new());

    // Use configurable request timeout from server config
    let timeout_secs = config.server.request_timeout_secs;
    router = router.layer(tower_http::timeout::TimeoutLayer::with_status_code(
        axum::http::StatusCode::REQUEST_TIMEOUT,
        std::time::Duration::from_secs(timeout_secs),
    ));

    if let Some(cors) = &config.server.cors {
        let cors_layer = crate::config::build_cors_layer(cors)?;
        router = router.layer(cors_layer);
    }

    // 安全响应头：配置面在 `security` feature 下才挂在 SdForgeConfig 上；
    // feature 关闭时用严格默认值（与开启时的默认配置一致）。
    #[cfg(feature = "security")]
    {
        router = apply_security_headers(router, &config.security);
    }
    #[cfg(not(feature = "security"))]
    {
        router = apply_security_headers(router, &crate::config::SecurityConfig::default());
    }

    #[cfg(feature = "security")]
    {
        use crate::config::AuthConfig;
        use crate::security::{
            AuthContext, AuthError, BearerAuth, SdForgeApiKeyAuth, auth_middleware,
        };
        use axum::http::HeaderValue;

        let auth_config = &config.authentication;

        if let AuthConfig::ApiKey {
            header_name,
            prefix,
            keys,
        } = auth_config
        {
            // 播种配置声明的 key；为空时显式构建错误，
            // 不再静默创建空 key store 把整条 API 锁死在 401。
            if keys.is_empty() {
                return Err(ConfigError::ValidationError(
                    "ApiKey auth is configured but no keys are provided: add `keys` entries \
                     (AuthConfig::ApiKey { keys: vec![ApiKeySeed { key, permissions }], .. }) \
                     so the API is not silently locked out"
                        .to_string(),
                ));
            }
            let auth = Arc::new(SdForgeApiKeyAuth::new());
            for seed in keys {
                auth.add_key(seed.key.clone(), seed.permissions.clone());
            }
            let auth_clone = auth.clone();
            let header_name = header_name.clone();
            let prefix = prefix.clone();
            let extract_auth =
                move |req: &axum::http::Request<Body>| -> Result<AuthContext, AuthError> {
                    let header_value = match req
                        .headers()
                        .get(&header_name)
                        .and_then(|v: &HeaderValue| v.to_str().ok())
                    {
                        Some(value) => value,
                        None => return Err(AuthError::MissingAuth),
                    };

                    // Use trusted-proxy-aware IP extraction:
                    // direct header reads allow IP spoofing; extract_client_ip
                    // only trusts X-Forwarded-For / X-Real-IP from trusted proxies.
                    let client_ip = crate::security::extract_client_ip(req)
                        .unwrap_or_else(|| "unknown".to_string());

                    // Validate prefix is not empty before checking
                    // This prevents authentication bypass when prefix is empty
                    if prefix.is_empty() {
                        return Err(AuthError::MissingAuth);
                    }

                    if header_value.starts_with(&prefix) {
                        let key = &header_value[prefix.len()..];
                        if let Some(permissions) = auth.validate_key(key, &client_ip) {
                            Ok(AuthContext {
                                user_id: Some(SdForgeApiKeyAuth::key_id(key)),
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
        } else if let AuthConfig::Jwt {
            secret,
            audience,
            issuer,
        } = auth_config
        {
            // 使用 try_new 而非 new：AuthConfig::validate() 接受的 secret（仅长度+弱词校验）
            // 可能不满足 BearerAuth 的强复杂度要求，`new` 会在启动时 panic；
            // 改为返回构建错误，由调用方处理而非崩溃。
            let mut auth = BearerAuth::try_new(secret)
                .map_err(|e| ConfigError::ValidationError(e.to_string()))?;
            // 声明的 aud/iss 透传给校验器：不配置即不校验，配置后令牌缺该
            // 声明或取值不符一律拒绝（防令牌替换）。
            if let Some(audience) = audience {
                auth = auth.with_audience_inner(audience.clone());
            }
            if let Some(issuer) = issuer {
                auth = auth.with_issuer_inner(issuer.clone());
            }
            let auth = Arc::new(auth);
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

    // 限流层：装在鉴权层之外（更早执行），故未认证的洪泛也会被限流；
    // 探针与 /metrics 在其后挂载，不受限流影响。
    #[cfg(feature = "ratelimit-http")]
    if let Some(limiter) = rate_limiter {
        router = router.layer(rate_limit_layer(limiter));
    }

    // 请求级语言作用域：在认证层之外再包一层，使认证拒绝（401）、路由处理器
    // 内 RBAC 拒绝等所有 i18n::t 文案随 Accept-Language 变化（进程 locale
    // 仅作回退）。
    #[cfg(feature = "i18n")]
    {
        router = router.layer(axum::middleware::from_fn(request_locale_middleware));
    }

    // mount /healthz + /readyz AFTER the auth layer — axum layers only
    // apply to routes registered before them, so probes added here bypass
    // authentication/rate-limiting by construction. A probe path already
    // claimed — by an inventory-registered route or by the merged-in `base`
    // router (mount_builtin_route checks both: route_path_taken plus a
    // trial-insert router_path_taken) — is skipped to avoid duplicate-route
    // panics.
    #[cfg(feature = "health")]
    {
        router = crate::health::mount_probes(router);
    }

    // mount /metrics after the auth layer (bypasses authentication).
    #[cfg(feature = "metrics")]
    {
        router = crate::metrics::mount_metrics(router);
    }

    // 日志初始化由使用方经 sdforge::inklog（`init_inklog_logger`）完成，本函数不介入
    Ok(router)
}
