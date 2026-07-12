// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! SDForge runtime library
//!
//! This crate provides the runtime types and service builders for the SDForge framework.

#![doc(html_root_url = "https://docs.rs/sdforge/0.4.0")]
#![warn(missing_docs)]

// Allow macro-generated code (which references `sdforge::cli::...`,
// `sdforge::prelude::ApiError`, etc.) to resolve when the `#[forge]`
// macro is expanded inside the sdforge crate itself — e.g. in the in-crate
// `src/cli/tests/macro_integration_tests.rs` suite. Downstream crates resolve
// `sdforge::` naturally via the extern prelude; this alias mirrors that for
// self-references. `pub` visibility still applies, so only `pub` items are
// reachable through `sdforge::`.
extern crate self as sdforge;

/// Re-export macros from sdforge-macros for convenient use
pub use sdforge_macros::{forge, service_module, test_macro};

/// Macro to implement Default::default() constructor for types
///
/// This macro generates both a `new()` static method and implements `Default` trait.
/// Useful for simple types that can be constructed with no arguments.
///
/// # Example
///
/// ```ignore
/// use sdforge::impl_default_new;
///
/// // Works with unit structs:
/// struct EmptyConfig;
/// impl_default_new!(EmptyConfig);
///
/// let config = EmptyConfig::new();
/// ```
#[macro_export]
macro_rules! impl_default_new {
    ($type:ident) => {
        impl $type {
            /// Create a new instance with default values
            pub fn new() -> Self {
                Self
            }
        }

        impl Default for $type {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Re-export inventory for use in generated code
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "websocket",
    feature = "grpc",
    feature = "cli"
))]
pub use inventory;

/// Re-export tokio_stream for use in generated code
#[cfg(feature = "streaming")]
pub use tokio_stream;

/// Re-export axum types for use in generated code
#[cfg(feature = "http")]
pub mod axum {
    pub use axum::body::Body;
    pub use axum::response::IntoResponse;

    /// HTTP routing utilities
    pub mod routing {
        pub use axum::routing::{MethodRouter, get, post};
    }

    /// Extractor utilities for HTTP requests
    pub mod extract {
        pub use axum::extract::{Extension, Form, Json, Path, Query};
        pub use axum_extra::TypedHeader;
    }

    /// HTTP types and utilities
    pub mod http {
        pub use axum::http::Request;
        /// HTTP header utilities
        pub mod header {
            pub use axum::http::header::CONTENT_TYPE;
        }
        /// HTTP status codes
        pub mod status {
            pub use axum::http::StatusCode;
        }
    }

    /// Handler utilities
    pub mod handler {
        pub use axum::handler::Handler;
    }

    pub use axum::serve;
}

/// Commonly used types and re-exports
pub mod prelude {
    #[cfg(feature = "http")]
    pub use crate::core::{validate_email, validate_length};
    pub use crate::core::{ApiError, ApiMetadata, ServiceError, ServiceResponse};
    #[cfg(feature = "http")]
    pub use crate::http::{HttpRoute, RouteRegistration};
    #[cfg(feature = "mcp")]
    pub use crate::mcp::McpToolInstance;

    #[cfg(feature = "http")]
    pub use crate::axum::IntoResponse;

    pub use sdforge_macros::{forge, service_module, test_macro};
}

/// Core types and utilities
pub mod core;

/// Framework error types
///
/// Provides comprehensive error types for the framework. See `SdForgeError`
/// for the unified error enum and `SdForgeResult<T>` for the standard result alias.
pub mod error;

/// Domain abstractions consumed by integrations (e.g. `ForgeRateLimiter`).
pub mod domain;

/// Integration modules connecting sdforge to external frameworks via
/// trait-kit 0.2.2 `AsyncKit`. Gated by `limiteron-integration` (which `kit`
/// implies).
#[cfg(any(feature = "limiteron-integration", feature = "kit"))]
pub mod integrations;

/// HTTP server and routing
#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "http")]
pub use http::version_routing::{VersionRouterConfig, VersionedRoute, build_version_router};

/// MCP (Model Context Protocol) support — built on official rmcp SDK
#[cfg(feature = "mcp")]
pub mod mcp;

// Re-export rmcp types for convenience (replaces old mcp_sdk re-exports)
#[cfg(feature = "mcp")]
pub use mcp::{
    InputRequiredResult, McpHeaderInfo, McpToolInstance, McpToolRegistration, MrtrSession,
    SdForgeMcpServer, SdForgeTool, StatelessServerHandler, build, get_mcp_tools,
};

