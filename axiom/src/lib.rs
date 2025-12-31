//! Axiom runtime library
//!
//! This crate provides the runtime types and service builders for the Axiom framework.

#![doc(html_root_url = "https://docs.rs/axiom/0.1.0")]
#![warn(missing_docs)]

/// Commonly used types
pub mod prelude {
    #[cfg(feature = "http")]
    pub use crate::http::HttpRoute;
    #[cfg(feature = "mcp")]
    pub use crate::mcp::McpToolRegistration;
    pub use crate::core::{ApiError, ApiMetadata, ServiceResponse, ServiceError};
}

mod core;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "mcp")]
mod mcp;

#[cfg(feature = "streaming")]
mod streaming;

#[cfg(feature = "streaming")]
pub use streaming::{StreamResponse, StreamEvent, create_stream_channel, stream_to_sse};

#[cfg(feature = "http")]
mod security;

#[cfg(feature = "http")]
pub use security::{
    ApiKeyAuth, BearerAuth, AuthContext, AuthError, AuthExtractor, AuthMetadata, AuthResult,
    RateLimiter, RateLimitConfig, RateLimitError,
    AuditLogger, AuditLog, AuditResult,
    auth_middleware, rate_limit_middleware,
};

#[cfg(feature = "http")]
mod config;

#[cfg(feature = "http")]
pub use config::{
    AppConfig, ServerConfig, ApiConfig, DatabaseConfig, TlsConfig, CorsConfig,
    RateLimitConfigFile, RateLimitEndpointConfig, AuthConfig, LoggingConfig, TracingConfig,
    ConfigLoader, ConfigError, EnvHelper,
};

#[cfg(feature = "http")]
pub use http::version_routing::{VersionedRoute, VersionRouterConfig, build_version_router};

#[cfg(feature = "logging")]
pub use config::{init_logging, init_logging_default};
