// Copyright (c) 2026 Kirky.X
//! SDForge runtime library
//!
//! This crate provides the runtime types and service builders for the SDForge framework.

#![doc(html_root_url = "https://docs.rs/sdforge/0.2.0")]
#![warn(missing_docs)]

/// Re-export macros from sdforge-macros for convenient use
pub use sdforge_macros::{service_api, service_module, test_macro};

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
    auth_middleware, rate_limit_middleware, ApiKeyAuth, AuditLog, AuditLogger, AuditResult,
    AuthContext, AuthError, AuthExtractor, AuthMetadata, AuthResult, BearerAuth, RateLimitConfig,
    RateLimitError, RateLimiter,
};

/// Configuration management
#[cfg(feature = "http")]
pub mod config;

#[cfg(feature = "http")]
pub use config::{
    ApiConfig, AppConfig, AuthConfig, ConfigError, ConfigLoader, CorsConfig, DatabaseConfig,
    EnvHelper, LoggingConfig, RateLimitConfigFile, RateLimitEndpointConfig, ServerConfig,
    TlsConfig, TracingConfig,
};

#[cfg(feature = "hot-reload")]
pub use config::hot_reload::ConfigWatcher;

/// Caching utilities and middleware
#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "cache")]
pub use cache::{CacheConfig, CacheMiddleware, CacheService};

/// WebSocket support
#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "websocket")]
pub use websocket::{
    build, websocket_upgrade, BoxFuture, ConnectionManager, WebSocketConnection, WebSocketHandler,
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

#[cfg(feature = "logging")]
pub use config::{init_logging, init_logging_default};

/// Initialize all registered plugins and ensure they are not optimized away by the linker.
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
    #[cfg(feature = "grpc")]
    use crate::grpc::GrpcRoute;
    use crate::http::RouteRegistration;
    #[cfg(feature = "mcp")]
    use crate::mcp::McpToolRegistration;
    #[cfg(feature = "websocket")]
    use crate::websocket::WebSocketRoute;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;

    // Store in global static to prevent linker optimization
    static ROUTES: Lazy<Mutex<Vec<&'static RouteRegistration>>> =
        Lazy::new(|| Mutex::new(inventory::iter::<RouteRegistration>().collect()));
    #[cfg(feature = "mcp")]
    static MCP_TOOLS: Lazy<Mutex<Vec<&'static McpToolRegistration>>> =
        Lazy::new(|| Mutex::new(inventory::iter::<McpToolRegistration>().collect()));
    #[cfg(feature = "websocket")]
    static WS_ROUTES: Lazy<Mutex<Vec<&'static WebSocketRoute>>> =
        Lazy::new(|| Mutex::new(inventory::iter::<WebSocketRoute>().collect()));
    #[cfg(feature = "grpc")]
    static GRPC_ROUTES: Lazy<Mutex<Vec<&'static GrpcRoute>>> =
        Lazy::new(|| Mutex::new(inventory::iter::<GrpcRoute>().collect()));

    let routes = ROUTES.lock().unwrap();
    #[cfg(feature = "mcp")]
    let mcp_tools = MCP_TOOLS.lock().unwrap();
    #[cfg(feature = "websocket")]
    let ws_routes = WS_ROUTES.lock().unwrap();
    #[cfg(feature = "grpc")]
    let grpc_routes = GRPC_ROUTES.lock().unwrap();

    PluginCounts {
        routes: routes.len(),
        #[cfg(feature = "mcp")]
        mcp_tools: mcp_tools.len(),
        #[cfg(feature = "websocket")]
        ws_routes: ws_routes.len(),
        #[cfg(feature = "grpc")]
        grpc_routes: grpc_routes.len(),
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