/// Streaming utilities for SSE and streaming responses
#[cfg(feature = "streaming")]
pub mod streaming;

#[cfg(feature = "streaming")]
pub use streaming::{StreamEvent, StreamResponse, create_stream_channel, stream_to_sse};

/// Security middleware and authentication utilities.
///
/// Available when either `security` or `ratelimit` is enabled: the
/// `ratelimit` submodule lives under `crate::security::ratelimit` and must be
/// reachable even when the full `security` feature is off. Non-ratelimit
/// submodules are individually gated by `feature = "security"` inside
/// `src/security/mod.rs`.
#[cfg(any(feature = "security", feature = "ratelimit"))]
pub mod security;

#[cfg(feature = "security")]
pub use security::{
    // Trait interfaces (feature layer)
    ApiKeyAuth,
    // Concrete implementations (renamed structs)
    AppApiKeyAuth,
    AppApiKeyAuthBuilder,
    AppAuditLogger,
    AppAuditLoggerBuilder,
    AuditLog,
    AuditLogger,
    // Supporting types
    AuditResult,
    AuthContext,
    AuthError,
    AuthExtractor,
    AuthMetadata,
    AuthResult,
    BearerAuth,
    BearerAuthBuilder,
    auth_middleware,
};

/// Configuration management
#[cfg(feature = "http")]
pub mod config;

#[cfg(feature = "http")]
pub use config::{
    ApiConfig, AppConfig, AuthConfig, ConfigError, CorsConfig, EnvHelper, ServerConfig, TlsConfig,
    TracingConfig,
};

/// 直接透传 oxcache 库（缓存功能由 oxcache 统一提供）
#[cfg(feature = "cache")]
pub use oxcache;

/// 缓存模块（直接透传 oxcache 的缓存接口）
#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "cache")]
pub use cache::{Cache, CacheKey, DashMapCache, OxcacheSyncCache, SharedCache, SyncCache};

/// WebSocket support
#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "websocket")]
pub use websocket::{
    BoxFuture, ConnectionManager, ValidatedWebSocketUpgrade, WebSocketConfig, WebSocketConnection,
    WebSocketHandler, WebSocketMessage, WebSocketRoute, parse_websocket_message, websocket_upgrade,
};

/// gRPC server support
#[cfg(feature = "grpc")]
pub mod grpc;

#[cfg(feature = "grpc")]
pub use grpc::{
    GrpcRoute, GrpcServerConfig, SdForgeGrpcService, build_server, build_server_with_config,
};

#[cfg(feature = "grpc")]
pub use grpc::sdforge_v1::{
    CallRequest, CallResponse, InfoRequest, InfoResponse,
    sd_forge_service_server::SdForgeServiceServer,
};

/// Structured logging utilities
#[cfg(feature = "logging")]
pub mod logging;

#[cfg(feature = "logging")]
pub use logging::{
    LogEntry, LogLevel, LoggerConfig, StructuredLogger, get_global_logger, init_global_logger,
};

/// inklog 结构化日志集成 — 将裸 `log` 输出桥接到 inklog LoggerManager。
///
/// 启用 `inklog` feature 后，调用 [`inklog::init_inklog_logger`] 即可
/// 将 inklog 安装为全局 `log` 后端。此后所有 `log::error!`/`log::warn!`
/// 等调用自动路由到 inklog 的结构化日志管道，无需修改现有调用点。
///
/// 未启用 `inklog` feature 时，此模块不存在，`log` 行为完全不变。
#[cfg(feature = "inklog")]
pub mod inklog;

/// ICU4X-backed internationalization — locale-aware HTTP formatting.
///
/// 启用 `i18n` feature 后可用。提供 `HttpI18nFormatter`：BCP-47 locale
/// 管理、Accept-Language 头解析、HTTP 错误消息/数字/时间戳/排序格式化。
/// 未启用时此模块不存在，默认 features 编译零开销。
#[cfg(feature = "i18n")]
pub mod i18n;

/// CLI (clap) integration — feature-gated by `cli`.
///
/// Promoted to `pub mod cli;` in T009 so that macro-generated
/// `sdforge::cli::CliCommandRegistration` paths (emitted by
/// `#[forge(cli = true)]`) resolve both inside the sdforge crate
/// (via `extern crate self as sdforge;` above) and in downstream crates.
/// T010 wires the inventory iteration into `init_all_plugins`.
#[cfg(feature = "cli")]
pub mod cli;

