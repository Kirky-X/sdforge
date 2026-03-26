// Copyright (c) 2026 Kirky.X
//! SDForge runtime library
//!
//! This crate provides the runtime types and service builders for the SDForge framework.

#![doc(html_root_url = "https://docs.rs/sdforge/0.2.0")]
#![warn(missing_docs)]

/// Re-export macros from sdforge-macros for convenient use
pub use sdforge_macros::{service_api, service_module, test_macro};

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
    feature = "grpc"
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
        pub use axum::routing::{get, post, MethodRouter};
    }

    /// Extractor utilities for HTTP requests
    pub mod extract {
        pub use axum::extract::{Form, Json, Path, Query};
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
    pub use crate::core::validation::validators::{validate_email, validate_length};
    pub use crate::core::{ApiError, ApiMetadata, ServiceError, ServiceResponse};
    #[cfg(feature = "http")]
    pub use crate::http::{HttpRoute, RouteRegistration};
    #[cfg(feature = "mcp")]
    pub use crate::mcp::McpToolInstance;

    #[cfg(feature = "http")]
    pub use crate::axum::IntoResponse;

    pub use sdforge_macros::{service_api, service_module, test_macro};
}

/// Core types and utilities
pub mod core;

/// HTTP server and routing
#[cfg(feature = "http")]
pub mod http;

/// MCP (Model Context Protocol) support
#[cfg(feature = "mcp")]
pub mod mcp;

// Create a module hierarchy for mcp-sdk to match generated code expectations
#[cfg(feature = "mcp")]
/// MCP SDK types re-exported for compatibility with generated code
pub mod mcp_sdk_types {
    /// MCP tool types
    pub mod tools {
        pub use ::mcp_sdk::tools::Tool;
    }

    /// MCP type definitions
    pub mod types {
        pub use ::mcp_sdk::types::CallToolResponse;
        pub use ::mcp_sdk::types::ToolResponseContent;
    }
}

// Make mcp_sdk available for generated code (aliases mcp_sdk_types)
#[cfg(feature = "mcp")]
pub use mcp_sdk_types as mcp_sdk;

// Re-export types at crate root for convenience
#[cfg(feature = "mcp")]
pub use mcp_sdk_types::tools::Tool;
#[cfg(feature = "mcp")]
pub use mcp_sdk_types::types::CallToolResponse;
#[cfg(feature = "mcp")]
pub use mcp_sdk_types::types::ToolResponseContent;

/// Streaming utilities for SSE and streaming responses
#[cfg(feature = "streaming")]
pub mod streaming;

#[cfg(feature = "streaming")]
pub use streaming::{create_stream_channel, stream_to_sse, StreamEvent, StreamResponse};

/// Security middleware and authentication utilities
#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "security")]
pub use security::{
    auth_middleware,
    rate_limit_middleware,
    // Trait interfaces (feature layer)
    ApiKeyAuth,
    // Concrete implementations (renamed structs)
    AppApiKeyAuth,
    AppApiKeyAuthBuilder,
    AppAuditLogger,
    AppAuditLoggerBuilder,
    AppRateLimiter,
    AppRateLimiterBuilder,
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
    RateLimitConfig,
    RateLimitError,
    RateLimiter,
};

/// Configuration management
#[cfg(feature = "http")]
pub mod config;

/// 直接透传 confers 库（配置管理由 confers 统一提供）
#[cfg(feature = "http")]
pub use confers;
/// confers 的 Config trait（用于派生配置结构体）
#[cfg(feature = "http")]
pub use confers::Config;

#[cfg(feature = "http")]
pub use config::{
    ApiConfig, AppConfig, AuthConfig, ConfigError, CorsConfig, EnvHelper, RateLimitConfigFile,
    RateLimitEndpointConfig, ServerConfig, TlsConfig, TracingConfig,
};

#[cfg(feature = "hot-reload")]
pub use config::hot_reload::{
    create_config_watcher, ConfigEvent, ConfigManager, ConfigWatcherImpl,
};

/// 直接透传 oxcache 库（缓存功能由 oxcache 统一提供）
#[cfg(feature = "cache")]
pub use oxcache;

/// 缓存抽象模块（功能组件的标准同步存储接口）
#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "cache")]
pub use cache::{DashMapCache, SharedCache, SyncCache};

