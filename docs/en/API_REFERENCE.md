# 📘 Sdforge API Reference

This document summarizes SDForge's public API based on the public interfaces in `src/` and README usage. SDForge's API surface consists of two parts: core types and macros that are always available, and protocol/capability modules gated by Cargo features — the API of an unenabled feature does not participate in compilation, which is exactly where "compile-time protocol selection" lands. For complete type signatures and documentation, [docs.rs/sdforge](https://docs.rs/sdforge) is authoritative.

## 📋 Table of Contents

<details open>
<summary>📑 Table of Contents</summary>

- [Overview](#-overview)
- [Core API](#-core-api)
- [Configuration/Extension API](#-configurationextension-api)
- [Error Types](#-error-types)
- [Feature-Gated API](#-feature-gated-api)
- [Usage Examples](#-usage-examples)
- [Best Practices](#-best-practices)
- [Feature Flags](#-feature-flags)

</details>

## 🧭 Overview

- **crates**: `sdforge` (runtime library) + `sdforge-macros` (proc macros, re-exported via the root crate)
- **Unified entry point**: most use cases start with `use sdforge::prelude::*;`, which provides the core types, macros, and common feature-gated re-exports
- **Dependency forwarding**: the framework forwards the dependencies needed by macro-generated code as re-exports (`sdforge::serde`, `sdforge::serde_json`, `sdforge::inventory`, `sdforge::axum`, `sdforge::rmcp`, `sdforge::tonic`, `sdforge::prost`, `sdforge::utoipa`, `sdforge::clap`, `sdforge::anyhow`, `sdforge::tokio_stream`, `sdforge::oxcache`, `sdforge::tower`, `sdforge::tower_http`), so downstream crates do not need to depend on these framework libraries directly to use `#[forge]`

### Macros (no feature gating)

| Macro | Description |
|----|------|
| `#[forge(...)]` | Unified endpoint annotation; generates HTTP / MCP / gRPC / WebSocket / CLI / OpenAPI registration code per enabled features |
| `#[service_module(prefix = "...")]` | Adds a unified path prefix to all endpoints in the module |
| `impl_default_new!(Type)` | Generates `new()` for a unit struct and implements `Default` |
| `test_macro!` | Test helper: wraps a function as a test entry point (exported via prelude) |

### `#[forge]` parameters

| Parameter | Description | Required | Default |
|------|------|------|--------|
| `name` | Endpoint name (cannot be a Rust reserved word) | Yes | - |
| `version` | API version | Yes | - |
| `path` | HTTP path (e.g. `/users/:id`) | No | - |
| `prefix` | Module path prefix: injected into the module's endpoints by `#[service_module(prefix = "...")]`; an explicit declaration overrides the module-level injection | No | - |
| `method` | HTTP method (GET/POST/PUT/DELETE, etc.) | No | GET |
| `status` | Success status code (100..=999) | No | 200 |
| `description` | Endpoint description | No | - |
| `i18n_key` | Runtime translation key for `description` | No | - |
| `deprecated` | Endpoint-level deprecation marker (bare flag or `= true/false`): HTTP success responses carry a `Deprecation: true` header, gRPC success responses inject `deprecation` metadata | No | false |
| `sunset` | Endpoint-level retirement date/value (HTTP success responses carry a `Sunset` header, gRPC success responses inject `sunset` metadata) | No | - |
| `successor` | Successor endpoint hint (HTTP success responses carry `Link: <...>; rel="successor-version"`, gRPC success responses inject `successor-version` metadata) | No | - |
| `tool_name` | MCP tool name (`mcp`) | No | - |
| `grpc_method` | gRPC method name (`grpc`) | No | - |
| `cli` | Register as a CLI command (`cli`) | No | false |
| `cache_ttl` | Result cache TTL in seconds (`cache`) | No | - |
| `ws_path` | WebSocket mount path (`websocket`): when declared, `websocket::build()` mounts at that path (missing leading slash normalized automatically); otherwise derived as `/{name}` | No | - |
| `stream` / `streaming` | SSE streaming response switch | No | false |
| `no_prefix` | Skip module/version prefix concatenation | No | false |
| `validate` | Bare flag: makes the validation rules of `#[param(...)]` effective (`validate`); source declarations and `flatten` are not bound by this flag; see the attribute table in the next section | No | false |
| `paginate` | Bare flag: declarative pagination (`paginate`) | No | false |
| `on_start` / `on_stop` | Bare flags: process lifecycle hooks (`lifecycle`) | No | false |
| `auth(role = "...")` | Endpoint-level RBAC role (effective on `http` / `mcp` / `grpc`: no matching role → HTTP 403, gRPC `permission_denied`, MCP JSON-RPC `-32003`; not effective on CLI/WS) | No | - |

### `#[param]` parameter attributes

Each parameter of the handler signature declares its extraction source and validation rules via `#[param(...)]`; the rules take effect only when the endpoint carries the `#[forge(validate)]` flag.

| Key | Form | Description |
|------|------|------|
| `kind` | `kind = "..."` | Extraction source: `path` / `query` / `header` / `form` / `body` / `state` / `extension` (the `#[state]` attribute is equivalent to `kind = "state"`) |
| `ge` / `le` | `ge = 1, le = 100` | Numeric lower / upper bound |
| `min_length` / `max_length` | `min_length = 2, max_length = 10` | String length lower / upper bound |
| `not_blank` | Bare marker | Non-blank validation |
| `email` | Bare marker | Email format validation |
| `flatten` | Bare marker, or `flatten = true` / `flatten = false` | **Only valid for `kind = "query"`**: the struct query parameter leaves the envelope and is extracted flatly through a dedicated `Query<ParamTy>` slot; the handler reads fields directly (`filters.budget`, no `.0`) |

#### Struct query parameters (`flatten`)

`serde_urlencoded` only supports flat `key=value`. A struct-typed query parameter is extracted through the envelope `Query<__ForgeQueryParams>` by default, and a nested struct always fails to deserialize and returns 400 (this fail-closed behavior is pinned by integration tests); with explicit `flatten`, the parameter goes through a dedicated extraction slot:

```rust
#[derive(Debug, serde::Deserialize)]
struct SearchFilters {
    budget: u64,
    keyword: String,
}

#[forge(name = "search", version = "v1", path = "/search", method = "GET")]
async fn search(
    #[param(kind = "query", ge = 1)] page: u64,
    #[param(kind = "query", flatten)] filters: SearchFilters,
) -> serde_json::Value {
    // GET /api/v1/search?page=1&budget=50&keyword=hello
    serde_json::json!({ "page": page, "budget": filters.budget, "keyword": filters.keyword })
}
```

Rules (fail-loud at macro expansion, no silent degradation):

- **Mutually exclusive with validation rules** — field validation belongs to the flattened struct's own fields; `#[param(kind = "query", flatten, ge = 1)]` is a compile error;
- **Only valid for `kind = "query"`** — other kinds (including undeclared kind) with `flatten` are a compile error;
- **Boolean-literal restriction** — only `flatten`, `flatten = true`, `flatten = false` are accepted; non-boolean values are a compile error;
- **Multiple can coexist** — multiple `flatten` parameters in the same signature each get an independent extraction slot and do not conflict with envelope-extracted scalar query parameters.

## 🧱 Core API

### The `prelude` module

`sdforge::prelude` exports:

- Core types: `ApiError`, `ApiMetadata`, `ServiceError`, `ServiceResponse`
- Macros: `forge`, `service_module`, `test_macro`
- Under the `http` feature: `validate_email`, `validate_length`, `HttpRoute`, `RouteRegistration`, `IntoResponse`
- Under the `mcp` feature: `McpToolInstance`
- The `mcp` feature re-exports `anyhow`; the `openapi` feature re-exports `utoipa` (for macro-generated code to resolve)

### Key functions (with multi-protocol features enabled)

| Function | Description |
|------|------|
| `init_all_plugins() -> PluginCounts` | Must be called **at least once**: consolidates the inventory registration items to prevent linker stripping, returns per-protocol registration counts |
| `get_websocket_routes()` (`websocket`) | Collects all registered WebSocket routes |
| `get_mcp_tools()` (`mcp`) | Collects all registered MCP tools |

`PluginCounts`' six fields always exist (always `0` when the corresponding protocol is not enabled); the struct itself is gated on any of `http` / `mcp` / `websocket` / `grpc` / `cli` being enabled: `routes` (HTTP), `mcp_tools`, `ws_routes`, `grpc_routes`, `grpc_handlers`, `cli_commands`.

### Other always-available APIs

- `sdforge::error`: `SdForgeError` (unified error enum), the `SdForgeResult<T>` alias, `ErrorContext`
- `sdforge::i18n`: `register_translation(locale, key, value)`, `set_locale`, `get_locale`, `translate_or_fallback(default, i18n_key)`, `clear_translations`; the translation registry needs no feature and can perform runtime multilingual replacement of the macro attribute `description`
- `sdforge::serde` / `sdforge::serde_json`: unconditional re-exports (for type references only; derive macros still require downstream direct dependencies on serde)

## ⚙️ Configuration/Extension API

The configuration module `sdforge::config` (`http` feature):

| Type/Function | Description |
|-----------|------|
| `SdForgeConfig` / `SdForgeConfigBuilder` | Application configuration aggregate root and Builder (with `security` / `cache` fields and `build_rate_limiter()` rate-limiter construction — see the assembly point `http::build_with_config_ext_rate_limited`) |
| `ServerConfig` / `TlsConfig` / `TimeoutConfig` | Listening (default `127.0.0.1:8080`, 30s timeout), TLS (`cert_path`/`key_path`/`alpn_protocols`, ALPN default `["h2", "http/1.1"]`), timeouts |
| `ApiConfig` / `TracingConfig` / `EnvHelper` | API behavior (route prefix, default version), tracing configuration, run-environment name helper (`environment` field) |
| `AuthConfig` / `ApiKeySeed` | Authentication configuration and API Key seeding (`AuthConfig::Jwt { secret, audience, issuer }`: after `audience` / `issuer` are configured, tokens missing those claims or with mismatched values are rejected) |
| `CacheConfig` | Cache configuration (`enabled`, `default_ttl_secs`, `max_items`, `track_stats`) |
| `SecurityConfig` / `defaults` | Security response header configuration (CSP, X-Frame-Options, etc.; with `enable_headers=false` no security headers are sent) and centralized defaults (`MIN_SECRET_LENGTH=32`, etc.) |
| `CorsConfig` / `build_cors_layer` | CORS configuration and layer construction (validates illegal origins) |
| `ConfigError` | Configuration error type (types implementing `ValidateConfig` also have `validate()`) |

HTTP build entry points `sdforge::http` (`http` feature):

| Function/Type | Description |
|-----------|------|
| `build() -> Router` | Builds an Axum Router from inventory registration items (works out of the box) |
| `build_with_config_ext(Router, &SdForgeConfig) -> Result<Router, ConfigError>` | The only configurable build entry point: merges the caller's routes first, then layers the middleware — axum's `Router::layer` only affects routes registered before the call, and routes merged in after the stack would bypass auth, hence no "stack-first" entry point exists (assembles middleware and health/metrics probes automatically; **does not assemble a rate-limiting layer**). Probe path ownership: same-named paths are taken over by the caller — routes registered via inventory and same-named paths declared in the caller's `base` both take priority, the built-in mounting skips and warns (occupancy is determined by "trial registration"; axum has no route enumeration API), and no build-time panic occurs |
| `build_with_config_ext_rate_limited(Router, &SdForgeConfig) -> Result<Router, ConfigError>` (`ratelimit-http`) | Async entry point: merges the caller's routes (protected by auth/rate-limit/security headers) and installs the rate-limiting layer (where `security.rate_limit` actually takes effect; rate limiting sits outside auth and outside probes and `/metrics`) — previously "merging routes" and "installing rate limiting" were mutually exclusive; probe path ownership is the same as `build_with_config_ext` |
| `build_with_redirect() -> Router` | Build with redirect behavior |
| `build_json_response` / `build_fallback_response` | JSON response and fallback response construction |
| `HttpRoute` / `RouteRegistration` | Route description and inventory registration item |
| `VersionRouterConfig` / `VersionedRoute` / `build_version_router` | Version routing |
| `SecurityHeaders` | Security response header configuration |
| `rate_limit_layer` (`ratelimit-http`) | HTTP rate-limiting layer |

TLS termination `sdforge::http::tls` (`serve-tls` feature, rustls aws-lc-rs; implemented independently of the gRPC-side `grpc-tls` `ServerTlsConfig` wiring):

| Function/Type | Description |
|-----------|------|
| `load_server_config(&TlsConfig) -> Result<rustls::ServerConfig, TlsError>` | Loads PEM cert/key (validates key matching at assembly time) and applies ALPN |
| `tls_acceptor(&TlsConfig) -> Result<TlsAcceptor, TlsError>` | Convenience builder for a static-certificate acceptor |
| `ReloadingTls::new(&TlsConfig) -> Result<ReloadingTls, TlsError>` | Hot-reloadable TLS server (acceptor and reloader bound at compile time); `acceptor()` takes the serve handle, `reload()` swaps in atomically (blocking; use `reload_async()` on tokio workers), keeping the old certificate on failure |
| `serve_with_graceful_shutdown_tls(router, listener, acceptor, shutdown, TlsServeConfig)` | TLS-terminating serve: injects `ConnectInfo<SocketAddr>` per request (rate limiting/auditing get an unspoofable peer IP), reusing graceful drain ordering; accept errors back off and retry per axum semantics without terminating |
| `TlsServeConfig` | Serve configuration: `handshake_timeout` (default 10s) / `header_read_timeout` (default 30s, h1 only) / `http2_keepalive_interval` (default 30s) / `http2_keepalive_timeout` (default 20s) pre-auth guardrails + `graceful` drain parameters |
| `TlsError` | Assembly errors (file read / PEM parse / rustls config / reload task — four failure surfaces) |

Extension approach: custom components follow the three construction modes `new()` / `builder()` / `with_dependencies()`; protocol extensions plug into the unified registration system via the `define_registration!` macro and the `Registration` trait.

## ❌ Error Types

### `ApiError` (`sdforge::error::ApiError`, core)

The error enum of the handler return type `Result<T, ApiError>` (`serde` tagged, `thiserror` derived):

| Variant | Fields | Category |
|------|------|------|
| `NotFound` | `resource`, `resource_id: Option<String>` | ClientError |
| `InvalidInput` | `message`, `field: Option<String>`, `value: Option<Value>` | ClientError |
| `AuthenticationFailed` | `reason` | AuthError |
| `AccessDenied` | `permission`, `user_id: Option<String>` | AuthError |
| `RateLimitExceeded` | `limit`, `window_seconds` | RateLimitError |
| `QuotaExhausted` | `used`, `total` | RateLimitError |
| `Internal` | `message` (sanitized), `error_id`, `source`, `context: Option<Box<ErrorContext>>` | ServerError |
| `ServiceUnavailable` | `service`, `retry_after: Option<u64>`, `source` | ServerError |
| `ValidationError` | `field`, `constraint` | ValidationError |

Common methods: `category() -> ErrorCategory` (error category for monitoring/alerting), `source()` (error chain), the `validation_error(code, message)` constructor. `Internal`'s `message` is sanitized and does not leak implementation details.

### `ServiceError` / `ServiceResponse` (`sdforge::core`)

- `ServiceError::new(code, message, status)` / `ServiceError::with_details(code, message, details, status)`: the unified carrier for business errors, supports `From<MyError>` conversion
- `ServiceResponse::success_with_status(data, code)`: dynamically specify the success status code (corresponds to `#[forge(status = ...)]`)

### Unified error contract (`sdforge::error::unified`, cross-protocol single source of truth)

- `mapping_for(&ApiError) -> (u16, &'static str)`: the unique mapping table from `ApiError` → HTTP status code + machine error code (consumed by the HTTP adapter)
- `grpc_code_for(&ApiError) -> tonic::Code` (feature = `grpc`): the same `ApiError` → gRPC status code
- `grpc_code_for_http_status(u16) -> tonic::Code` (feature = `grpc`): the **cross-protocol behavior contract table**, the alignment axis from HTTP status → gRPC status code (same row set as `code_for_http_status`)
- `code_for_http_status(u16) -> &'static str`: HTTP status → machine error code
- `UnifiedError`: the cross-protocol error payload `{code, message, trace_id?, field?, details?}`, shared in shape by HTTP body and gRPC `Status::details`; `details` appears only when same-status errors need distinguishing (e.g. 429's `RateLimitExceeded` / `QuotaExhausted`), and `details.type` holds the error variant name (not localized)

**Error-code behavior contract table** (the values of the same `ApiError` on the two protocol wires; pinned by
the `error_code_contract_tests` wire-level tests and the `unified` unit tests' full-universe JOIN
invariant — drift on either side goes red):

| `ApiError` variant | HTTP status | Payload `code` (shared by both protocols) | gRPC status code |
|-----------------|-----------|--------------------------|-------------|
| `InvalidInput` | 400 | `BAD_REQUEST` | `invalid_argument` |
| `ValidationError` | 422 | `UNPROCESSABLE_ENTITY` | `invalid_argument` |
| `NotFound` | 404 | `NOT_FOUND` | `not_found` |
| `AuthenticationFailed` | 401 | `UNAUTHORIZED` | `unauthenticated` |
| `AccessDenied` | 403 | `FORBIDDEN` | `permission_denied` |
| `RateLimitExceeded` | 429 | `TOO_MANY_REQUESTS` | `resource_exhausted` |
| `QuotaExhausted` | 429 | `TOO_MANY_REQUESTS` | `resource_exhausted` |
| `ServiceUnavailable` | 503 | `UNAVAILABLE` | `unavailable` |
| `Internal` | 500 | `INTERNAL` | `internal` |

Semantic convention: 400 = malformed syntax / missing parameters / parse failure (`InvalidInput`); 422 = semantic constraint violation (`ValidationError` and `#[forge(validate)]` violations, the error body carries an `errors` array).

The 429 family's variant distinction is carried by the payload `details.type` (`RateLimitExceeded` / `QuotaExhausted`);
status code and machine code remain one-status-one-code: clients use them to decide whether to back off and retry or stop retrying, without parsing localized copy.

**Single vocabulary**: this table is the framework's only machine error-code vocabulary. `ApiError::code()` is its sole
derivation entry point, and the HTTP response body (`UnifiedError`), gRPC `Status::details`,
`ServiceError::code()`, and MCP output (`to_mcp_json`) all give the same error the same code
(pinned by the `unified` unit test `every_surface_reports_the_same_machine_code`).
`SdForgeError`'s non-`Api` variants also draw from this table (`Auth` / `Jwt` → `UNAUTHORIZED`,
`AuthConfig` / `Internal` → `INTERNAL`, `Config` → `BAD_REQUEST`), with the error subtype carried
by the error body's `details.type` (`auth` / `jwt` / `auth_config` / `config` /
`internal`), without introducing another machine code.
`ErrorCategory` (`error::context`) is a coarse monitoring classification, not the wire vocabulary.

Cross-protocol scope: gRPC has no native status code corresponding to HTTP 422; 400/422 both land on
`invalid_argument` on the gRPC side, with semantic distinction carried by the HTTP status code and the payload's `code` field (transported via
`Status::details` on the gRPC side); idempotency in-flight 409 → `already_exists`; statuses not registered in
the contract table fall back to `unknown`.

### Idempotency protection (feature = `idempotency`)

- `cache::IdempotencyStore`: protocol-agnostic three-state state machine (`Execute` / `InFlight` / `Replay(body, status)`),
  key prefix `sdforge:idempotency:`, scope bound to route/gRPC method
- `http::idempotency::idempotency_middleware`: the `Idempotency-Key` header (POST/PUT/PATCH),
  replayed responses carry `Idempotency-Replayed: true`, concurrent in-flight returns 409; the `ServerConfig.idempotency`
  configuration section (`enabled` default false / `ttl_secs` default 86400 / `inflight_ttl_secs` default 30 /
  `max_response_bytes` default 1 MiB / `store` — an externally injected shared `IdempotencyStore`,
  created internally by default)
- gRPC: `idempotency-key` metadata, in-flight returns `ALREADY_EXISTS`; injected via
  `GrpcServerConfig::idempotency_store`

### `SdForgeError` (framework-unified error)

`Api(ApiError)`, `Auth(AuthError)`, `Jwt(JwtError)`, `AuthConfig(AuthConfigError)`, `Config(ConfigError)`, `Internal(String)`; plus the `SdForgeResult<T>` alias.

The machine code and HTTP status of `SdForgeError::to_service_error()` share the same source (derived via
`error::unified::code_for_http_status`) and the same vocabulary as the table above:

| Variant | Status | `code` | `details.type` |
|------|------|--------|----------------|
| `Api` | follows `ApiError` | follows `ApiError` | follows `ApiError` |
| `Auth` | 401 | `UNAUTHORIZED` | `auth` |
| `Jwt` | 401 | `UNAUTHORIZED` | `jwt` |
| `AuthConfig` | 500 | `INTERNAL` | `auth_config` |
| `Config` | 400 | `BAD_REQUEST` | `config` |
| `Internal` | 500 | `INTERNAL` | `internal` |

`Auth` / `Jwt` / `AuthConfig` require the `security` feature, `Config` requires the `http` feature.

## 🚪 Feature-Gated API

| Feature | Module | Main APIs |
|---------|------|----------|
| `http` | `sdforge::http` / `config` / `axum` (facade) / `rbac` | `build`, `build_with_config_ext`, `build_with_redirect`, `RouteRegistration`, `validate_email` / `validate_length`, `require_role` (paired with `#[forge(auth(role = "..."))]`); axum/tower/tower-http re-exports |
| `mcp` | `sdforge::mcp` | `SdForgeMcpServer`, `StatelessServerHandler`, `ToolCallContext`, `McpToolInstance` / `McpToolRegistration`, `build()`, `get_mcp_tools()`, `serve_stdio()`, `McpHeaderInfo`, `InputRequiredResult`, `MrtrSession` (the session manager `MrtrSessionManager` is not in the root export — access via `sdforge::mcp::mrtr::MrtrSessionManager`; header parsing `parse_mcp_headers` is likewise not in the root export — access via `sdforge::mcp::headers::parse_mcp_headers`), `cache_semantics`; `rmcp` / `anyhow` re-exports |
| `grpc` | `sdforge::grpc` | `SdForgeGrpcService` (`Call` / `CallStream` / `GetInfo`; `CallStream` also needs `streaming`), `GrpcServerConfig` (`state: Option<Arc<dyn Any + Send + Sync>>`, `require_auth` (default `true`), `auth` (`Option<BearerAuth>` under `security`, degrading to `Option<()>` when disabled), `rate_limiter`, `extra_services: Vec<ExtraServiceMount>` (same-port mounting callbacks for the app's own tonic services; `Clone` shares the same callback list), `max_connections` (default 1000), `timeout_seconds` (default 30s), `http2_keepalive_interval` / `http2_keepalive_timeout` (`None` = tonic defaults), `tls` (`grpc-tls`, `Option<tonic::transport::ServerTlsConfig>`), `idempotency_store` / `idempotency_ttl_secs` (default 86400) / `idempotency_inflight_ttl_secs` (default 30) (`idempotency`)), `build_server_with_config(addr, config)`, `build_server_with_graceful_shutdown(addr, config, signal)` (`signal: Future<Output = ()>`), `build_server(addr)` (**deprecated**, see the note below), `GrpcRoute`, `CallRequest` / `CallResponse` / `InfoRequest` / `InfoResponse`, `SdForgeServiceServer`, `GrpcHandlerRegistration` / `GrpcStreamHandlerRegistration` (`streaming`); `tonic` / `prost` re-exports |
| `websocket` | `sdforge::websocket` | `WebSocketRoute` / `WebSocketHandler`, `websocket_upgrade` / `ValidatedWebSocketUpgrade`, `ConnectionManager`, `WebSocketConfig` / `WebSocketConnection` / `WebSocketMessage`, `parse_websocket_message` / `parse_websocket_message_with_limit` (the latter judges message size against a caller-given byte cap), `build` / `build_with_config` (the latter injects a `WebSocketConfig` with auth, validating JWT / API keys at handshake) |
| `streaming` | `sdforge::streaming` | `StreamEvent`, `StreamResponse`, `stream_to_sse`, `create_stream_channel`, `SseFrame` / `SseFrameParser` / `SseStreamError` / `frames` (inbound SSE frame parsing: WHATWG framing, `with_limits` memory caps, zero extra dependencies) (the `grpc_method` + `stream = true` combination maps to gRPC `CallStream`, see the `grpc` row); `tokio_stream` re-export |
| `ratelimit-dist` | `sdforge::security::ratelimit::dist` | `DistributedRateLimiter` (generic over limiteron `DistributedLimiter` backends: `InMemoryDistributedLimiter` for single-instance / `RedisDistributedLimiter` for multi-replica), `DistributedRateLimitConfig` (with adapter-layer circuit breaking `with_circuit_failure_threshold`/`with_circuit_open_duration`: opens after consecutive backend errors, decides directly by policy without hitting the backend while open, recovers via half-open probing), `BackendFailurePolicy` (fail-open default/fail-close) |
| `cache-l2` | `sdforge::cache::l2` | `RedisL2Cache` (oxcache `RedisBackend` sync surface → `SyncCache`, multi-threaded runtime only; async surface `get_async`/`set_async`/`delete_async`/`contains_async` for async-middleware hot paths), `RedisL2CacheConfig` (AUTH/TLS connection string — Debug output masks credentials, key prefix, default TTL), `CacheL2Error` |
| `security` / `ratelimit` / `ratelimit-http` | `sdforge::security` | Authentication: `ApiKeyAuth`, `BearerAuth(+Builder)`, `SdForgeApiKeyAuth(+Builder)`, `AuthContext`, `AuthExtractor`, `auth_middleware`; Auditing: `AuditLogger` / `SdForgeAuditLogger(+Builder)`, `AuditLog` / `AuditResult`, `AuditSink`; Rate limiting: the `RateLimiter` trait, `LimiteronAdapter`, `RateLimitLayer` (`ratelimit-http`) | <!-- pragma: allowlist secret -->
| `cache` | `sdforge::cache` | `Cache` / `CacheKey`, `SyncCache` / `SharedCache`, `DashMapCache` (`OxcacheSyncCache` alias), `ResponseCacheLayer` / `ResponseCacheMiddleware` / `DEFAULT_MAX_CACHEABLE_BODY_BYTES` / `DEFAULT_CREDENTIAL_HEADER_NAMES` (also needs `http`; key = normalized path + verbatim query; requests carrying credentials bypass the cache, credential header names configured via `ResponseCacheLayer::with_credential_header_names` / `with_credential_headers` or `AuthConfig::credential_header_names()`; responses over the cacheable cap stream through; responses with `Set-Cookie` are not cached); `oxcache` re-export |
| `openapi` | `sdforge::openapi` | `generate_openapi_spec()`, `OpenApiBuilder` (`title` / `version` / `description` / `build`), `OpenApiRouteInfo` / `OpenApiPathParam`; `utoipa` re-export |
| `schemars` | `sdforge::openapi` (reflection API) | `SchemaProbe::probe`, `PreciseSchema` / `FallbackSchema` (return types deriving `JsonSchema` produce field-precise response schemas, silently degrading otherwise; implies `openapi`) |
| `cli` | `sdforge::cli` | `CliBuilder` (`new`, `with_dependencies`, `with_name`, `with_global_arg`, `build -> clap::Command`, `execute -> !`), `dispatch`, `GlobalArg`, `CliCommandRegistration` / `CliHandlerRegistration`; `clap` re-export |
| `docs` | `sdforge::docs` | `generate_docs` / `write_docs`, `DocFormat` / `DocError`; `swagger_ui_router` (also needs `http`) |
| `sdk` | `sdforge::sdk` | `generate_rust_client` / `generate_typescript_client` (pure rendering functions, deterministic `(method, path)` ordering; Rust `Transport` receives full URLs — `base_url` is concatenated by the client), `ClientRoute` / `GrpcMethodInfo` / `collect_routes` / `collect_grpc_methods`; CLI keeps the `sdk --lang ... --output-dir ... [--reqwest]` subcommand |
| `health` | `sdforge::health` | `CheckOutcome` (`healthy` / `unhealthy`), the `ReadinessCheck` / `HealthDataSource` traits, `register_readiness_check(_fn)` |
| `metrics` | `sdforge::metrics` | `MetricsRegistry` (`record` / `render`), `global_registry()`, `record_request()` (Prometheus text-format `/metrics`) |
| `context` | `sdforge::context` | `RequestContext`, `generate_id`, `scope` (request_id/trace_id cross-protocol injection) |
| `lifecycle` | `sdforge::lifecycle` | `LifecycleHookRegistration`, `run_on_start` / `run_on_stop` (paired with `#[forge(on_start / on_stop)]`) |
| `hooks` | `sdforge::hooks` | The `RequestHooks` trait, `install_hooks`, `hooks_middleware` (pre/post handler hook pipeline) |
| `otel` | `sdforge::otel` | `start_span` / `with_attr` / `finish_span` / `take_spans`, `OtelConfig` (OTLP/HTTP JSON export) |
| `inklog` | `sdforge::inklog` / `sdforge::forge::log` | `init_inklog_logger()`: bridges `log` calls into the inklog structured pipeline; the `#[forge::log]` declarative logging attribute macro (`args`/`result`/`err_detail`/`level` parameters, DataMasker masking + 64 KiB truncation, level-guard lazy rendering, `unsafe` qualifier propagation, `log_attr::mask`/`render_enter`/`render_exit` independently reusable) |
| `i18n` | `sdforge::i18n` | `HttpI18nFormatter` (ICU4X locale-aware formatting), `I18nError`, `locale_from_accept_language` / `locale_from_headers` (request language parsing), `scope_request_locale` / `request_locale` (request-scoped locale, consumed preferentially by `t()`) |
| `limiteron-integration` / `kit` / `db-integration` / `db-permission` | `sdforge::integrations` | trait-kit AsyncKit integration (`SdforgeModule`), `LimiteronForgeAdapter`, the dbnexus data API gateway (`DbGateway`: `allow_table` + `query(GatewayQuery)` whitelisted read-only surface, `GatewayQuery::all/filter/paging`, `with_session_role` session role — only enforced by dbnexus's per-table permission checks under `db-permission` (implying `dbnexus/permission`); otherwise it provides only whitelist plus forced-ownership predicate isolation with an explicit warning) |

> Capabilities without a dedicated module: `validate` (`#[forge(validate)]` + `#[param(...)]`, attribute table in [`#[param]` parameter attributes](#param-parameter-attributes)), `paginate` (`#[forge(paginate)]`), `etag` (GET strong ETag + 304), `graceful` (graceful shutdown), `timestamp` (response timestamps), `simd-json` (SIMD JSON path) take effect via macro flags or build configuration.
>
> Independence notes: `mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` are all independent of `http` and can be enabled alone; `security` pulls in `http` + `ratelimit-http` + `cache`; `websocket` needs `http` + `streaming`; `docs` needs `openapi` + `cli`.
>
> **⚠️ `build_server(addr)` is deprecated** (marked `#[deprecated]` in source): this entry point starts an **unauthenticated** gRPC server and cannot configure authentication, kept only for backward compatibility. New code should use `build_server_with_config(addr, config)` and configure `auth` — `require_auth` defaults to `true`, so `auth` being `None` rejects startup at assembly time (preventing accidental deployment of unauthenticated endpoints). With an illegal address format both entry points return the `InvalidInput` error text without echoing parse details.

## 💻 Usage Examples

### Minimal HTTP service

For the complete minimal example of `#[forge]` annotation + `init_all_plugins()` + `http::build()` (including Axum serve and the version prefix explanation), see the [user guide](USER_GUIDE.md#-quick-start).

### Custom error conversion

```rust
impl From<MyError> for ServiceError {
    fn from(err: MyError) -> Self {
        match err {
            MyError::NotFound { resource } => ServiceError::with_details(
                "NOT_FOUND",
                format!("Resource not found: {}", resource),
                serde_json::json!({ "resource": resource }),
                404,
            ),
            // ...
        }
    }
}
```

### OpenAPI generation

```rust
use sdforge::openapi::{generate_openapi_spec, OpenApiBuilder};

let spec = generate_openapi_spec();                 // collect all #[forge] routes
let spec = OpenApiBuilder::new()
    .title("My Service")
    .version("2.0.0")
    .description("User-facing API")
    .build();
```

### One-stop CLI entry point

For the minimal example of `#[forge(cli = true)]` and `CliBuilder::execute()` (returns `!`, exits 0/1 internally), see the [README quick start](../../README.md#-minimal-runnable-example); for advanced CLI usage see the [user guide](USER_GUIDE.md#cli-applications).

More examples are in the repository's `examples/` (see the [user guide · examples](USER_GUIDE.md#-examples) for the full list and how to run them).

## ✅ Best Practices

1. **Always import from `prelude`** — `use sdforge::prelude::*;` guarantees that types referenced by macro-generated code (`ApiError`, `utoipa`, `anyhow`, etc.) resolve downstream
2. **Call `init_all_plugins()` at the earliest point in `main`** — it is idempotent (internally OnceLock-cached), returns `PluginCounts` usable for startup self-checks
3. **Converge error types onto `ApiError` / `ServiceError`** — the error constraint of protocol handlers is `ApiError` (consistent across gRPC/CLI/HTTP); business errors enter the unified pipeline via `From` conversion
4. **Inject state with `with_dependencies` + `downcast_state`** — `CliBuilder::with_dependencies` / `GrpcServerConfig.state` accept `Arc<dyn Any + Send + Sync>`; inside handlers, retrieve via the `#[state]` parameter and `downcast_state::<T>()`
5. **Minimize the API surface per feature** — modules of unenabled features do not exist and the macro generates no code for them; when writing library dependencies declare only the features actually used
6. **Do not bypass the framework re-exports** — use forwarded paths like `sdforge::axum::Router` and `sdforge::tonic::transport::Channel` to avoid version conflicts with direct dependencies
7. **Keep the version field and path parameters consistently named** — the path `:id` segment relies on a same-named function parameter for extraction; the `#[service_module]` prefix and `version` jointly determine the final path

## 🎨 Feature Flags

`default = []`: every feature is optional and enabled explicitly.

Minimum requirements:

- **Rust 1.97.1+** (edition 2024, toolchain pinned by `rust-toolchain.toml`)
- **protoc**: only needed when the `grpc` feature is enabled (`build.rs` compiles the protobuf)

<details>
<summary>📋 Table</summary>

<table>
  <tr><th>Flag</th><th>Description</th><th>Default</th></tr>
  <tr><td><code>http</code></td><td>HTTP server (Axum 0.8 routing, Tower middleware, version routing)</td><td>❌</td></tr>
  <tr><td><code>mcp</code></td><td>MCP protocol (rmcp 3.5, 2026-07-28 spec: stateless HTTP headers, MRTR, cache semantics)</td><td>❌</td></tr>
  <tr><td><code>mcp-http</code></td><td>MCP Streamable HTTP (enables <code>rmcp/transport-streamable-http-server</code>): mount rmcp's <code>StreamableHttpService</code> at <code>/mcp</code> — all types via the <code>sdforge::rmcp</code> re-export, no direct rmcp dependency downstream; mounting uses the <code>sdforge::axum</code> facade; included in <code>full</code></td><td>❌</td></tr>
  <tr><td><code>grpc</code></td><td>gRPC (tonic + prost, independent of http, proto generated by build.rs; unary <code>Call</code> + server-streaming <code>CallStream</code> — the latter also needs <code>streaming</code>; <code>extra_services</code> supports same-port mounting of the app's own tonic services)</td><td>❌</td></tr>
  <tr><td><code>grpc-tls</code></td><td>gRPC TLS wiring (exposes tonic <code>ServerTlsConfig</code>; certificate loading is the caller's responsibility; implemented independently of HTTP-side <code>serve-tls</code> termination and likewise excluded from <code>full</code>)</td><td>❌</td></tr>
  <tr><td><code>websocket</code></td><td>WebSocket (requires http + streaming)</td><td>❌</td></tr>
  <tr><td><code>streaming</code></td><td>SSE streaming (independent of http; combined with <code>grpc</code>, <code>grpc_method</code> + <code>stream = true</code> maps to the gRPC <code>CallStream</code> RPC)</td><td>❌</td></tr>
  <tr><td><code>cli</code></td><td>CLI integration (clap, independent of http)</td><td>❌</td></tr>
  <tr><td><code>openapi</code></td><td>OpenAPI 3.1 spec generation (utoipa, independent of http)</td><td>❌</td></tr>
  <tr><td><code>schemars</code></td><td>Return-type schema reflection (implies openapi: <code>#[forge]</code> emits field-precise response schemas for return types deriving <code>JsonSchema</code>, silently degrading to coarse mapping otherwise)</td><td>❌</td></tr>
  <tr><td><code>docgen</code></td><td>Unified docs output (Swagger UI + CLI/MCP Markdown; requires openapi + cli; legacy name <code>docs</code> remains as a compat alias. Swagger UI v5.32.6 assets are self-vendored under <code>assets/swagger-ui/</code> with zero extra dependencies)</td><td>❌</td></tr>
  <tr><td><code>sdk</code></td><td>Multi-protocol client SDK generation (Rust zero-dep / optional reqwest + TypeScript fetch with type definitions; keeps the CLI <code>sdk</code> subcommand; requires openapi + cli)</td><td>❌</td></tr>
  <tr><td><code>security</code></td><td>Auth (API Key / JWT Bearer), audit, security headers, rate limiting and caching (includes http + ratelimit-http + cache)</td><td>❌</td></tr>
  <tr><td><code>ratelimit</code></td><td>Rate limiting core (limiteron, no http dependency)</td><td>❌</td></tr>
  <tr><td><code>ratelimit-http</code></td><td>HTTP rate limiting middleware (Tower Layer; requires http + ratelimit)</td><td>❌</td></tr>
  <tr><td><code>ratelimit-dist</code></td><td>Distributed rate limiting (limiteron distributed counting backends, quota shared across replicas; unreachable-backend fail-open/fail-close is explicit config, fail-open by default)</td><td>❌</td></tr>
  <tr><td><code>cache</code></td><td>oxcache in-memory cache (independent of http)</td><td>❌</td></tr>
  <tr><td><code>cache-l2</code></td><td>oxcache Redis L2 cache (cross-replica shared cache layer, implies cache; inherently fail-open on failure: read miss / write skip)</td><td>❌</td></tr>
  <tr><td><code>idempotency</code></td><td>Idempotent replay protection (<code>Idempotency-Key</code> HTTP middleware + gRPC metadata path; requires http + cache)</td><td>❌</td></tr>
  <tr><td><code>health</code></td><td><code>/healthz</code> and <code>/readyz</code> probes (auto-mounted by <code>build_with_config_ext</code>, auth bypassed; a same-named route from inventory or the caller's <code>base</code> wins and the built-in probe is skipped with a warning — no panic)</td><td>❌</td></tr>
  <tr><td><code>metrics</code></td><td>Prometheus text-format <code>/metrics</code> endpoint (request counts, latency histograms, status code distribution; lightweight in-house renderer; a same-named route from inventory or the caller's <code>base</code> wins and the built-in endpoint is skipped with a warning — no panic)</td><td>❌</td></tr>
  <tr><td><code>graceful</code></td><td>Graceful shutdown (SIGTERM/SIGINT: stop accepting, drain in-flight, phased teardown)</td><td>❌</td></tr>
  <tr><td><code>serve-tls</code></td><td>HTTP TLS termination (rustls aws-lc-rs: PEM cert/key loading, configurable ALPN, <code>ReloadingTls</code> hot reload; per-request <code>ConnectInfo</code> injection, handshake/header-read timeout guards, shutdown choreography sharing graceful's stop phase. Doc-cross-linked with the gRPC-side <code>grpc-tls</code> but implemented independently)</td><td>❌</td></tr>
  <tr><td><code>tls-watch</code></td><td>Debounced certificate-watch hot reload (confers <code>FsWatcher</code> drives <code>reload_async</code> on cert/key file changes; implies <code>serve-tls</code>; keeps the previous certificate on failed reload)</td><td>❌</td></tr>
  <tr><td><code>context</code></td><td>Request context (request_id/trace_id generation and cross-protocol HTTP/MCP/gRPC/WS injection)</td><td>❌</td></tr>
  <tr><td><code>validate</code></td><td><code>#[forge(validate)]</code> + <code>#[param(ge/le/...)]</code> parameter validation, 422 with field-level errors; struct query parameters extracted flatly via <code>#[param(kind = "query", flatten)]</code></td><td>❌</td></tr>
  <tr><td><code>paginate</code></td><td><code>#[forge(paginate)]</code> declarative pagination (auto page/size and the <code>{items,total,next}</code> wrapper)</td><td>❌</td></tr>
  <tr><td><code>etag</code></td><td>ETag conditional requests (automatic SHA-256 strong ETag on GET; If-None-Match returns 304)</td><td>❌</td></tr>
  <tr><td><code>lifecycle</code></td><td><code>#[forge(on_start/on_stop)]</code> lifecycle hooks (ordered with graceful shutdown)</td><td>❌</td></tr>
  <tr><td><code>hooks</code></td><td>Pre/post handler hook pipeline (middleware-style, unified error contract)</td><td>❌</td></tr>
  <tr><td><code>otel</code></td><td>OTLP/HTTP JSON export of request spans and metric snapshots (zero extra dependencies)</td><td>❌</td></tr>
  <tr><td><code>inklog</code></td><td>inklog structured logging bridge (<code>init_inklog_logger()</code>) + the <code>#[forge::log]</code> declarative logging attribute macro (enter/exit/duration/error + DataMasker masking, loud error when the feature is off). Since 0.5.0-rc it absorbs the removed in-house <code>StructuredLogger</code> stack (the <code>logging</code> feature is gone, breaking)</td><td>❌</td></tr>
  <tr><td><code>timestamp</code></td><td>Response timestamps</td><td>❌</td></tr>
  <tr><td><code>i18n</code></td><td>ICU4X internationalization (locale-aware formatting + Accept-Language parsing)</td><td>❌</td></tr>
  <tr><td><code>simd-json</code></td><td>SIMD-accelerated JSON serialization/deserialization</td><td>❌</td></tr>
  <tr><td><code>limiteron-integration</code></td><td>Pulls in the limiteron dependency (foundation for kit integration)</td><td>❌</td></tr>
  <tr><td><code>db-integration</code></td><td>dbnexus data API gateway (the <code>DbGateway</code> whitelisted read-only data plane: table/column whitelists + filter-value escaping + server-side page clamping; independent of http)</td><td>❌</td></tr>
  <tr><td><code>db-sqlite</code></td><td><code>db-integration</code> + the dbnexus/sqlite driver (since 0.5.1 the dbnexus integration uses <code>embedded-runtime</code> without sqlite; gateway users needing a sqlite file database enable this explicitly)</td><td>❌</td></tr>
  <tr><td><code>db-permission</code></td><td>Enables dbnexus permission: per-table permission checks apply to non-admin session roles (fail-closed); with the feature off the permission checks compile away entirely, leaving only whitelist and ownership-predicate isolation</td><td>❌</td></tr>
  <tr><td><code>kit</code></td><td>trait-kit AsyncKit integration (SdforgeModule graph)</td><td>❌</td></tr>
  <tr><td><code>tokio</code></td><td>Internal feature: enables the tokio dependency (pulled in automatically by other features)</td><td>❌</td></tr>
  <tr><td><code>full</code></td><td>All runtime features (26: http/mcp/grpc/websocket/streaming/security/cache/health/metrics/graceful/context/validate/paginate/etag/hooks/lifecycle/otel/timestamp/openapi/cli/docgen/inklog/i18n/idempotency/serve-tls/tls-watch; excludes the gRPC-side <code>grpc-tls</code> and the optional heavy deps / opt-in surface <code>simd-json</code>/<code>kit</code>/<code>limiteron-integration</code>/<code>schemars</code>/<code>ratelimit-dist</code>/<code>cache-l2</code>/<code>db-integration</code>/<code>sdk</code>)</td><td>❌</td></tr>
</table>

</details>

<details>
<summary>🔗 Feature dependency relations</summary>

- Independent of `http`: `mcp` / `grpc` / `openapi` / `cli` / `streaming` / `cache` / `timestamp` / `context` / `inklog` / `i18n` / `simd-json` / `limiteron-integration` / `db-integration` (with derived `db-sqlite` / `db-permission`) / `schemars` / `ratelimit-dist`
- **check-cfg snippet for downstream mirrored features** (macro-emitted code is gated on the downstream crate's same-named
  features; see the `[lints]` whitelist pattern in examples): add to the downstream `Cargo.toml`
  `[lints.rust] unexpected_cfgs = { level = "allow", check-cfg =
  ['cfg(feature, values("http", "mcp", "grpc", "cli", "websocket",
  "openapi", "security", "lifecycle"))'] }` (trimmed to the mirrored features actually
  enabled), to avoid unknown-cfg warnings
- Derived from `http`: `security` (includes `ratelimit-http` → `ratelimit` and `cache`), `ratelimit-http`, `websocket` (includes `streaming`), `health` / `metrics` / `graceful` / `serve-tls` (includes `graceful`) / `tls-watch` (includes `serve-tls`) / `validate` / `paginate` / `lifecycle` / `hooks` / `otel`
- `docgen` = `openapi` + `cli` (mounting the Swagger UI additionally requires `http`; legacy name `docs` remains as a compat alias)
- `kit` = `trait-kit` (health + lifecycle) + `limiteron-integration` + `limiteron/kit` + `oxcache/kit`
- `full` covers 26 runtime features (incl. `idempotency`, `docgen`, deployment-surface HTTP TLS `serve-tls`/`tls-watch`) and excludes the optional heavy deps `simd-json` / `kit` / `limiteron-integration` (SIMD JSON, trait-kit module graph, rate-limit integration foundation) plus the gRPC-side `grpc-tls` (gRPC-side TLS wiring) / `schemars` / `ratelimit-dist` / `cache-l2` / `db-integration` / `sdk` (opt-in surface), all enabled individually as needed

> ℹ️ **`docgen` availability note** (the 2026-10-16 upstream `zip` yank): that incident broke dependency resolution of `docgen`'s dependency `utoipa-swagger-ui` (pinned to `zip ^8.6`), so enabling `docgen` from scratch (including `full`, which covers it) failed outright. The dependency has now been removed: the Swagger UI v5.32.6 official dist functional subset (Apache-2.0, modifications documented in the asset `NOTICE.md`) is self-vendored under `assets/swagger-ui/`, embedded at compile time via `include_bytes!`, and served directly by `swagger_ui_router*`; `docgen` is now just `openapi + cli`, so downstream dependency resolution works again. Refresh assets with `scripts/update_swagger_ui.sh`.

</details>