/// OpenAPI 3.1 specification generation.
///
/// Only available when the `openapi` feature is enabled. See [`openapi`] module
/// docs for usage.
#[cfg(feature = "openapi")]
pub mod openapi;

#[cfg(feature = "openapi")]
pub use openapi::{OpenApiBuilder, OpenApiPathParam, OpenApiRouteInfo, generate_openapi_spec};

/// 统一文档输出模块 — Swagger UI + CLI/MCP Markdown。
///
/// 仅当 `docs` feature 启用时可用。T011-T019 实现逐步填充。
#[cfg(feature = "docs")]
pub mod docs;

#[cfg(feature = "docs")]
pub use docs::{DocError, DocFormat, generate_docs, write_docs};

#[cfg(all(feature = "docs", feature = "http"))]
pub use docs::swagger_ui_router;

/// 初始化所有已注册的插件，确保它们不会被链接器优化掉。
///
/// This function must be called at least once to ensure that all inventory-based
/// registrations (HTTP routes, MCP tools, WebSocket routes, gRPC routes) are linked
/// into the final binary. Call this at the start of your application.
///
/// Returns the count of registered items for each type for debugging purposes.
///
/// # Example
///
/// ```ignore
/// use sdforge::init_all_plugins;
///
/// fn main() {
///     let counts = sdforge::init_all_plugins();
/// }
/// ```
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "websocket",
    feature = "grpc",
    feature = "cli"
))]
pub fn init_all_plugins() -> PluginCounts {
    use std::sync::Mutex;
    use std::sync::OnceLock;

    // Store in global static to prevent linker optimization.
    // Poison-aware: 若任一 inventory 收集期间 panic 导致 Mutex 中毒，
    // 降级返回 0 而非连锁 panic，避免 init_all_plugins 永久失效。
    #[cfg(feature = "http")]
    let routes = {
        use crate::http::RouteRegistration;

        static ROUTES: OnceLock<Mutex<Vec<&'static RouteRegistration>>> = OnceLock::new();
        let routes =
            ROUTES.get_or_init(|| Mutex::new(inventory::iter::<RouteRegistration>().collect()));
        routes.lock().map(|g| g.len()).unwrap_or_else(|e| {
            log::error!("inventory Mutex poisoned: {}", e);
            0
        })
    };
    #[cfg(not(feature = "http"))]
    let routes = 0;

    #[cfg(feature = "mcp")]
    let mcp_tools = {
        use crate::mcp::McpToolRegistration;

        static MCP_TOOLS: OnceLock<Mutex<Vec<&'static McpToolRegistration>>> = OnceLock::new();
        let tools = MCP_TOOLS
            .get_or_init(|| Mutex::new(inventory::iter::<McpToolRegistration>().collect()));
        tools.lock().map(|g| g.len()).unwrap_or_else(|e| {
            log::error!("inventory Mutex poisoned: {}", e);
            0
        })
    };
    #[cfg(feature = "websocket")]
    let ws_routes = {
        use crate::websocket::WebSocketRoute;

        static WS_ROUTES: OnceLock<Mutex<Vec<&'static WebSocketRoute>>> = OnceLock::new();
        let routes =
            WS_ROUTES.get_or_init(|| Mutex::new(inventory::iter::<WebSocketRoute>().collect()));
        routes.lock().map(|g| g.len()).unwrap_or_else(|e| {
            log::error!("inventory Mutex poisoned: {}", e);
            0
        })
    };
    #[cfg(feature = "grpc")]
    let grpc_routes = {
        use crate::grpc::GrpcRouteRegistration;

        static GRPC_ROUTES: OnceLock<Mutex<Vec<&'static GrpcRouteRegistration>>> = OnceLock::new();
        let routes = GRPC_ROUTES
            .get_or_init(|| Mutex::new(inventory::iter::<GrpcRouteRegistration>().collect()));
        routes.lock().map(|g| g.len()).unwrap_or_else(|e| {
            log::error!("inventory Mutex poisoned: {}", e);
            0
        })
    };

    // T010: touch CLI inventory so the linker keeps `inventory::submit!`
    // blocks emitted by `#[forge(cli = true)]`. Mirrors the http/mcp/
    // websocket/grpc blocks above. Both `CliCommandRegistration` and
    // `CliHandlerRegistration` are collected; the returned count reflects
    // command registrations (handler registrations are paired 1:1).
    #[cfg(feature = "cli")]
    let cli_commands = {
        use crate::cli::{CliCommandRegistration, CliHandlerRegistration};

        static CLI_CMDS: OnceLock<Mutex<Vec<&'static CliCommandRegistration>>> = OnceLock::new();
        let cmds = CLI_CMDS
            .get_or_init(|| Mutex::new(inventory::iter::<CliCommandRegistration>().collect()));

        // Also iterate handler registrations to prevent the linker from
        // stripping the paired `CliHandlerRegistration` submit blocks.
        static CLI_HANDLERS: OnceLock<Mutex<Vec<&'static CliHandlerRegistration>>> =
            OnceLock::new();
        let _handlers = CLI_HANDLERS
            .get_or_init(|| Mutex::new(inventory::iter::<CliHandlerRegistration>().collect()));

        cmds.lock().map(|g| g.len()).unwrap_or_else(|e| {
            log::error!("inventory Mutex poisoned: {}", e);
            0
        })
    };

    PluginCounts {
        routes,
        #[cfg(feature = "mcp")]
        mcp_tools,
        #[cfg(feature = "websocket")]
        ws_routes,
        #[cfg(feature = "grpc")]
        grpc_routes,
        #[cfg(feature = "cli")]
        cli_commands,
    }
}

