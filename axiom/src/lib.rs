//! Axiom runtime library
//!
//! This crate provides the runtime types and service builders for the Axiom framework.

#![doc(html_root_url = "https://docs.rs/axiom/0.1.0")]
#![warn(missing_docs)]

/// Re-export macros from axiom-macros for convenient use
pub use axiom_macros::{service_api, service_module, test_macro};

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
}

pub mod core;

/// Re-export core types at crate root for convenience
pub use crate::core::{ApiError, ApiMetadata, ServiceError, ServiceResponse};

#[cfg(feature = "http")]
pub use core::validation::validators;

#[cfg(feature = "http")]
pub use inventory;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "http")]
pub use axum;

#[cfg(feature = "mcp")]
pub mod mcp;

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
pub use config::hot_reload::{ConfigEvent, ConfigWatcher};

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
    build_server, build_server_with_config, AxiomGrpcService, GrpcRoute, GrpcServerConfig,
};

#[cfg(feature = "grpc")]
pub use grpc::axiom_v1::{
    axiom_service_server::AxiomServiceServer, CallRequest, CallResponse, InfoRequest, InfoResponse,
};

#[cfg(feature = "http")]
pub use http::version_routing::{build_version_router, VersionRouterConfig, VersionedRoute};

#[cfg(feature = "logging")]
pub use config::{init_logging, init_logging_default};
