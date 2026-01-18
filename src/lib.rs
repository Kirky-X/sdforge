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
    pub use axum::response::Response;
    pub use tower;

    pub mod routing {
        pub use axum::routing::{
            any, any_service, connect, connect_service, delete, delete_service, get, get_service,
            head, head_service, on, on_service, options, options_service, patch, patch_service,
            post, post_service, put, put_service, trace, trace_service, MethodRouter,
        };
    }

    pub mod extract {
        pub use axum::extract::{Form, Json, Path, Query};
        pub use axum_extra::TypedHeader;
    }

    pub mod http {
        pub use axum::http;
        pub use axum::http::Request;
        pub mod header {
            pub use axum::http::header::CONTENT_TYPE;
        }
        pub mod status {
            pub use axum::http::StatusCode;
        }
    }

    pub mod body {
        pub use axum::body;
        pub use axum::body::Body;
    }

    pub mod handler {
        //! Re-export Handler trait from axum
        pub use axum::handler;
        pub use axum::handler::Handler;
    }

    pub mod response {
        pub use axum::response;
        pub use axum::response::Response;
    }

    pub use axum::serve;
}

/// Commonly used types
pub mod prelude {
    #[cfg(feature = "http")]
    pub use crate::core::validation::validators::{validate_email, validate_length};
    pub use crate::core::{ApiError, ApiMetadata, ServiceError, ServiceResponse};
    #[cfg(feature = "http")]
    pub use crate::http::{HttpRoute, RouteRegistration};
    #[cfg(feature = "mcp")]
    pub use crate::mcp::McpToolInstance;

    #[cfg(feature = "http")]
    pub use axum::response::IntoResponse;

    pub use sdforge_macros::{service_api, service_module, test_macro};
}

pub mod core;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "mcp")]
pub mod mcp;

// Create a module hierarchy for mcp-sdk to match generated code expectations
#[cfg(feature = "mcp")]
pub mod mcp_sdk_types {
    pub mod tools {
        pub use ::mcp_sdk::tools::Tool;
    }

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

#[cfg(feature = "streaming")]
pub mod streaming;

#[cfg(feature = "streaming")]
pub use streaming::{create_stream_channel, stream_to_sse, StreamEvent, StreamResponse};

#[cfg(feature = "security")]
pub mod security;

#[cfg(feature = "security")]
pub use security::{
    auth_middleware, rate_limit_middleware, ApiKeyAuth, AuditLog, AuditLogger, AuditResult,
    AuthContext, AuthError, AuthExtractor, AuthMetadata, AuthResult, BearerAuth, RateLimitConfig,
    RateLimitError, RateLimiter,
};

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

#[cfg(feature = "cache")]
pub mod cache;

#[cfg(feature = "cache")]
pub use cache::{CacheConfig, CacheMiddleware, CacheService};

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "websocket")]
pub use websocket::{
    build, build_with_manager, websocket_upgrade, BoxFuture, ConnectionManager,
    WebSocketConnection, WebSocketHandler, WebSocketMessage, WebSocketRoute,
};

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