/// Counts of registered plugins after initialization
///
/// This struct provides visibility into which protocol implementations
/// have been registered via inventory and are available at runtime.
///
/// # Usage
///
/// ```ignore
/// use sdforge::init_all_plugins;
///
/// fn main() {
///     let counts = init_all_plugins();
///     
///     println!("Registered:");
///     println!("  HTTP routes: {}", counts.routes);
///     #[cfg(feature = "mcp")]
///     println!("  MCP tools: {}", counts.mcp_tools);
///     #[cfg(feature = "websocket")]
///     println!("  WebSocket routes: {}", counts.ws_routes);
///     #[cfg(feature = "grpc")]
///     println!("  gRPC routes: {}", counts.grpc_routes);
/// }
/// ```
///
/// # Feature Flags
///
/// Fields are conditionally compiled based on features:
/// - `routes`: Always present when any protocol feature is enabled
/// - `mcp_tools`: Only with `mcp` feature
/// - `ws_routes`: Only with `websocket` feature
/// - `grpc_routes`: Only with `grpc` feature
/// - `cli_commands`: Only with `cli` feature
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "websocket",
    feature = "grpc",
    feature = "cli"
))]
pub struct PluginCounts {
    /// Number of registered HTTP routes
    pub routes: usize,
    /// Number of registered MCP tools
    #[cfg(feature = "mcp")]
    pub mcp_tools: usize,
    /// Number of registered WebSocket routes
    #[cfg(feature = "websocket")]
    pub ws_routes: usize,
    /// Number of registered gRPC routes
    #[cfg(feature = "grpc")]
    pub grpc_routes: usize,
    /// Number of registered CLI commands (emitted by `#[forge(cli = true)]`)
    #[cfg(feature = "cli")]
    pub cli_commands: usize,
}