/// 直接透传 inklog 库（日志功能由 inklog 统一提供）
#[cfg(feature = "logging")]
pub use inklog;

/// WebSocket support
#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "websocket")]
pub use websocket::{
    build, parse_websocket_message, websocket_upgrade, BoxFuture, ConnectionManager,
    ValidatedWebSocketUpgrade, WebSocketConfig, WebSocketConnection, WebSocketHandler,
    WebSocketMessage, WebSocketRoute,
};

/// gRPC server support
#[cfg(feature = "grpc")]
pub mod grpc;

#[cfg(feature = "grpc")]
pub use grpc::{
    build_server, build_server_with_config, GrpcRoute, GrpcServerConfig, SdForgeGrpcService,
};

#[cfg(feature = "grpc")]
pub use grpc::sdforge_v1::{
    sd_forge_service_server::SdForgeServiceServer, CallRequest, CallResponse, InfoRequest,
    InfoResponse,
};

#[cfg(feature = "http")]
pub use http::version_routing::{build_version_router, VersionRouterConfig, VersionedRoute};

/// 日志功能由 sdforge::inklog 统一管理（已移除内置日志实现）
///
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
///     println!("Routes: {}, MCP: {}, WS: {}, gRPC: {}",
///         counts.routes, counts.mcp_tools, counts.ws_routes, counts.grpc_routes);
/// }
/// ```
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "websocket",
    feature = "grpc"
))]
pub fn init_all_plugins() -> PluginCounts {
    use std::sync::Mutex;
    use std::sync::OnceLock;

    // Store in global static to prevent linker optimization
    #[cfg(feature = "http")]
    let routes = {
        use crate::http::RouteRegistration;

        static ROUTES: OnceLock<Mutex<Vec<&'static RouteRegistration>>> = OnceLock::new();
        let routes =
            ROUTES.get_or_init(|| Mutex::new(inventory::iter::<RouteRegistration>().collect()));
        routes.lock().unwrap().len()
    };
    #[cfg(not(feature = "http"))]
    let routes = 0;

    #[cfg(feature = "mcp")]
    let mcp_tools = {
        use crate::mcp::McpToolRegistration;

        static MCP_TOOLS: OnceLock<Mutex<Vec<&'static McpToolRegistration>>> = OnceLock::new();
        let tools = MCP_TOOLS
            .get_or_init(|| Mutex::new(inventory::iter::<McpToolRegistration>().collect()));
        tools.lock().unwrap().len()
    };
    #[cfg(feature = "websocket")]
    let ws_routes = {
        use crate::websocket::WebSocketRoute;

        static WS_ROUTES: OnceLock<Mutex<Vec<&'static WebSocketRoute>>> = OnceLock::new();
        let routes =
            WS_ROUTES.get_or_init(|| Mutex::new(inventory::iter::<WebSocketRoute>().collect()));
        routes.lock().unwrap().len()
    };
    #[cfg(feature = "grpc")]
    let grpc_routes = {
        use crate::grpc::GrpcRouteRegistration;

        static GRPC_ROUTES: OnceLock<Mutex<Vec<&'static GrpcRouteRegistration>>> = OnceLock::new();
        let routes = GRPC_ROUTES
            .get_or_init(|| Mutex::new(inventory::iter::<GrpcRouteRegistration>().collect()));
        routes.lock().unwrap().len()
    };

    PluginCounts {
        routes,
        #[cfg(feature = "mcp")]
        mcp_tools,
        #[cfg(feature = "websocket")]
        ws_routes,
        #[cfg(feature = "grpc")]
        grpc_routes,
    }
}

/// Counts of registered plugins after initialization
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "websocket",
    feature = "grpc"
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
}

/// Get all registered MCP tools
#[cfg(feature = "mcp")]
pub fn get_mcp_tools() -> Vec<crate::mcp::McpToolInstance> {
    crate::mcp::get_mcp_tools()
}

/// Get all registered WebSocket routes
#[cfg(feature = "websocket")]
pub fn get_websocket_routes() -> Vec<&'static crate::websocket::WebSocketRoute> {
    inventory::iter::<crate::websocket::WebSocketRoute>().collect()
}