/// Get all registered WebSocket routes
#[cfg(feature = "websocket")]
pub fn get_websocket_routes() -> Vec<&'static crate::websocket::WebSocketRoute> {
    inventory::iter::<crate::websocket::WebSocketRoute>().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // init_all_plugins tests
    //
    // init_all_plugins uses process-wide OnceLocks to cache inventory iterations,
    // so the function is idempotent: the first call initializes the locks and
    // subsequent calls return the same counts. We use #[serial] to ensure these
    // tests don't run concurrently with any other test that may touch the same
    // globals.
    // ============================================================================

    #[test]
    #[serial_test::serial]
    fn test_init_all_plugins_returns_counts() {
        let counts = init_all_plugins();
        // With the `full` feature, all protocol features are enabled. The exact
        // counts depend on how many inventory items are registered in the test
        // binary, so we only assert structural correctness here.
        let _ = counts.routes;
        #[cfg(feature = "mcp")]
        let _ = counts.mcp_tools;
        #[cfg(feature = "websocket")]
        let _ = counts.ws_routes;
        #[cfg(feature = "grpc")]
        let _ = counts.grpc_routes;
        #[cfg(feature = "cli")]
        let _ = counts.cli_commands;
    }

    #[test]
    #[serial_test::serial]
    fn test_init_all_plugins_is_idempotent() {
        // Calling init_all_plugins twice should return consistent counts
        // because the OnceLocks cache the inventory iteration results.
        let first = init_all_plugins();
        let second = init_all_plugins();
        assert_eq!(first.routes, second.routes);
        #[cfg(feature = "mcp")]
        assert_eq!(first.mcp_tools, second.mcp_tools);
        #[cfg(feature = "websocket")]
        assert_eq!(first.ws_routes, second.ws_routes);
        #[cfg(feature = "grpc")]
        assert_eq!(first.grpc_routes, second.grpc_routes);
        #[cfg(feature = "cli")]
        assert_eq!(first.cli_commands, second.cli_commands);
    }

    // ============================================================================
    // get_mcp_tools tests
    // ============================================================================

    #[cfg(feature = "mcp")]
    #[test]
    fn test_get_mcp_tools_returns_vec() {
        // get_mcp_tools collects inventory::iter::<McpToolRegistration> into
        // a Vec<McpToolInstance>. The exact count depends on how many tools
        // are registered in the test binary, so we only assert it returns
        // without panicking.
        let tools = get_mcp_tools();
        let _ = tools.len();
    }

    // ============================================================================
    // get_websocket_routes tests
    // ============================================================================

    #[cfg(feature = "websocket")]
    #[test]
    fn test_get_websocket_routes_returns_vec() {
        // get_websocket_routes collects inventory::iter::<WebSocketRoute>
        // into a Vec. The exact count depends on registrations in the test
        // binary, so we only assert it returns without panicking.
        let routes = get_websocket_routes();
        let _ = routes.len();
    }

    // ============================================================================
    // impl_default_new macro test
    // ============================================================================

    #[test]
    fn test_impl_default_new_macro() {
        struct EmptyConfig;
        impl_default_new!(EmptyConfig);

        let config = EmptyConfig::new();
        let _default = EmptyConfig;
        // Ensure both construction paths produce the same type
        let _: EmptyConfig = config;
    }

    // ============================================================================
    // impl_default_new! macro: Default trait integration
    //
    // The macro generates both `new()` and `Default::default()`. The existing
    // test only calls `new()`. These tests verify the generated `Default`
    // impl produces an equivalent value and that the macro can be applied to
    // multiple distinct unit structs in the same scope.
    // ============================================================================

    /// Test the impl_default_new! macro generates a working Default impl
    /// whose `default()` equals `new()`.
    #[test]
    #[allow(clippy::default_constructed_unit_structs)] // test asserts Default impl exists
    fn test_impl_default_new_macro_generates_default_trait() {
        struct ConfigA;
        impl_default_new!(ConfigA);

        let from_new = ConfigA::new();
        let from_default = ConfigA::default();
        // Unit structs have a single value, so both must be equal in type.
        let _: ConfigA = from_new;
        let _: ConfigA = from_default;
    }

    /// Test the impl_default_new! macro can be applied to multiple distinct
    /// unit structs without name collisions.
    #[test]
    #[allow(clippy::default_constructed_unit_structs)] // test asserts Default impl exists
    fn test_impl_default_new_macro_multiple_structs() {
        struct FirstConfig;
        struct SecondConfig;
        impl_default_new!(FirstConfig);
        impl_default_new!(SecondConfig);

        let _a = FirstConfig::new();
        let _b = SecondConfig::new();
        let _a2 = FirstConfig::default();
        let _b2 = SecondConfig::default();
    }

    // ============================================================================
    // PluginCounts struct: field access under the http,cache feature set
    //
    // When only `http` and `cache` features are enabled, PluginCounts has a
    // single `routes` field (the mcp_tools/ws_routes/grpc_routes fields are
    // cfg-gated out). These tests verify the struct can be constructed and
    // its field accessed without panicking.
    // ============================================================================

    /// Test PluginCounts can be constructed via init_all_plugins and the
    /// `routes` field is a non-negative usize.
    #[test]
    #[serial_test::serial]
    fn test_plugin_counts_routes_field_is_usize() {
        let counts = init_all_plugins();
        // routes is always present when any protocol feature is enabled.
        let routes: usize = counts.routes;
        // usize is always >= 0 by definition; this just ensures the field
        // is accessible and has a sane value type.
        let _ = routes;
    }

    /// Test calling init_all_plugins multiple times returns consistent counts
    /// (idempotency via the internal OnceLocks), verifying the cached
    /// inventory iteration does not change between calls.
    #[test]
    #[serial_test::serial]
    fn test_init_all_plugins_counts_are_stable_across_calls() {
        let a = init_all_plugins();
        let b = init_all_plugins();
        let c = init_all_plugins();
        assert_eq!(a.routes, b.routes);
        assert_eq!(b.routes, c.routes);
    }
}
